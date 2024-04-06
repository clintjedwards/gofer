package main

import (
	"context"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/clintjedwards/avail/v2"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	extsdk "github.com/clintjedwards/gofer/sdk/go/extensions"
	"github.com/rs/zerolog/log"
)

// "expression is the cron expression for pipeline scheduling"
const ParameterExpression = "expression"

var checkInterval = time.Minute

type subscription struct {
	namespace              string
	pipeline               string
	pipelineExtensionLabel string
	timeframe              avail.Timeframe
}

type subscriptionID struct {
	namespace              string
	pipeline               string
	pipelineExtensionLabel string
}

type extension struct {
	isInitialized bool
	subscriptions map[subscriptionID]*subscription
	client        *sdk.Client
}

func (e *extension) Init(ctx context.Context, request *extsdk.ExtensionInitRequest) (*extsdk.ExtensionInitResponse, *extsdk.ExtensionError) {
	config, _ := extsdk.GetExtensionSystemConfig()

	e.subscriptions = map[subscriptionID]*subscription{}
	e.isInitialized = true
	client, err := sdk.NewClient(config.GoferHost)
	if err != nil {
		return nil, extsdk.NewExtensionError(http.StatusInternalServerError, "Could not initialize client to Gofer API")
	}
	e.client = client

	go func() {
		for {
			time.Sleep(checkInterval)
			e.checkTimeFrames()
		}
	}()

	return &extsdk.ExtensionInitResponse{}, nil
}

func (e *extension) checkTimeFrames() {
	for _, subscription := range e.subscriptions {
		if subscription.timeframe.Able(time.Now()) {
			config, _ := extsdk.GetExtensionSystemConfig()

			resp, err := client.StartRun(ctx, &extsdk.StartRunRequest{
				NamespaceId: subscription.namespace,
				PipelineId:  subscription.pipeline,
				Variables:   map[string]string{},
				Initiator: &extsdk.Initiator{
					Type:   extsdk.Initiator_EXTENSION,
					Name:   fmt.Sprintf("%s (%s)", config.Name, subscription.pipelineExtensionLabel),
					Reason: fmt.Sprintf("Triggered due to current time %q being within the timeframe expression %q", time.Now().Format(time.RFC1123), subscription.timeframe.Expression),
				},
			})
			if err != nil {
				log.Error().Str("namespaceID", subscription.namespace).Str("pipelineID", subscription.pipeline).
					Str("extension_label", subscription.pipelineExtensionLabel).Msg("could not start new run")

				continue
			}

			log.Debug().Str("extension_label", subscription.pipelineExtensionLabel).Str("pipeline_id", subscription.pipeline).
				Str("namespace_id", subscription.namespace).Int64("run_id", resp.Run.Id).
				Msg("Pipeline within timeframe; new event spawned")
		}
	}
}

func (e *extension) Subscribe(ctx context.Context, request *extsdk.ExtensionSubscribeRequest) (*extsdk.ExtensionSubscribeResponse, error) {
	expression, exists := request.Config[strings.ToUpper(ParameterExpression)]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", ParameterExpression)
	}

	timeframe, err := avail.New(expression)
	if err != nil {
		return nil, fmt.Errorf("could not parse expression: %q", err)
	}

	subID := subscriptionID{
		request.NamespaceId,
		request.PipelineId,
		request.PipelineExtensionLabel,
	}

	// It is perfectly possible for Gofer to attempt to subscribe an already subscribed pipeline. In this case,
	// we can simply ignore the request.
	_, exists = e.subscriptions[subID]
	if exists {
		log.Debug().Str("namespace_id", request.NamespaceId).Str("extension_label", request.PipelineExtensionLabel).
			Str("pipeline_id", request.PipelineId).Msg("pipeline already subscribed; ignoring request")
		return &extsdk.ExtensionSubscribeResponse{}, nil
	}

	// While it might result in a faster check to start a goroutine for each subscription the interval
	// for most of these expressions should be on the order of minutes. So one event loop checking the
	// result for all of them should still result in no missed checks.
	e.subscriptions[subID] = &subscription{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
		timeframe:              timeframe,
	}

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &extsdk.ExtensionSubscribeResponse{}, nil
}

