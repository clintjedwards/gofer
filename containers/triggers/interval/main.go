// Trigger interval simply triggers the subscribed pipeline at the given interval.
//
// This package is commented in such a way to make it easy to deduce what is going on, making it
// a perfect example of how to build other triggers.
//
// What is going on below is relatively simple:
//   - We create our trigger as just a regular program, paying attention to what we want our variables to be
//     when we install the trigger and when a pipeline subscribes to this trigger.
//   - We assimilate the program to become a long running trigger by using the Gofer SDK and implementing
//     the needed sdk.TriggerServiceInterface.
//   - We simply call NewTrigger and let the SDK and Gofer go to work.
package main

import (
	"context"
	"fmt"
	"strings"
	"time"

	// The proto package provides some data structures that we'll need to return to our interface.
	proto "github.com/clintjedwards/gofer/proto/go"

	// The plugins package contains a bunch of convenience functions that we use to build our trigger.
	// It is possible to build a trigger without using the SDK, but the SDK makes the process much
	// less cumbersome.
	sdk "github.com/clintjedwards/gofer/sdk/go/plugins"

	// Golang doesn't have a standardized logging interface and as such Gofer triggers can technically
	// use any logging package, but because Gofer and provided triggers use zerolog, it is heavily encouraged
	// to use zerolog. The log level for triggers is set by Gofer on trigger start via Gofer's configuration.
	"github.com/rs/zerolog/log"
)

// Triggers have two types of variables they can be passed.
//   - They take variables called "config" when they are installed.
//   - And they take variables called parameters for each pipeline that subscribes to them.

// This trigger has a single parameter called "every".
const (
	// "every" is the time between pipeline runs.
	// Supports golang native duration strings: https://pkg.go.dev/time#ParseDuration
	//
	// Examples: "1m", "60s", "3h", "3m30s"
	ParameterEvery = "every"
)

// And a single config called "min_duration".
const (
	// The minimum duration pipelines can set for the "every" parameter.
	ConfigMinDuration = "min_duration"
)

// Triggers are subscribed to by pipelines when that pipeline is registered. Gofer will call the `subscribe`
// function for the trigger and pass it details about the pipeline and the parameters it wants.
// This structure is to keep details about those subscriptions so that we may perform the triggers duties on those
// pipeline subscriptions.
type subscription struct {
	pipelineTriggerLabel string
	pipeline             string
	namespace            string
	quit                 context.CancelFunc
}

// SubscriptionID is simply a composite key of the many things that make a single subscription unique.
// We use this as the key in a hash table to lookup subscriptions. Some might wonder why label is part
// of this unique key. That is because, when relevant triggers should be expected that pipelines might
// want to subscribe more than once.
type subscriptionID struct {
	pipelineTriggerLabel string
	pipeline             string
	namespace            string
}

// Trigger is a structure that every Gofer trigger should have. It is essentially the God struct. It contains
// all information about our trigger that we might want to reference.
type trigger struct {
	// The limit for how long a pipeline configuration can request a minimum duration.
	minDuration time.Duration

	// During shutdown the trigger will want to stop all intervals immediately. Having the ability to stop all goroutines
	// is very useful.
	quitAllSubscriptions context.CancelFunc

	// The events channel is simply a store for trigger events.
	// It keeps them until Gofer calls the Watch function and requests them.
	events chan *proto.TriggerWatchResponse

	// The parent context is stored here so that we have a common parent for all goroutines we spin up.
	// This enables us to manipulate all goroutines at the same time.
	parentContext context.Context

	// Mapping of subscription id to actual subscription. The subscription in this case also contains the goroutine
	// cancel context for the specified trigger. This is important, as when a pipeline unsubscribes from a this trigger
	// we will need a way to stop that specific goroutine from running.
	subscriptions map[subscriptionID]*subscription
}

// NewTrigger is the entrypoint to the overall trigger. It sets up all initial trigger state, validates any config
// that was passed to it and generally gets the trigger ready to take requests.
func newTrigger() (*trigger, error) {
	// The GetConfig function wrap our `min duration` config and retrieves it from the environment.
	// Trigger config environment variables are passed in as "GOFER_PLUGIN_CONFIG_%s" so that they don't conflict
	// with any other environment variables that might be around.
	minDurationStr := sdk.GetConfig(ConfigMinDuration)
	minDuration := time.Minute * 1
	if minDurationStr != "" {
		parsedDuration, err := time.ParseDuration(minDurationStr)
		if err != nil {
			return nil, err
		}
		minDuration = parsedDuration
	}

	ctx, cancel := context.WithCancel(context.Background())

	return &trigger{
		minDuration:          minDuration,
		events:               make(chan *proto.TriggerWatchResponse, 100),
		quitAllSubscriptions: cancel,
		parentContext:        ctx,
		subscriptions:        map[subscriptionID]*subscription{},
	}, nil
}

// startInterval is the main logic of what enables the interval trigger to work. Each pipeline that is subscribed runs
// this function which simply waits for the set duration and then pushes a "WatchResponse" event into the trigger's main channel.
func (t *trigger) startInterval(ctx context.Context, pipeline, namespace, pipelineTriggerLabel string, duration time.Duration) {
	for {
		select {
		case <-ctx.Done():
			return
		case <-time.After(duration):
			t.events <- &proto.TriggerWatchResponse{
				Details:              "Triggered due to the passage of time.",
				PipelineTriggerLabel: pipelineTriggerLabel,
				NamespaceId:          namespace,
				PipelineId:           pipeline,
				Result:               proto.TriggerWatchResponse_SUCCESS,
				Metadata:             map[string]string{},
			}
			log.Debug().Str("namespaceID", namespace).Str("pipelineID", pipeline).
				Str("trigger_label", pipelineTriggerLabel).Msg("new tick for specified interval; new event spawned")
		}
	}
}

