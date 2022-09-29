package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/base64"
	"fmt"
	"net/http"
	"os"
	"strconv"
	"strings"

	"github.com/bradleyfalzon/ghinstallation/v2"
	"github.com/clintjedwards/gofer/sdk"
	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	"github.com/google/go-github/v42/github"
	"github.com/rs/zerolog/log"
)

// Trigger configuration env vars
// These are general settings needed for github apps:
// https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id
const (
	envvarID            = "GOFER_TRIGGER_GITHUB_APPS_ID"
	envvarInstallation  = "GOFER_TRIGGER_GITHUB_APPS_INSTALLATION"
	envvarKey           = "GOFER_TRIGGER_GITHUB_APPS_KEY"
	envvarWebhookSecret = "GOFER_TRIGGER_GITHUB_APPS_WEBHOOK_SECRET"
)

// Pipeline configuration parameters
const (
	// parameterEvents is a comma separated list of events to listen for. Events are listed here:
	// https://docs.github.com/en/developers/webhooks-and-events/webhooks/webhook-events-and-payloads
	parameterEvents = "events"

	// parameterRepository is the repository the pipeline will like to be alerted for.
	// The format is <organization>/<repository>
	// 	Ex: clintjedwards/gofer
	parameterRepository = "repository"
)

// pipelineSubscription represents info about a particular pipeline subscription. Used to pass the correct event
// back to the appropriate pipeline.
type pipelineSubscription struct {
	event        string
	repository   string
	triggerLabel string
	pipeline     string
	namespace    string
}

type trigger struct {
	webhookSecret string // Github app webhook secret

	client *github.Client

	// events channel are omitted when a pipeline should be run. The main Gofer process watches this channel.
	events chan *sdkProto.CheckResponse

	// subscriptions is a mapping of event type to pipeline subscription. A single subscription could possibly
	// be listening for multiple
	subscriptions map[string]map[string][]pipelineSubscription
}

func newTrigger() (*trigger, error) {
	rawApp := os.Getenv(envvarID)
	if rawApp == "" {
		return nil, fmt.Errorf("could not find required environment variable %q", envvarID)
	}
	app, err := strconv.Atoi(rawApp)
	if err != nil {
		return nil, fmt.Errorf("malformed github app id %q", rawApp)
	}

	rawInstallation := os.Getenv(envvarInstallation)
	if rawInstallation == "" {
		return nil, fmt.Errorf("could not find required environment variable %q", envvarInstallation)
	}
	installation, err := strconv.Atoi(rawInstallation)
	if err != nil {
		return nil, fmt.Errorf("malformed github installation id %q", rawApp)
	}

	rawKey := os.Getenv(envvarKey)
	if rawKey == "" {
		return nil, fmt.Errorf("could not find required environment variable %q", envvarKey)
	}

	key, err := base64.StdEncoding.DecodeString(rawKey)
	if err != nil {
		return nil, fmt.Errorf("could not decode base64 private key; %v", err)
	}

	webhookSecret := os.Getenv(envvarWebhookSecret)
	if webhookSecret == "" {
		return nil, fmt.Errorf("could not find required environment variable %q", envvarWebhookSecret)
	}

	client, err := newGithubClient(int64(app), int64(installation), key)
	if err != nil {
		return nil, fmt.Errorf("could not init Github client %v", err)
	}

	return &trigger{
		client:        client,
		webhookSecret: webhookSecret,
		events:        make(chan *sdkProto.CheckResponse, 100),
		subscriptions: map[string]map[string][]pipelineSubscription{},
	}, nil
}

func newGithubClient(app, installation int64, key []byte) (*github.Client, error) {
	transport, err := ghinstallation.New(http.DefaultTransport, app, installation, key)
	if err != nil {
		return nil, err
	}

	client := github.NewClient(&http.Client{Transport: transport})
	client.UserAgent = "clintjedwards/gofer"
	return client, nil
}

func (t *trigger) processNewEvent(req *http.Request) error {
	log.Debug().Msg("processing new webhook event")
	rawPayload, err := github.ValidatePayload(req, []byte(t.webhookSecret))
	if err != nil {
		return err
	}

	parsedPayload, err := github.ParseWebHook(github.WebHookType(req), rawPayload)
	if err != nil {
		return err
	}

	handler, exists := handlers[github.WebHookType(req)]
	if !exists {
		// We don't return this as an error, because it is not an error on the Github side.
		// Instead we just log that we've received it and we move along.
		log.Debug().Msgf("event type %q not supported", github.WebHookType(req))
		return nil
	}

	repo, metadata, err := handler(parsedPayload)
	if err != nil {
		return err
	}

	for _, sub := range t.matchSubscriptions(github.WebHookType(req), repo) {
		t.events <- &sdkProto.CheckResponse{
			Details:              fmt.Sprintf("New %q event from %q", github.WebHookType(req), repo),
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

// matchSubscriptions returns all subscriptions with the proper event/repo combination.
func (t *trigger) matchSubscriptions(event, repo string) []pipelineSubscription {
	repoMap, exists := t.subscriptions[event]
	if !exists {
		return []pipelineSubscription{}
	}

	subscriptions, exists := repoMap[repo]
	if !exists {
		return []pipelineSubscription{}
	}

	return subscriptions
}

func (t *trigger) Subscribe(ctx context.Context, request *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error) {
	eventStr, exists := request.Config[strings.ToUpper(parameterEvents)]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterEvents)
	}

	eventList := strings.Split(eventStr, ",")

	repo, exists := request.Config[strings.ToUpper(parameterRepository)]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterRepository)
	}

	for _, event := range eventList {
		event = strings.TrimSpace(event)
		_, exists = t.subscriptions[event]
		if !exists {
			t.subscriptions[event] = map[string][]pipelineSubscription{
				repo: {},
			}
		}

		t.subscriptions[event][repo] = append(t.subscriptions[event][repo], pipelineSubscription{
			event:        event,
			repository:   repo,
			triggerLabel: request.PipelineTriggerLabel,
			pipeline:     request.PipelineId,
			namespace:    request.NamespaceId,
		})
	}

	log.Debug().Str("trigger_label", request.PipelineTriggerLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &sdkProto.SubscribeResponse{}, nil
}

func (t *trigger) Unsubscribe(ctx context.Context, request *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error) {
	for event, repoMap := range t.subscriptions {
		for repo, subscriptions := range repoMap {
			for index, sub := range subscriptions {
				if sub.triggerLabel == request.PipelineTriggerLabel &&
					sub.namespace == request.NamespaceId &&
					sub.pipeline == request.PipelineId {
					t.subscriptions[event][repo] = append(subscriptions[:index], subscriptions[index+1:]...)
					return &sdkProto.UnsubscribeResponse{}, nil
				}
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
	newTrigger, err := newTrigger()
	if err != nil {
		panic(err)
	}

	sdk.NewTriggerServer(newTrigger)
}
