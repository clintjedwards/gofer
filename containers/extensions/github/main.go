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
	configID           = "app_id"
	configInstallation = "app_installation"
	configKey          = "app_key"
	configWebhookKey   = "app_webhook_secret"
)

// Pipeline configuration parameters.
const (
	// parameterEvents is a comma separated list of events to listen for. Events are listed here:
	// https://docs.github.com/en/developers/webhooks-and-events/webhooks/webhook-events-and-payloads
	parameterEvents = "events"

	// parameterRepository is the repository the pipeline will be alerted for.
	// The format is <organization>/<repository>
	// 	Ex: clintjedwards/gofer
	parameterRepository = "repository"
)

// pipelineSubscription represents info about a particular pipeline subscription. Used to pass the correct event
// back to the appropriate pipeline.
type pipelineSubscription struct {
	event          string
	repository     string
	extensionLabel string
	pipeline       string
	namespace      string
}

type extension struct {
	webhookSecret string // Github app webhook secret

	client *github.Client

	// A 3mapping of event type to pipeline subscription. A single subscription could possibly
	// be listening for multiple repositories.
	// Map layout is map[event_type]map[repository][]subscriptions
	subscriptions map[string]map[string][]pipelineSubscription
}

func newExtension() (*extension, error) {
	appIDStr := sdk.GetConfig(configID)
	app, err := strconv.Atoi(appIDStr)
	if err != nil {
		return nil, fmt.Errorf("malformed github app id %q", appIDStr)
	}

	installationStr := sdk.GetConfig(configInstallation)
	installation, err := strconv.Atoi(installationStr)
	if err != nil {
		return nil, fmt.Errorf("malformed github installation id %q", installationStr)
	}

	keyStr := sdk.GetConfig(configKey)
	key, err := base64.StdEncoding.DecodeString(keyStr)
	if err != nil {
		return nil, fmt.Errorf("could not decode base64 private key; %v", err)
	}

	webhookSecret := sdk.GetConfig(configWebhookKey)
	if webhookSecret == "" {
		return nil, fmt.Errorf("could not find required environment variable %q", webhookSecret)
	}

	client, err := newGithubClient(int64(app), int64(installation), key)
	if err != nil {
		return nil, fmt.Errorf("could not init Github client %v", err)
	}

	return &extension{
		client:        client,
		webhookSecret: webhookSecret,
		subscriptions: map[string]map[string][]pipelineSubscription{},
	}, nil
}

func newGithubClient(app, installation int64, key []byte) (*github.Client, error) {
	transport, err := ghinstallation.New(http.DefaultTransport, app, installation, key)
	if err != nil {
		return nil, err
	}

	client := github.NewClient(&http.Client{Transport: transport})
	// client.UserAgent = "clintjedwards/gofer"
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

	repo, metadata, err := handler(parsedPayload)
	if err != nil {
		return err
	}

	for _, sub := range t.matchSubscriptions(github.WebHookType(req), repo) {
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
			Msg("new tick for specified interval; new event spawned")
	}

	return nil
}

// matchSubscriptions returns all subscriptions with the proper event/repo combination.
func (t *extension) matchSubscriptions(event, repo string) []pipelineSubscription {
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

func (t *extension) Subscribe(ctx context.Context, request *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error) {
	eventStr, exists := request.Config[parameterEvents]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", parameterEvents)
	}

	eventList := strings.Split(eventStr, ",")

	repo, exists := request.Config[parameterRepository]
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
			event:          event,
			repository:     repo,
			extensionLabel: request.PipelineExtensionLabel,
			pipeline:       request.PipelineId,
			namespace:      request.NamespaceId,
		})
	}

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &proto.ExtensionSubscribeResponse{}, nil
}

func (t *extension) Unsubscribe(ctx context.Context, request *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error) {
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

func (t *extension) Info(ctx context.Context, request *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error) {
	return sdk.InfoResponse("https://clintjedwards.com/gofer/ref/extensions/provided/github.html")
}

func (t *extension) Shutdown(ctx context.Context, request *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error) {
	return &proto.ExtensionShutdownResponse{}, nil
}

func (t *extension) ExternalEvent(ctx context.Context, request *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error) {
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

// InstallInstructions are Gofer's way to allowing the extension to guide Gofer administrators through their
// personal installation process. This is needed because some extensions might require special auth tokens and information
// in a way that might be confusing for extension administrators.
func installInstructions() sdk.InstallInstructions {
	instructions := sdk.NewInstructionsBuilder()
	return instructions
}

// TODO():
func main() {
	extension, err := newExtension()
	if err != nil {
		panic(err)
	}

	sdk.NewExtension(extension, installInstructions())
}
