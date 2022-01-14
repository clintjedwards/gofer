package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"os"
	"strconv"

	"github.com/clintjedwards/gofer/sdk"
	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	"github.com/google/go-github/v42/github"
	"github.com/rs/zerolog/log"
)

// parameterEvent is the type of event to watch out for the repository. Events are listed here:
// https://docs.github.com/en/developers/webhooks-and-events/webhooks/webhook-events-and-payloads
const parameterEvent = "event"

// parameterRepository is the repository the pipeline will like to be alerted for.
// The format is <organization>/<repository>
// 	Ex: clintjedwards/gofer
const parameterRepository = "repository"

// parameterSecret is the secret webhook token used to validate this request has come from Github.
const parameterSecret = "secret"

// pipelineSubscription represents info about a particular pipeline subscription. Used to pass the correct event
// back to the appropriate pipeline.
type pipelineSubscription struct {
	event        string
	triggerLabel string
	pipeline     string
	namespace    string
}

// repoSecretKey is composite key of the repository and secret. This is used to match the correct repository/secret
// combination for payload verification.
func repoSecretKey(repo, token string) string {
	return fmt.Sprintf("%s:%s", repo, token)
}

type trigger struct {
	events chan *sdkProto.CheckResponse

	// repoKeys is a mapping of repository to possible webhook secret tokens. This is due to the nature that we
	// don't preload repository credentials from the admin so it is provided by the user. This means that
	// occassionally we will have to try multiple credentials to see which one works. When we find one we combine
	// it with the repository name and store it as the key to figure out which subscriptions this should go to.
	repoKeys map[string]map[string]struct{}

	// subscriptions is a mapping that uses a composite key of repository:secretkey to map onto subscriptions.
	subscriptions map[string][]pipelineSubscription
}

func newTrigger() *trigger {
	return &trigger{
		repoKeys:      map[string]map[string]struct{}{},
		events:        make(chan *sdkProto.CheckResponse, 100),
		subscriptions: map[string][]pipelineSubscription{},
	}
}

// repoInfo is used to parse the repository name from the initial payload.
type repoInfo struct {
	Repository *struct {
		FullName string `json:"full_name"`
	} `json:"repository"`
}

// processNewEvent performs the validation, parsing, and generation of a new Github webhook event.
func (t *trigger) processNewEvent(req *http.Request) error {
	signature := req.Header.Get(github.SHA256SignatureHeader)
	if signature == "" {
		signature = req.Header.Get(github.SHA1SignatureHeader)
	}

	body, err := ioutil.ReadAll(req.Body)
	if err != nil {
		return err
	}

	repoInfo := repoInfo{}
	err = json.Unmarshal(body, &repoInfo)
	if err != nil {
		return err
	}

	if repoInfo.Repository == nil {
		return fmt.Errorf("could not process request; missing repository struct")
	}

	key, payload, err := t.trySignatures(repoInfo.Repository.FullName, signature, body)
	if err != nil {
		return fmt.Errorf("no signatures match: %w", err)
	}

	event := github.WebHookType(req)
	subs := t.findSubscriptionsByEvent(key, event)

	metadata, err := parsePayload(event, payload)
	if err != nil {
		return err
	}

	for _, sub := range subs {
		t.events <- &sdkProto.CheckResponse{
			Details:              fmt.Sprintf("New %q event from %q", event, repoInfo.Repository.FullName),
			PipelineTriggerLabel: sub.triggerLabel,
			NamespaceId:          sub.namespace,
			PipelineId:           sub.pipeline,
			Result:               sdkProto.CheckResponse_SUCCESS,
			Metadata:             metadata,
		}

		log.Debug().Str("namespaceID", sub.namespace).Str("pipelineID", sub.pipeline).
			Str("trigger_label", sub.triggerLabel).Msg("new webhook event generated")
	}

	return nil
}

