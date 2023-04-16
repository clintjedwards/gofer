package main

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/clintjedwards/avail/v2"
	proto "github.com/clintjedwards/gofer/proto/go"
	sdk "github.com/clintjedwards/gofer/sdk/go/extensions"
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
	subscriptions map[subscriptionID]*subscription
}

func newExtension() *extension {
	return &extension{}
}

func (t *extension) checkTimeFrames() {
	for _, subscription := range t.subscriptions {
		if subscription.timeframe.Able(time.Now()) {
			client, ctx, err := sdk.Connect()
			if err != nil {
				log.Error().Str("namespace_id", subscription.namespace).Str("pipeline_id", subscription.pipeline).
					Str("extension_label", subscription.pipelineExtensionLabel).Msg("could not connect to Gofer")

				continue
			}

			config, _ := sdk.GetExtensionSystemConfig()

			resp, err := client.StartRun(ctx, &proto.StartRunRequest{
				NamespaceId: subscription.namespace,
				PipelineId:  subscription.pipeline,
				Variables:   map[string]string{},
				Initiator: &proto.Initiator{
					Type:   proto.Initiator_EXTENSION,
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

func (t *extension) Subscribe(ctx context.Context, request *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error) {
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
	_, exists = t.subscriptions[subID]
	if exists {
		log.Debug().Str("namespace_id", request.NamespaceId).Str("extension_label", request.PipelineExtensionLabel).
			Str("pipeline_id", request.PipelineId).Msg("pipeline already subscribed; ignoring request")
		return &proto.ExtensionSubscribeResponse{}, nil
	}

	// While it might result in a faster check to start a goroutine for each subscription the interval
	// for most of these expressions should be on the order of minutes. So one event loop checking the
	// result for all of them should still result in no missed checks.
	t.subscriptions[subID] = &subscription{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
		timeframe:              timeframe,
	}

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &proto.ExtensionSubscribeResponse{}, nil
}

func (t *extension) Unsubscribe(ctx context.Context, request *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error) {
	subID := subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
	}

	delete(t.subscriptions, subID)

	log.Debug().Str("extension_label", request.PipelineExtensionLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return &proto.ExtensionUnsubscribeResponse{}, nil
}

func (t *extension) Info(ctx context.Context, request *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error) {
	registered := []string{}
	for _, sub := range t.subscriptions {
		registered = append(registered, fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline))
	}

	config, _ := sdk.GetExtensionSystemConfig()

	return &proto.ExtensionInfoResponse{
		Name:          config.Name,
		Documentation: "https://clintjedwards.com/gofer/ref/extensions/provided/cron.html",
		Registered:    registered,
	}, nil
}

func (t *extension) Shutdown(ctx context.Context, request *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error) {
	return &proto.ExtensionShutdownResponse{}, nil
}

func installInstructions() sdk.InstallInstructions {
	instructions := sdk.NewInstructionsBuilder()
	instructions = instructions.AddMessage(":: The cron extension allows users to extension their pipelines on the passage" +
		" of time by setting particular timeframes.").
		AddMessage("").
		AddMessage("There are no configuration options for the cron extension.")

	return instructions
}

func (t *extension) ExternalEvent(ctx context.Context, request *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error) {
	return &proto.ExtensionExternalEventResponse{}, nil
}

func main() {
	newExtension := newExtension()

	go func(t *extension) {
		for {
			time.Sleep(checkInterval)
			t.checkTimeFrames()
		}
	}(newExtension)

	sdk.NewExtension(newExtension, installInstructions())
}
