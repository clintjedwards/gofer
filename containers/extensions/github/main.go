package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"

	"github.com/bradleyfalzon/ghinstallation/v2"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	extsdk "github.com/clintjedwards/gofer/sdk/go/extensions"
	"github.com/google/go-github/v58/github"
	"github.com/rs/zerolog/log"
)

// Extension configuration.
// These are general settings needed for github apps:
// https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id
const (
	configAppID            = "app_id"
	configAppInstallation  = "app_installation"
	configAppKey           = "app_key"
	configAppWebhookSecret = "app_webhook_secret"
)

// Pipeline configuration parameters.
const (
	// parameterEventFilter is the event/action combination the pipeline will be triggered upon. It's presented in the form: <event>/<action>,<action2>...
	// For events that do not have actions or if you simply want to trigger on any action, just putting the <event> will suffice.
	// To be clear if you don't include actions on an event that has multiple, Gofer will be triggered on any action.
	// You can find a list of events and their actions here(Actions listed as 'activity type'): https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows.
	parameterEventFilter = "event_filter"

	// parameterRepository is the repository the pipeline will be alerted for.
	// The format is <organization>/<repository>
	// 	Ex: clintjedwards/gofer
	parameterRepository = "repository"
)

var eventSet = map[string]struct{}{
	"branch_protection_rule":      {},
	"check_run":                   {},
	"check_suite":                 {},
	"create":                      {},
	"delete":                      {},
	"deployment":                  {},
	"deployment_status":           {},
	"discussion":                  {},
	"discussion_comment":          {},
	"fork":                        {},
	"gollum":                      {},
	"issue_comment":               {},
	"issues":                      {},
	"label":                       {},
	"merge_group":                 {},
	"milestone":                   {},
	"page_build":                  {},
	"project":                     {},
	"project_card":                {},
	"project_column":              {},
	"public":                      {},
	"pull_request":                {},
	"pull_request_comment":        {},
	"pull_request_review":         {},
	"pull_request_review_comment": {},
	"pull_request_target":         {},
	"push":                        {},
	"registry_package":            {},
	"release":                     {},
	"repository_dispatch":         {},
	"schedule":                    {},
	"status":                      {},
	"watch":                       {},
	"workflow_call":               {},
	"workflow_dispatch":           {},
	"workflow_run":                {},
}

// pipelineSubscription represents info about a particular pipeline subscription. Used to pass the correct event
// back to the appropriate pipeline.
type pipelineSubscription struct {
	// The event filter string that the user registered the pipeline with.
	eventFilter    string
	event          string
	actions        map[string]struct{} // If actions is empty, will trigger at any given action.
	repository     string
	extensionLabel string
	pipeline       string
	namespace      string
}

type extension struct {
	webhookSecret string // Github app webhook secret

	client *github.Client

	// A mapping of event type to pipeline subscription. A single subscription could possibly
	// be listening for multiple repositories.
	// Map layout is map[event_type]map[repository][]subscriptions
	subscriptions map[string]map[string][]pipelineSubscription

	config extsdk.ExtensionSystemConfig
}