func (e *extension) Unsubscribe(ctx context.Context, request *extsdk.ExtensionUnsubscribeRequest) (*extsdk.ExtensionUnsubscribeResponse, error) {
	subID := subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
	}

	delete(e.subscriptions, subID)

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return &extsdk.ExtensionUnsubscribeResponse{}, nil
}

func (e *extension) Info(ctx context.Context, request *extsdk.ExtensionInfoRequest) (*extsdk.ExtensionInfoResponse, error) {
	registered := []string{}
	for _, sub := range e.subscriptions {
		registered = append(registered, fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline))
	}

	config, _ := extsdk.GetExtensionSystemConfig()

	return &extsdk.ExtensionInfoResponse{
		Name:          config.Name,
		Documentation: "https://clintjedwards.com/gofer/ref/extensions/provided/cron.html",
		Registered:    registered,
	}, nil
}

func (e *extension) Shutdown(ctx context.Context, request *extsdk.ExtensionShutdownRequest) (*extsdk.ExtensionShutdownResponse, error) {
	return &extsdk.ExtensionShutdownResponse{}, nil
}

func (e *extension) ExternalEvent(ctx context.Context, request *extsdk.ExtensionExternalEventRequest) (*extsdk.ExtensionExternalEventResponse, error) {
	return &extsdk.ExtensionExternalEventResponse{}, nil
}

// The ExtensionInstaller is a small script that gets piped to the admin who is trying to set up this particular
// extension. The installer is meant to guide the user through the different configuration options that the
// installer has globally.
func (e *extension) RunExtensionInstaller(stream extsdk.ExtensionService_RunExtensionInstallerServer) error {
	err := extsdk.SendInstallerMessageToClient(stream, "The cron extension allows users to run their pipelines on the passage "+
		"of time by setting particular timeframes. There are no configuration options for the cron extension.")
	if err != nil {
		return err
	}

	return nil
}

// The PipelineConfigurator is a small script that a pipeline owner can run when subscribing to this extension.
// It's meant to guide the pipeline owner through the different options of the extension.
func (e *extension) RunPipelineConfigurator(stream extsdk.ExtensionService_RunPipelineConfiguratorServer) error {
	err := extsdk.SendConfiguratorMessageToClient(stream, "The cron extension allows users to run their pipelines on the passage "+
		"of time by setting particular timeframes.\n")
	if err != nil {
		return err
	}

	err = extsdk.SendConfiguratorMessageToClient(stream, `It uses a stripped down version of the cron syntax to do so:

	Field           Allowed values  Allowed special characters

	Minutes         0-59            * , -
	Hours           0-23            * , -
	Day of month    1-31            * , -
	Month           1-12            * , -
	Day of week     0-6             * , -
	Year            1970-2100       * , -
`)
	if err != nil {
		return err
	}

	err = extsdk.SendConfiguratorMessageToClient(stream, "For example the cron expression '0 1 25 12 * *' would run a pipeline every year on Christmas.")
	if err != nil {
		return err
	}

	err = extsdk.SendConfiguratorMessageToClient(stream, "You can read more information about the cron format here: https://clintjedwards.com/gofer/ref/extensions/provided/cron.html\n")
	if err != nil {
		return err
	}

	err = extsdk.SendConfiguratorQueryToClient(stream, "Set your pipeline run cron expression: ")
	if err != nil {
		return err
	}

	clientMsg, err := stream.Recv()
	if err != nil {
		return err
	}

	_, err = avail.New(clientMsg.Msg)
	if err != nil {
		err = extsdk.SendConfiguratorMessageToClient(stream, fmt.Sprintf("Malformed expression %q; %v", clientMsg.Msg, err))
		if err != nil {
			return err
		}
	}

	err = extsdk.SendConfiguratorParamSettingToClient(stream, ParameterExpression, clientMsg.Msg)
	if err != nil {
		return err
	}

	return nil
}

func main() {
	extension := extension{}
	extsdk.NewExtension(&extension)
}