// parsePayload attempts to turn the payload into its appropriate struct representation. Once in this form we can
// apply filters to also remove subscriptions
func parsePayload(event string, payload []byte) (map[string]string, error) {
	rawEvent, err := github.ParseWebHook(event, payload)
	if err != nil {
		return nil, fmt.Errorf("could not parse webhook payload for event %q", event)
	}

	switch rawEvent := rawEvent.(type) {
	case *github.CreateEvent:
		return map[string]string{
			"GOFER_TRIGGER_GITHUB_REF":        *rawEvent.Ref,
			"GOFER_TRIGGER_GITHUB_REF_TYPE":   *rawEvent.RefType,
			"GOFER_TRIGGER_GITHUB_REPOSITORY": *rawEvent.Repo.FullName,
		}, nil
	case *github.PushEvent:
		return map[string]string{
			"GOFER_TRIGGER_GITHUB_REF":                           *rawEvent.Ref,
			"GOFER_TRIGGER_GITHUB_REPOSITORY":                    *rawEvent.Repo.FullName,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_ID":                *rawEvent.HeadCommit.ID,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_AUTHOR_NAME":       *rawEvent.HeadCommit.Author.Name,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_AUTHOR_EMAIL":      *rawEvent.HeadCommit.Author.Email,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_AUTHOR_USERNAME":   *rawEvent.HeadCommit.Author.Login,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_COMMITER_NAME":     *rawEvent.HeadCommit.Committer.Name,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_COMMITER_EMAIL":    *rawEvent.HeadCommit.Committer.Email,
			"GOFER_TRIGGER_GITHUB_HEAD_COMMIT_COMMITER_USERNAME": *rawEvent.HeadCommit.Committer.Login,
		}, nil
	case *github.ReleaseEvent:
		return map[string]string{
			"GOFER_TRIGGER_GITHUB_ACTION":                   *rawEvent.Action,
			"GOFER_TRIGGER_GITHUB_REPOSITORY":               *rawEvent.Repo.FullName,
			"GOFER_TRIGGER_GITHUB_RELEASE_TAG_NAME":         *rawEvent.Release.TagName,
			"GOFER_TRIGGER_GITHUB_RELEASE_TARGET_COMMITISH": *rawEvent.Release.TargetCommitish,
			"GOFER_TRIGGER_GITHUB_RELEASE_AUTHOR_LOGIN":     *rawEvent.Release.Author.Login,
			"GOFER_TRIGGER_GITHUB_RELEASE_CREATED_AT":       rawEvent.Release.CreatedAt.String(),
			"GOFER_TRIGGER_GITHUB_RELEASE_PUBLISHED_AT":     rawEvent.Release.PublishedAt.String(),
		}, nil
	case *github.CheckSuiteEvent:
		return map[string]string{
			"GOFER_TRIGGER_GITHUB_ACTION":                  *rawEvent.Action,
			"GOFER_TRIGGER_GITHUB_REPOSITORY":              *rawEvent.Repo.FullName,
			"GOFER_TRIGGER_GITHUB_CHECK_SUITE_ID":          strconv.FormatInt(*rawEvent.CheckSuite.ID, 10),
			"GOFER_TRIGGER_GITHUB_CHECK_SUITE_HEAD_SHA":    *rawEvent.CheckSuite.HeadSHA,
			"GOFER_TRIGGER_GITHUB_CHECK_SUITE_HEAD_BRANCH": *rawEvent.CheckSuite.HeadBranch,
		}, nil
	}

	return nil, nil
}

// findSubscriptionsByEvent returns all subscriptions with a certain repo/secret combination that are subscribed to
// a particular event.
func (t *trigger) findSubscriptionsByEvent(repoSecretKey, event string) []pipelineSubscription {
	subscriptions := []pipelineSubscription{}

	potentialSubscriptions, exists := t.subscriptions[repoSecretKey]
	if !exists {
		return nil
	}

	for _, sub := range potentialSubscriptions {
		if sub.event == event {
			subscriptions = append(subscriptions, sub)
		}
	}

	return subscriptions
}