func newExtension() extension {
	appIDStr := extsdk.GetConfigFromEnv(configAppID)
	app, err := strconv.Atoi(appIDStr)
	if err != nil {
		log.Fatal().Err(err).Str(configAppID, appIDStr).Msg("malformed github app id")
	}

	installationStr := extsdk.GetConfigFromEnv(configAppInstallation)
	installation, err := strconv.Atoi(installationStr)
	if err != nil {
		log.Fatal().Err(err).Str(configAppInstallation, installationStr).Msg("malformed github installation id")
	}

	keyStr := extsdk.GetConfigFromEnv(configAppKey)
	key, err := base64.StdEncoding.DecodeString(keyStr)
	if err != nil {
		log.Fatal().Err(err).Str(configAppKey, keyStr).Msg("could not decode base64 private key")
	}

	webhookSecret := extsdk.GetConfigFromEnv(configAppWebhookSecret)
	if webhookSecret == "" {
		log.Fatal().Err(err).Str("env_var", configAppWebhookSecret).Msg("could not find required env var")
	}

	client, err := newGithubClient(int64(app), int64(installation), key)
	if err != nil {
		log.Fatal().Err(err).Msg("could not init Github client")
	}

	config, err := extsdk.GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not parse system configuration")
	}

	extension := extension{
		webhookSecret: webhookSecret,
		client:        client,
		config:        config,
		subscriptions: map[string]map[string][]pipelineSubscription{},
	}

	subscriptions, err := sdk.ListExtensionSubscriptions(config.ID, config.GoferHost, config.Secret, config.UseTLS, sdk.GoferAPIVersion0)
	if err != nil {
		log.Fatal().Err(err).Msg("Could not query subscriptions from Gofer host")
	}
	// TODO: Eventually we should make this more intelligent to prevent thundering herd problems. But for right now
	// this should suffice.
	for _, subscription := range subscriptions {
		// We just call the internal subscribe function here since it does all the validation we'd have to redo either
		// way.
		err := extension.Subscribe(context.Background(), extsdk.SubscriptionRequest{
			NamespaceId:                subscription.NamespaceId,
			PipelineId:                 subscription.PipelineId,
			PipelineSubscriptionId:     subscription.SubscriptionId,
			PipelineSubscriptionParams: subscription.Settings,
		})
		if err != nil {
			log.Fatal().Str("err", err.Message).Msg("Could not restore subscription")
		}
	}

	return extension
}

// The event filter contains potentially two parts an event and an action. We'll need to parse the filter to
// figure out these bespoke parts for later use when narrowing down which to call.
func parseEventFilter(filterStr string) (event string, actions []string) {
	event, actionStr, found := strings.Cut(filterStr, "/")

	actions = []string{}

	if !found {
		return event, actions
	}

	actionList := strings.Split(actionStr, ",")
	for _, action := range actionList {
		action := action
		if action == "" {
			continue
		}

		action = strings.ToLower(action)
		actions = append(actions, action)
	}

	return event, actions
}

func (e *extension) Health(_ context.Context) *extsdk.HttpError {
	return nil
}

func newGithubClient(app, installation int64, key []byte) (*github.Client, error) {
	transport, err := ghinstallation.New(http.DefaultTransport, app, installation, key)
	if err != nil {
		return nil, err
	}

	client := github.NewClient(&http.Client{Transport: transport})
	return client, nil
}

func (e *extension) processNewEvent(req *http.Request) error {
	log.Debug().Msg("processing new webhook event")
	rawPayload, err := github.ValidatePayload(req, []byte(e.webhookSecret))
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
		log.Debug().Str("event", github.WebHookType(req)).Msg("event type not supported")
		return nil
	}

	repo, action, metadata, err := handler(parsedPayload)
	if err != nil {
		return err
	}

	for _, sub := range e.matchSubscriptions(github.WebHookType(req), repo, action) {
		log := log.With().Str("namespace_id", sub.namespace).Str("pipeline_id", sub.pipeline).
			Str("pipeline_subscription_id", sub.extensionLabel).Logger()

		client, err := sdk.NewClientWithHeaders(e.config.GoferHost, e.config.Secret, e.config.UseTLS, sdk.GoferAPIVersion0)
		if err != nil {
			log.Error().Err(err).Msg("Could not establish Gofer client")
			continue
		}

		resp, err := client.StartRun(context.Background(), sub.namespace, sub.pipeline, sdk.StartRunRequest{
			Variables: metadata,
		})
		if err != nil {
			log.Error().Err(err).Msg("could not start new run")
			continue
		}
		defer resp.Body.Close()

		if resp.StatusCode < 200 || resp.StatusCode > 299 {
			log.Error().Int("status_code", resp.StatusCode).Msg("could not start new run; received non 2xx status code")
			continue
		}

		body, err := io.ReadAll(resp.Body)
		if err != nil {
			log.Error().Err(err).Msg("could not read response body while attempting to start run")
			continue
		}

		startRunResponse := sdk.StartRunResponse{}
		if err := json.Unmarshal(body, &startRunResponse); err != nil {
			log.Error().Err(err).Msg("could not parse response body while attempting to read start run response")
			continue
		}

		log.Debug().Int64("run_id", int64(startRunResponse.Run.RunId)).Msg("started new run for github webhook")
	}

	return nil
}

