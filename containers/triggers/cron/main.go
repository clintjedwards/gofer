package main

import (
	"context"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/clintjedwards/avail/v2"
	proto "github.com/clintjedwards/gofer/proto/go"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	"github.com/rs/zerolog/log"
)

// "expression is the cron expression for pipeline scheduling"
const ParameterExpression = "expression"

var checkInterval = time.Minute

type subscription struct {
	pipelineTriggerLabel string
	pipeline             string
	namespace            string
	timeframe            avail.Timeframe
}

type trigger struct {
	events        chan *proto.TriggerWatchResponse
	subscriptions []subscription
}

func newTrigger() *trigger {
	return &trigger{
		events: make(chan *proto.TriggerWatchResponse, 100),
	}
}

func (t *trigger) checkTimeFrames() {
	for _, subscription := range t.subscriptions {
		if subscription.timeframe.Able(time.Now()) {
			t.events <- &proto.TriggerWatchResponse{
				Details: fmt.Sprintf("Triggered due to current time %q being within the timeframe expression %q",
					time.Now().Format(time.RFC1123), subscription.timeframe.Expression),
				NamespaceId:          subscription.namespace,
				PipelineId:           subscription.pipeline,
				PipelineTriggerLabel: subscription.pipelineTriggerLabel,
				Result:               proto.TriggerWatchResponse_SUCCESS,
				Metadata:             map[string]string{},
			}

			log.Debug().Str("trigger_label", subscription.pipelineTriggerLabel).Str("pipeline_id", subscription.pipeline).
				Str("namespace_id", subscription.namespace).Msg("Pipeline within timeframe; new event spawned")
		}
	}
}

func (t *trigger) Subscribe(ctx context.Context, request *proto.TriggerSubscribeRequest) (*proto.TriggerSubscribeResponse, error) {
	expression, exists := request.Config[strings.ToUpper(ParameterExpression)]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", ParameterExpression)
	}

	timeframe, err := avail.New(expression)
	if err != nil {
		return nil, fmt.Errorf("could not parse expression: %q", err)
	}

	// While it might result in a faster check to start a goroutine for each subscription the interval
	// for most of these expressions should be on the order of minutes. So one event loop checking the
	// result for all of them should still result in no missed checks.
	t.subscriptions = append(t.subscriptions, subscription{
		pipelineTriggerLabel: request.PipelineTriggerLabel,
		pipeline:             request.PipelineId,
		namespace:            request.NamespaceId,
		timeframe:            timeframe,
	})

	log.Debug().Str("trigger_label", request.PipelineTriggerLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return &proto.TriggerSubscribeResponse{}, nil
}

func (t *trigger) Unsubscribe(ctx context.Context, request *proto.TriggerUnsubscribeRequest) (*proto.TriggerUnsubscribeResponse, error) {
	for index, subscription := range t.subscriptions {
		if subscription.pipelineTriggerLabel == request.PipelineTriggerLabel &&
			subscription.namespace == request.NamespaceId &&
			subscription.pipeline == request.PipelineId {
			t.subscriptions = append(t.subscriptions[:index], t.subscriptions[index+1:]...)
			return &proto.TriggerUnsubscribeResponse{}, nil
		}
	}

	log.Debug().Str("trigger_label", request.PipelineTriggerLabel).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return &proto.TriggerUnsubscribeResponse{}, nil
}

func (t *trigger) Watch(ctx context.Context, request *proto.TriggerWatchRequest) (*proto.TriggerWatchResponse, error) {
	select {
	case <-ctx.Done():
		return &proto.TriggerWatchResponse{}, nil
	case event := <-t.events:
		return event, nil
	}
}

func (t *trigger) Info(ctx context.Context, request *proto.TriggerInfoRequest) (*proto.TriggerInfoResponse, error) {
	registered := []string{}
	for _, sub := range t.subscriptions {
		registered = append(registered, fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline))
	}

	return &proto.TriggerInfoResponse{
		Name:          os.Getenv("GOFER_TRIGGER_NAME"),
		Documentation: "https://clintjedwards.com/gofer/docs/triggers/cron/overview",
		Registered:    registered,
	}, nil
}

func (t *trigger) Shutdown(ctx context.Context, request *proto.TriggerShutdownRequest) (*proto.TriggerShutdownResponse, error) {
	close(t.events)
	return &proto.TriggerShutdownResponse{}, nil
}

func installInstructions() sdk.InstallInstructions {
	instructions := sdk.NewInstructionsBuilder()
	instructions = instructions.AddMessage(":: The cron trigger allows users to trigger their pipelines on the passage" +
		" of time by setting particular timeframes.").
		AddMessage("").
		AddMessage("There are no configuration options for the cron trigger.")

	return instructions
}

func (t *trigger) ExternalEvent(ctx context.Context, request *proto.TriggerExternalEventRequest) (*proto.TriggerExternalEventResponse, error) {
	return &proto.TriggerExternalEventResponse{}, nil
}

func main() {
	newTrigger := newTrigger()

	go func(t *trigger) {
		for {
			time.Sleep(checkInterval)
			t.checkTimeFrames()
		}
	}(newTrigger)

	sdk.NewTrigger(newTrigger, installInstructions())
}
