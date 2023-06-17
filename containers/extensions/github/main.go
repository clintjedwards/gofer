package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/base64"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/bradleyfalzon/ghinstallation/v2"
	proto "github.com/clintjedwards/gofer/proto/go"
	sdk "github.com/clintjedwards/gofer/sdk/go/extensions"
	"github.com/google/go-github/v42/github"
	"github.com/rs/zerolog/log"
)

// Extension configuration.
// These are general settings needed for github apps:
// https://docs.github.com/en/developers/apps/getting-started-with-apps/setting-up-your-development-environment-to-create-a-github-app#step-3-save-your-private-key-and-app-id
const (
	configAppID           = "app_id"
	configAppInstallation = "app_installation"
	configAppKey          = "app_key"
	configAppWebhookKey   = "app_webhook_secret"
)

// Pipeline configuration parameters.
const (
	// parameterEventFilter is the event/action combination the pipeline will be triggered upon. It's presented in the form: <event>/[<action>,<action2>...]
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

func (t *extension) Init(_ context.Context, request *proto.ExtensionInitRequest) (*proto.ExtensionInitResponse, error) {
	appIDStr := request.Config[configAppID]
	app, err := strconv.Atoi(appIDStr)
	if err != nil {
		return nil, fmt.Errorf("malformed github app id %q", appIDStr)
	}

	installationStr := request.Config[configAppInstallation]
	installation, err := strconv.Atoi(installationStr)
	if err != nil {
		return nil, fmt.Errorf("malformed github installation id %q", installationStr)
	}

	keyStr := request.Config[configAppKey]
	key, err := base64.StdEncoding.DecodeString(keyStr)
	if err != nil {
		return nil, fmt.Errorf("could not decode base64 private key; %v", err)
	}

	t.webhookSecret = request.Config[configAppWebhookKey]
	if t.webhookSecret == "" {
		return nil, fmt.Errorf("could not find required environment variable %q", configAppWebhookKey)
	}

	t.client, err = newGithubClient(int64(app), int64(installation), key)
	if err != nil {
		return nil, fmt.Errorf("could not init Github client %v", err)
	}

	t.subscriptions = map[string]map[string][]pipelineSubscription{}

	return &proto.ExtensionInitResponse{}, nil
}

func newGithubClient(app, installation int64, key []byte) (*github.Client, error) {
	transport, err := ghinstallation.New(http.DefaultTransport, app, installation, key)
	if err != nil {
		return nil, err
	}

	client := github.NewClient(&http.Client{Transport: transport})
	return client, nil
}

func (t *extension) processNewEvent(req *http.Request) error {
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
		log.Debug().Str("event", github.WebHookType(req)).Msg("event type not supported")
		return nil
	}

	repo, action, metadata, err := handler(parsedPayload)
	if err != nil {
		return err
	}

	for _, sub := range t.matchSubscriptions(github.WebHookType(req), repo, action) {
		client, ctx, err := sdk.Connect()
		if err != nil {
			log.Error().Err(err).Str("namespace_id", sub.namespace).Str("pipeline_id", sub.pipeline).
				Str("extension_label", sub.extensionLabel).Msg("could not connect to Gofer")

			continue
		}

		config, _ := sdk.GetExtensionSystemConfig()

		resp, err := client.StartRun(ctx, &proto.StartRunRequest{
			NamespaceId: sub.namespace,
			PipelineId:  sub.pipeline,
			Variables:   metadata,
			Initiator: &proto.Initiator{
				Type:   proto.Initiator_EXTENSION,
				Name:   fmt.Sprintf("%s (%s)", config.Name, sub.extensionLabel),
				Reason: fmt.Sprintf("New %q event from %q", github.WebHookType(req), repo),
			},
		})
		if err != nil {
			log.Error().Err(err).Str("namespace_id", sub.namespace).Str("pipeline_id", sub.pipeline).
				Str("extension_label", sub.extensionLabel).Msg("could not start new run")

			continue
		}

		log.Debug().Str("namespace_id", sub.namespace).Str("pipeline_id", sub.pipeline).
			Str("extension_label", sub.extensionLabel).Int64("run_id", resp.Run.Id).
			Msg("started new run for github webhook")
	}

	return nil
}