// matchSubscriptions returns all subscriptions with the proper event/repo/action combination.
// Action could be an empty string; if so just the event will be matched.
func (e *extension) matchSubscriptions(event, repo, action string) []pipelineSubscription {
	repoMap, exists := e.subscriptions[event]
	if !exists {
		return []pipelineSubscription{}
	}

	subscriptions, exists := repoMap[repo]
	if !exists {
		return []pipelineSubscription{}
	}

	if action == "" {
		return subscriptions
	}

	matchedSubscriptions := []pipelineSubscription{}

	for _, subscription := range subscriptions {
		// If the special 'any' action exists that means we match for any action given
		_, exists := subscription.actions["any"]
		if exists {
			matchedSubscriptions = append(matchedSubscriptions, subscription)
			continue
		}

		// Else we just simply check for the normal action
		action = strings.ToLower(action)
		_, exists = subscription.actions[action]
		if exists {
			matchedSubscriptions = append(matchedSubscriptions, subscription)
		}
	}

	return matchedSubscriptions
}

func (e *extension) Subscribe(_ context.Context, request extsdk.SubscriptionRequest) *extsdk.HttpError {
	eventStr, exists := request.PipelineSubscriptionParams[strings.ToUpper(parameterEventFilter)]
	if !exists {
		return &extsdk.HttpError{
			Message: fmt.Sprintf("could not find required pipeline subscription parameter %q; received params: %+v",
				parameterEventFilter, request.PipelineSubscriptionParams),
			StatusCode: http.StatusBadRequest,
		}
	}

	event, actions := parseEventFilter(eventStr)

	_, exists = eventSet[event]
	if !exists {
		return &extsdk.HttpError{
			Message:    fmt.Sprintf("event %q unknown; event names can be gleaned from github documentation:  https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows", parameterEventFilter),
			StatusCode: http.StatusBadRequest,
		}
	}

	repo, exists := request.PipelineSubscriptionParams[strings.ToUpper(parameterRepository)]
	if !exists {
		return &extsdk.HttpError{
			Message:    fmt.Sprintf("could not find required configuration parameter %q", parameterRepository),
			StatusCode: http.StatusBadRequest,
		}
	}

	event = strings.TrimSpace(event)
	_, exists = e.subscriptions[event]
	if !exists {
		e.subscriptions[event] = map[string][]pipelineSubscription{
			repo: {},
		}
	}

	actionMap := map[string]struct{}{}

	for _, action := range actions {
		normalizedAction := strings.ToLower(action)
		actionMap[normalizedAction] = struct{}{}
	}

	// If the action map is empty we want to instead use a special indicator that we should match any action for the event.
	if len(actionMap) == 0 {
		actionMap["any"] = struct{}{}
	}

	e.subscriptions[event][repo] = append(e.subscriptions[event][repo], pipelineSubscription{
		eventFilter:    eventStr,
		event:          event,
		actions:        actionMap,
		repository:     repo,
		extensionLabel: request.PipelineSubscriptionId,
		pipeline:       request.PipelineId,
		namespace:      request.NamespaceId,
	})

	log.Debug().Str("extension_label", request.PipelineSubscriptionId).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return nil
}