// trySignatures checks the payload to make sure that it is indeed from github by iterating through potentially multiple
// secret keys and checking them against the payload.
// We have to do this because the way github secret webhook tokens are given to the trigger is by a user's pipeline
// configuration. Because of this it is possible that multiple users might enter different secret tokens for the
// same repository. Even though this is incorrect, we still have to accept the key and operate in a fashion that the
// user has a potentially correct key.
func (t *trigger) trySignatures(repository, signature string, body []byte) (subscriptionKey string, payload []byte, err error) {
	keys, exists := t.repoKeys[repository]
	if !exists {
		return "", nil, fmt.Errorf("could not find credentials in repository keystore")
	}

	for key := range keys {
		payload, err = github.ValidatePayloadFromBody("application/json", bytes.NewBuffer(body), signature, []byte(key))
		if err != nil {
			continue
		}

		return repoSecretKey(repository, key), payload, nil
	}

	return "", nil, fmt.Errorf("no keys match repository/signature combination given")
}

func (t *trigger) Subscribe(ctx context.Context, request *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error) {
	event, exists := request.Config[parameterEvent]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterEvent)
	}

	repo, exists := request.Config[parameterRepository]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterRepository)
	}

	secret, exists := request.Config[parameterSecret]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterSecret)
	}

	_, exists = t.repoKeys[repo]
	if !exists {
		t.repoKeys[repo] = map[string]struct{}{}
	}

	t.repoKeys[repo][secret] = struct{}{}

	_, exists = t.subscriptions[repoSecretKey(repo, secret)]
	if !exists {
		t.subscriptions[repoSecretKey(repo, secret)] = []pipelineSubscription{}
	}

	t.subscriptions[repoSecretKey(repo, secret)] = append(t.subscriptions[repoSecretKey(repo, secret)], pipelineSubscription{
		event:        event,
		triggerLabel: request.PipelineTriggerLabel,
		pipeline:     request.PipelineId,
		namespace:    request.NamespaceId,
	})

	log.Debug().Str("trigger_label", request.PipelineTriggerLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &sdkProto.SubscribeResponse{}, nil
}

func (t *trigger) Unsubscribe(ctx context.Context, request *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error) {
	for key, subscriptions := range t.subscriptions {
		for index, subscription := range subscriptions {
			if subscription.triggerLabel == request.PipelineTriggerLabel &&
				subscription.namespace == request.NamespaceId &&
				subscription.pipeline == request.PipelineId {
				t.subscriptions[key] = append(subscriptions[:index], subscriptions[index+1:]...)
				return &sdkProto.UnsubscribeResponse{}, nil
			}
		}
	}

	log.Debug().Str("trigger_label", request.PipelineTriggerLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return &sdkProto.UnsubscribeResponse{}, nil
}

func (t *trigger) Check(ctx context.Context, request *sdkProto.CheckRequest) (*sdkProto.CheckResponse, error) {
	select {
	case <-ctx.Done():
		return &sdkProto.CheckResponse{}, nil
	case event := <-t.events:
		return event, nil
	}
}

func (t *trigger) Info(ctx context.Context, request *sdkProto.InfoRequest) (*sdkProto.InfoResponse, error) {
	return &sdkProto.InfoResponse{
		Kind:          os.Getenv("GOFER_TRIGGER_KIND"),
		Documentation: "https://clintjedwards.com/gofer/docs/triggers/github/overview",
	}, nil
}

func (t *trigger) Shutdown(ctx context.Context, request *sdkProto.ShutdownRequest) (*sdkProto.ShutdownResponse, error) {
	close(t.events)
	return &sdkProto.ShutdownResponse{}, nil
}

func (t *trigger) ExternalEvent(ctx context.Context, request *sdkProto.ExternalEventRequest) (*sdkProto.ExternalEventResponse, error) {
	payload := bytes.NewBuffer(request.Payload)
	payloadReader := bufio.NewReader(payload)
	req, err := http.ReadRequest(payloadReader)
	if err != nil {
		return nil, err
	}

	err = t.processNewEvent(req)
	if err != nil {
		return nil, err
	}
	return &sdkProto.ExternalEventResponse{}, nil
}

func main() {
	newTrigger := newTrigger()

	sdk.NewTriggerServer(newTrigger)
}