// Gofer calls subscribe when a pipeline configuration is being registered and a pipeline wants to subscribe to this trigger.
// The logic here is simple:
//   - We retrieve the pipeline's requested parameters.
//   - We validate the parameters.
//   - We create a new subscription object and enter it into our map.
//   - We call the `startInterval` function in a goroutine for that specific pipeline and return.
func (t *trigger) Subscribe(ctx context.Context, request *proto.TriggerSubscribeRequest) (*proto.TriggerSubscribeResponse, error) {
	interval, exists := request.Config[strings.ToUpper(ParameterEvery)]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q; received config params: %+v", ParameterEvery, request.Config)
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
	return &proto.TriggerSubscribeResponse{}, nil
}

// Gofer continuously calls the watch endpoint to receive events from the Trigger. Below we simply block until we
// are told to shutdown or we have an event to give.
func (t *trigger) Watch(ctx context.Context, request *proto.TriggerWatchRequest) (*proto.TriggerWatchResponse, error) {
	select {
	case <-ctx.Done():
		return &proto.TriggerWatchResponse{}, nil
	case event := <-t.events:
		return event, nil
	}
}

// Pipelines change and this means that sometimes they will no longer want to be executed by a particular trigger or maybe
// they want to change the previous settings on that trigger. Because of this we need a way to remove pipelines that were
// previously subscribed.
func (t *trigger) Unsubscribe(ctx context.Context, request *proto.TriggerUnsubscribeRequest) (*proto.TriggerUnsubscribeResponse, error) {
	subscription, exists := t.subscriptions[subscriptionID{
		pipelineTriggerLabel: request.PipelineTriggerLabel,
		pipeline:             request.PipelineId,
		namespace:            request.NamespaceId,
	}]
	if !exists {
		return &proto.TriggerUnsubscribeResponse{},
			fmt.Errorf("could not find subscription for trigger %s pipeline %s namespace %s",
				request.PipelineTriggerLabel, request.PipelineId, request.NamespaceId)
	}

	subscription.quit()
	delete(t.subscriptions, subscriptionID{
		pipelineTriggerLabel: request.PipelineTriggerLabel,
		pipeline:             request.PipelineId,
		namespace:            request.NamespaceId,
	})
	return &proto.TriggerUnsubscribeResponse{}, nil
}

// Info is mostly used as a health check endpoint. It returns some basic info about a trigger, the most important
// being where to get more documentation about that specific trigger.
func (t *trigger) Info(ctx context.Context, request *proto.TriggerInfoRequest) (*proto.TriggerInfoResponse, error) {
	return sdk.InfoResponse("https://clintjedwards.com/gofer/ref/triggers/provided/interval.html")
}

// The ExternalEvent endpoint tells the trigger what to do if they get messages from Gofer's external event system.
// This system is set up to facilitate webhook interactions like those that occur for github
// (A user pushes a branch, Gofer gets an event from github).
// The ExternalEvent will come with a payload which the trigger can then authenticate, process, and take action on.
func (t *trigger) ExternalEvent(ctx context.Context, request *proto.TriggerExternalEventRequest) (*proto.TriggerExternalEventResponse, error) {
	return &proto.TriggerExternalEventResponse{}, nil
}

// A graceful shutdown for a trigger should clean up any resources it was working with that might be left hanging.
// Sometimes that means sending requests to third parties that it is shutting down, sometimes that just means
// reaping its personal goroutines.
func (t *trigger) Shutdown(ctx context.Context, request *proto.TriggerShutdownRequest) (*proto.TriggerShutdownResponse, error) {
	t.quitAllSubscriptions()
	close(t.events)

	return &proto.TriggerShutdownResponse{}, nil
}

// InstallInstructions are Gofer's way to allowing the trigger to guide Gofer administrators through their
// personal installation process. This is needed because some triggers might require special auth tokens and information
// in a way that might be confusing for trigger administrators.
func installInstructions() sdk.InstallInstructions {
	instructions := sdk.NewInstructionsBuilder()
	instructions = instructions.AddMessage(":: The interval trigger allows users to trigger their pipelines on the passage"+
		" of time by setting a particular duration.").
		AddMessage("").
		AddMessage("First, let's prevent users from setting too low of an interval by setting a minimum duration. "+
			"Durations are set via Golang duration strings. For example, entering a duration of '10h' would be 10 hours, "+
			"meaning that users can only run their pipeline at most every 10 hours. "+
			"You can find more documentation on valid strings here: https://pkg.go.dev/time#ParseDuration.").
		AddQuery("Set a minimum duration for all pipelines", ConfigMinDuration)

	return instructions
}

// Lastly we call our personal NewTrigger function, which now implements the TriggerServerInterface and then we
// pass it to the NewTrigger function within the SDK.
//
// From here the SDK will use the given interface and run a GRPC service whenever this program is called with the
// positional parameter "server". Ex. "./trigger server"
//
// Whenever this program is called with the parameter "installer" then it will print out the installation instructions
// instead.
func main() {
	trigger, err := newTrigger()
	if err != nil {
		panic(err)
	}
	sdk.NewTrigger(trigger, installInstructions())
}