func (e *extension) Unsubscribe(_ context.Context, request extsdk.UnsubscriptionRequest) *extsdk.HttpError {
	for event, repoMap := range e.subscriptions {
		for repo, subscriptions := range repoMap {
			for index, sub := range subscriptions {
				if sub.extensionLabel == request.PipelineSubscriptionId &&
					sub.namespace == request.NamespaceId &&
					sub.pipeline == request.PipelineId {
					e.subscriptions[event][repo] = append(subscriptions[:index], subscriptions[index+1:]...)
					return nil
				}
			}
		}
	}

	log.Debug().Str("extension_label", request.PipelineSubscriptionId).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return nil
}

func (e *extension) Info(_ context.Context) (*extsdk.InfoResponse, *extsdk.HttpError) {
	return &extsdk.InfoResponse{
		ExtensionId: "", // The extension wrapper automagically fills this in.
		Documentation: extsdk.Documentation{
			Body: "You can find more information on this extension at the official Gofer docs site: https://clintjedwards.com/gofer/ref/extensions/provided/github.html",
			PipelineSubscriptionParams: []extsdk.Parameter{
				{
					Key: parameterEventFilter,
					Documentation: "The event/action combination the pipeline will be triggered upon. It's presented in" +
						" the form: <event>/<action>,<action2>... For events that do not have actions or if you simply want to trigger on any" +
						" action, just putting the <event> wil suffice. To be clear if you don't include actions on an event that has multiple," +
						" Gofer will be triggered on any action. you can find a list of events and their actions here(Actions listed as 'activity type'):" +
						" https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows.",
					Required: true,
				},
				{
					Key: parameterRepository,
					Documentation: "The repository the pipeline will be alerted for. The format is <organization>/<repository>" +
						" Ex: clintjedwards/gofer",
					Required: true,
				},
			},
			ConfigParams: []extsdk.Parameter{
				{
					Key:           configAppID,
					Documentation: "General settings for all Github apps: https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id",
					Required:      true,
				},
				{
					Key:           configAppInstallation,
					Documentation: "General settings for all Github apps: https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id",
					Required:      true,
				},
				{
					Key:           configAppKey,
					Documentation: "General settings for all Github apps: https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id",
					Required:      true,
				},
				{
					Key:           configAppWebhookSecret,
					Documentation: "General settings for all Github apps: https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id",
					Required:      true,
				},
			},
		},
	}, nil
}

func (e *extension) Debug(_ context.Context) extsdk.DebugResponse {
	registeredMap := map[string]struct{}{}
	for _, repoMap := range e.subscriptions {
		for _, subList := range repoMap {
			for _, sub := range subList {
				registeredMap[fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline)] = struct{}{}
			}
		}
	}

	registeredList := []string{}
	for key := range registeredMap {
		registeredList = append(registeredList, key)
	}

	config, _ := extsdk.GetExtensionSystemConfig()

	debug := struct {
		RegisteredPipelines []string `json:"registered_pipelines"`
		Config              extsdk.ExtensionSystemConfig
	}{
		RegisteredPipelines: registeredList,
		Config:              config,
	}

	data, jsonErr := json.Marshal(debug)
	if jsonErr != nil {
		log.Error().Err(jsonErr).Msg("Could not serialize response for debug endpoint")
	}

	return extsdk.DebugResponse{
		Info: string(data),
	}
}

func (e *extension) Shutdown(_ context.Context) {}

func (e *extension) ExternalEvent(_ context.Context, request extsdk.ExternalEventRequest) *extsdk.HttpError {
	payload := bytes.NewBuffer(request.Payload)
	payloadReader := bufio.NewReader(payload)
	req, err := http.ReadRequest(payloadReader)
	if err != nil {
		return &extsdk.HttpError{
			StatusCode: http.StatusBadRequest,
			Message:    fmt.Sprintf("Could not parse external request body: %v", err),
		}
	}

	err = e.processNewEvent(req)
	if err != nil {
		return &extsdk.HttpError{
			StatusCode: http.StatusBadRequest,
			Message:    fmt.Sprintf("Could not process external request: %v", err),
		}
	}
	return nil
}

func main() {
	extension := newExtension()
	extsdk.NewExtension(&extension)
}