// matchSubscriptions returns all subscriptions with the proper event/repo/action combination.
// Action could be an empty string; if so just the event will be matched.
func (t *extension) matchSubscriptions(event, repo, action string) []pipelineSubscription {
	repoMap, exists := t.subscriptions[event]
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

func (t *extension) Subscribe(_ context.Context, request *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error) {
	eventStr, exists := request.Config[parameterEventFilter]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterEventFilter)
	}

	event, actions := parseEventFilter(eventStr)

	_, exists = eventSet[event]
	if !exists {
		return nil, fmt.Errorf("event %q unknown; event names can be gleaned from github documentation", parameterEventFilter)
	}

	repo, exists := request.Config[parameterRepository]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterRepository)
	}

	event = strings.TrimSpace(event)
	_, exists = t.subscriptions[event]
	if !exists {
		t.subscriptions[event] = map[string][]pipelineSubscription{
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

	t.subscriptions[event][repo] = append(t.subscriptions[event][repo], pipelineSubscription{
		eventFilter:    eventStr,
		event:          event,
		actions:        actionMap,
		repository:     repo,
		extensionLabel: request.PipelineExtensionLabel,
		pipeline:       request.PipelineId,
		namespace:      request.NamespaceId,
	})

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &proto.ExtensionSubscribeResponse{}, nil
}

func (t *extension) Unsubscribe(_ context.Context, request *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error) {
	for event, repoMap := range t.subscriptions {
		for repo, subscriptions := range repoMap {
			for index, sub := range subscriptions {
				if sub.extensionLabel == request.PipelineExtensionLabel &&
					sub.namespace == request.NamespaceId &&
					sub.pipeline == request.PipelineId {
					t.subscriptions[event][repo] = append(subscriptions[:index], subscriptions[index+1:]...)
					return &proto.ExtensionUnsubscribeResponse{}, nil
				}
			}
		}
	}

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return &proto.ExtensionUnsubscribeResponse{}, nil
}

func (t *extension) Info(_ context.Context, _ *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error) {
	registeredMap := map[string]struct{}{}
	for _, repoMap := range t.subscriptions {
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

	config, _ := sdk.GetExtensionSystemConfig()

	return &proto.ExtensionInfoResponse{
		Name:          config.Name,
		Documentation: "https://clintjedwards.com/gofer/ref/extensions/provided/github.html",
		Registered:    registeredList,
	}, nil
}

func (t *extension) Shutdown(_ context.Context, _ *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error) {
	return &proto.ExtensionShutdownResponse{}, nil
}

func (t *extension) ExternalEvent(_ context.Context, request *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error) {
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
	return &proto.ExtensionExternalEventResponse{}, nil
}

func (t *extension) RunExtensionInstaller(stream proto.ExtensionService_RunExtensionInstallerServer) error {
	err := sdk.SendInstallerMessageToClient(stream, "The Github extension allows Gofer pipelines to be run on Github webhook events.\n")
	if err != nil {
		return err
	}

	err = sdk.SendInstallerMessageToClient(stream, `It requires the setup and use of a new Github app. The best walkthrough available is
available on the Gofer documentation site here: https://clintjedwards.com/gofer/ref/extensions/provided/github.html.

Follow the instructions and enter the values requested below.`)
	if err != nil {
		return err
	}

	err = sdk.SendInstallerQueryToClient(stream, "First, enter the App ID located on the configuration page for the new Github App")
	if err != nil {
		return err
	}

	appID, err := stream.Recv()
	if err != nil {
		return err
	}

	err = sdk.SendInstallerConfigSettingToClient(stream, configAppID, appID.Msg)
	if err != nil {
		return err
	}

	err = sdk.SendInstallerQueryToClient(stream, "Next, enter the webhook secret you created")
	if err != nil {
		return err
	}

	webhookSecret, err := stream.Recv()
	if err != nil {
		return err
	}

	err = sdk.SendInstallerConfigSettingToClient(stream, configAppWebhookKey, webhookSecret.Msg)
	if err != nil {
		return err
	}

	err = sdk.SendInstallerQueryToClient(stream, "Next, enter the base64'd private key you created")
	if err != nil {
		return err
	}

	privateKey, err := stream.Recv()
	if err != nil {
		return err
	}

	err = sdk.SendInstallerConfigSettingToClient(stream, configAppWebhookKey, privateKey.Msg)
	if err != nil {
		return err
	}

	err = sdk.SendInstallerQueryToClient(stream, "Lastly, enter the installation ID of your new Github App")
	if err != nil {
		return err
	}

	installID, err := stream.Recv()
	if err != nil {
		return err
	}

	err = sdk.SendInstallerConfigSettingToClient(stream, configAppInstallation, installID.Msg)
	if err != nil {
		return err
	}

	err = sdk.SendInstallerMessageToClient(stream, "Setup finished!")
	if err != nil {
		return err
	}

	return nil
}

func (t *extension) RunPipelineConfigurator(stream proto.ExtensionService_RunPipelineConfiguratorServer) error {
	err := sdk.SendConfiguratorMessageToClient(stream, "The Github extension allows Gofer pipelines to be run on Github webhook events.\n")
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorQueryToClient(stream, "Firstly, let's set which repository you're targeting, repositories are "+
		"specified in the format <organization>/<name> . For example: clintjedwards/gofer")
	if err != nil {
		return err
	}

	clientMsg, err := stream.Recv()
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorParamSettingToClient(stream, parameterRepository, clientMsg.Msg)
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorMessageToClient(stream,
		"Next, for the selected repository, we'll trigger your pipeline based on specific events and actions within those events. "+
			"You can find a comprehensive list of events, their associated actions, and details on the environment variables they return "+
			"at this link: https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows.\n\n"+
			"To target specific events and actions, use the following format: <event>/[<action1>,<action2>...]. "+
			"For example: `pull_request/opened`.\n\n"+
			"If you'd like to trigger the pipeline for all available actions within a particular event, you can use the keyword 'any'. "+
			"For example: `pull_request/any`.",
	)
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorQueryToClient(stream, "Specify which event/action combination to listen for: ")
	if err != nil {
		return err
	}

	clientMsg, err = stream.Recv()
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorParamSettingToClient(stream, parameterEventFilter, clientMsg.Msg)
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorMessageToClient(stream, "Github extension configuration finished")
	if err != nil {
		return err
	}

	return nil
}

func main() {
	extension := extension{}
	sdk.NewExtension(&extension)
}
