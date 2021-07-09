// Trigger interval just simply triggers the Subscribeed pipeline at the time it designates in seconds.
package main

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/clintjedwards/gofer/sdk"
	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	"github.com/rs/zerolog/log"
)

const (
	// "every" is the time between pipeline runs.
	// Supports golang native duration strings: https://pkg.go.dev/time#ParseDuration
	//
	// Examples: "1m", "60s", "3h", "3m30s"
	ParameterEvery = "every"
)

type subscription struct {
	pipelineTriggerLabel string
	pipeline             string
	namespace            string
	quit                 context.CancelFunc
}

type subscriptionID struct {
	pipelineTriggerLabel string
	pipeline             string
	namespace            string
}

type trigger struct {
	minDuration          time.Duration
	events               chan *sdkProto.CheckResponse // in-memory store to be passed to the main program through the check function
	quitAllSubscriptions context.CancelFunc
	parentContext        context.Context
	subscriptions        map[subscriptionID]*subscription // mapping of subscription id to quit channel so we can reap the goroutines.
}

func newTrigger() (*trigger, error) {
	minDurationStr := os.Getenv("GOFER_TRIGGER_INTERVAL_MIN_DURATION")
	minDuration := time.Minute * 1
	if minDurationStr != "" {
		parsedDuration, err := time.ParseDuration(minDurationStr)
		if err != nil {
			return nil, err
		}
		minDuration = parsedDuration
	}

	ctx, cancel := context.WithCancel(context.Background())

	log.Info().Dur("min_duration", minDuration).Msg("initiating trigger service")

	return &trigger{
		minDuration:          minDuration,
		events:               make(chan *sdkProto.CheckResponse, 100),
		quitAllSubscriptions: cancel,
		parentContext:        ctx,
		subscriptions:        map[subscriptionID]*subscription{},
	}, nil
}

func (t *trigger) startInterval(ctx context.Context, pipeline, namespace, pipelineTriggerLabel string, duration time.Duration) {
	for {
		select {
		case <-ctx.Done():
			return
		case <-time.After(duration):
			t.events <- &sdkProto.CheckResponse{
				Details:              "Triggered due to the passage of time.",
				PipelineTriggerLabel: pipelineTriggerLabel,
				NamespaceId:          namespace,
				PipelineId:           pipeline,
				Result:               sdkProto.CheckResponse_SUCCESS,
				Metadata:             map[string]string{},
			}
			log.Debug().Str("namespaceID", namespace).Str("pipelineID", pipeline).
				Str("trigger_label", pipelineTriggerLabel).Msg("new tick for specified interval; new event spawned")
		}
	}
}

func (t *trigger) Subscribe(ctx context.Context, request *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error) {
	interval, exists := request.Config[ParameterEvery]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q", ParameterEvery)
	}

	duration, err := time.ParseDuration(interval)
	if err != nil {
		return nil, fmt.Errorf("could not parse interval string: %w", err)
	}

	if duration < t.minDuration {
		return nil, fmt.Errorf("durations cannot be less than %s", t.minDuration)
	}

	subctx, quit := context.WithCancel(t.parentContext)
	t.subscriptions[subscriptionID{
		request.PipelineTriggerLabel,
		request.PipelineId,
		request.NamespaceId,
	}] = &subscription{request.PipelineTriggerLabel, request.NamespaceId, request.PipelineId, quit}

	go t.startInterval(subctx, request.PipelineId, request.NamespaceId, request.PipelineTriggerLabel, duration)

	log.Debug().Str("namespace_id", request.NamespaceId).Str("trigger_label", request.PipelineTriggerLabel).Str("pipeline_id", request.PipelineId).Msg("subscribed pipeline")
	return &sdkProto.SubscribeResponse{}, nil
}

func (t *trigger) Check(ctx context.Context, request *sdkProto.CheckRequest) (*sdkProto.CheckResponse, error) {
	select {
	case <-ctx.Done():
		return &sdkProto.CheckResponse{}, nil
	case event := <-t.events:
		return event, nil
	}
}

func (t *trigger) Unsubscribe(ctx context.Context, request *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error) {
	subscription, exists := t.subscriptions[subscriptionID{
		pipelineTriggerLabel: request.PipelineTriggerLabel,
		pipeline:             request.PipelineId,
		namespace:            request.NamespaceId,
	}]
	if !exists {
		return &sdkProto.UnsubscribeResponse{},
			fmt.Errorf("could not find subscription for trigger %s pipeline %s namespace %s",
				request.PipelineTriggerLabel, request.PipelineId, request.NamespaceId)
	}

	subscription.quit()
	delete(t.subscriptions, subscriptionID{
		pipelineTriggerLabel: request.PipelineTriggerLabel,
		pipeline:             request.PipelineId,
		namespace:            request.NamespaceId,
	})
	return &sdkProto.UnsubscribeResponse{}, nil
}

func (t *trigger) Info(ctx context.Context, request *sdkProto.InfoRequest) (*sdkProto.InfoResponse, error) {
	return &sdkProto.InfoResponse{
		Kind:          os.Getenv("GOFER_TRIGGER_KIND"),
		Documentation: "https://clintjedwards.com/gofer/docs/triggers/interval/overview",
	}, nil
}

func (t *trigger) ExternalEvent(ctx context.Context, request *sdkProto.ExternalEventRequest) (*sdkProto.ExternalEventResponse, error) {
	return &sdkProto.ExternalEventResponse{}, nil
}

func (t *trigger) Shutdown(ctx context.Context, request *sdkProto.ShutdownRequest) (*sdkProto.ShutdownResponse, error) {
	t.quitAllSubscriptions()
	close(t.events)

	return &sdkProto.ShutdownResponse{}, nil
}

func main() {
	trigger, err := newTrigger()
	if err != nil {
		panic(err)
	}
	sdk.NewTriggerServer(trigger)
}
