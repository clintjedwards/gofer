package main

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/clintjedwards/avail/v2"
	"github.com/clintjedwards/gofer/sdk"
	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
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
	events        chan *sdkProto.CheckResponse
	subscriptions []subscription
}

func newTrigger() *trigger {
	return &trigger{
		events: make(chan *sdkProto.CheckResponse, 100),
	}
}

func (t *trigger) checkTimeFrames() {
	for _, subscription := range t.subscriptions {
		if subscription.timeframe.Able(time.Now()) {
			t.events <- &sdkProto.CheckResponse{
				Details: fmt.Sprintf("Triggered due to current time %q being within the timeframe expression %q",
					time.Now().Format(time.RFC1123), subscription.timeframe.Expression),
				PipelineTriggerLabel: subscription.pipelineTriggerLabel,
				PipelineId:           subscription.pipeline,
				NamespaceId:          subscription.namespace,
				Result:               sdkProto.CheckResponse_SUCCESS,
				Metadata:             map[string]string{},
			}

			log.Debug().Str("trigger_label", subscription.pipelineTriggerLabel).Str("pipeline_id", subscription.pipeline).
				Str("namespace_id", subscription.namespace).Msg("Pipeline within timeframe; new event spawned")
		}
	}
}

func (t *trigger) Subscribe(ctx context.Context, request *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error) {
	expression, exists := request.Config[ParameterExpression]
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
	return &sdkProto.SubscribeResponse{}, nil
}

func (t *trigger) Unsubscribe(ctx context.Context, request *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error) {
	for index, subscription := range t.subscriptions {
		if subscription.pipelineTriggerLabel == request.PipelineTriggerLabel &&
			subscription.namespace == request.NamespaceId &&
			subscription.pipeline == request.PipelineId {
			t.subscriptions = append(t.subscriptions[:index], t.subscriptions[index+1:]...)
			return &sdkProto.UnsubscribeResponse{}, nil
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
		Documentation: "https://clintjedwards.com/gofer/docs/triggers/cron/overview",
	}, nil
}

func (t *trigger) Shutdown(ctx context.Context, request *sdkProto.ShutdownRequest) (*sdkProto.ShutdownResponse, error) {
	close(t.events)
	return &sdkProto.ShutdownResponse{}, nil
}

func (t *trigger) ExternalEvent(ctx context.Context, request *sdkProto.ExternalEventRequest) (*sdkProto.ExternalEventResponse, error) {
	return &sdkProto.ExternalEventResponse{}, nil
}

func main() {
	newTrigger := newTrigger()

	go func(t *trigger) {
		for {
			time.Sleep(checkInterval)
			log.Debug().Time("check_time", time.Now()).Msg("checking time frames")
			t.checkTimeFrames()
		}
	}(newTrigger)

	sdk.NewTriggerServer(newTrigger)
}
