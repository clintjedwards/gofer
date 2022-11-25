// Extension interval simply extensions the subscribed pipeline at the given interval.
//
// This package is commented in such a way to make it easy to deduce what is going on, making it
// a perfect example of how to build other extensions.
//
// What is going on below is relatively simple:
//   - We create our extension as just a regular program, paying attention to what we want our variables to be
//     when we install the extension and when a pipeline subscribes to this extension.
//   - We assimilate the program to become a long running extension by using the Gofer SDK and implementing
//     the needed sdk.ExtensionServiceInterface.
//   - We simply call NewExtension and let the SDK and Gofer go to work.
package main

import (
	"context"
	"fmt"
	"strings"
	"time"

	// The proto package provides some data structures that we'll need to return to our interface.
	proto "github.com/clintjedwards/gofer/proto/go"

	// The plugins package contains a bunch of convenience functions that we use to build our extension.
	// It is possible to build a extension without using the SDK, but the SDK makes the process much
	// less cumbersome.
	sdk "github.com/clintjedwards/gofer/sdk/go/plugins"

	// Golang doesn't have a standardized logging interface and as such Gofer extensions can technically
	// use any logging package, but because Gofer and provided extensions use zerolog, it is heavily encouraged
	// to use zerolog. The log level for extensions is set by Gofer on extension start via Gofer's configuration.
	"github.com/rs/zerolog/log"
)

// Extensions have two types of variables they can be passed.
//   - They take variables called "config" when they are installed.
//   - And they take variables called parameters for each pipeline that subscribes to them.

// This extension has a single parameter called "every".
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

// Extensions are subscribed to by pipelines when that pipeline is registered. Gofer will call the `subscribe`
// function for the extension and pass it details about the pipeline and the parameters it wants.
// This structure is to keep details about those subscriptions so that we may perform the extensions duties on those
// pipeline subscriptions.
type subscription struct {
	namespace              string
	pipeline               string
	pipelineExtensionLabel string
	quit                   context.CancelFunc
}

// SubscriptionID is simply a composite key of the many things that make a single subscription unique.
// We use this as the key in a hash table to lookup subscriptions. Some might wonder why label is part
// of this unique key. That is because, when relevant extensions should be expected that pipelines might
// want to subscribe more than once.
type subscriptionID struct {
	namespace              string
	pipeline               string
	pipelineExtensionLabel string
}

// Extension is a structure that every Gofer extension should have. It is essentially the God struct. It contains
// all information about our extension that we might want to reference.
type extension struct {
	// The limit for how long a pipeline configuration can request a minimum duration.
	minDuration time.Duration

	// During shutdown the extension will want to stop all intervals immediately. Having the ability to stop all goroutines
	// is very useful.
	quitAllSubscriptions context.CancelFunc

	// The events channel is simply a store for extension events.
	// It keeps them until Gofer calls the Watch function and requests them.
	events chan *proto.ExtensionWatchResponse

	// The parent context is stored here so that we have a common parent for all goroutines we spin up.
	// This enables us to manipulate all goroutines at the same time.
	parentContext context.Context

	// Mapping of subscription id to actual subscription. The subscription in this case also contains the goroutine
	// cancel context for the specified extension. This is important, as when a pipeline unsubscribes from a this extension
	// we will need a way to stop that specific goroutine from running.
	subscriptions map[subscriptionID]*subscription
}

// NewExtension is the entrypoint to the overall extension. It sets up all initial extension state, validates any config
// that was passed to it and generally gets the extension ready to take requests.
func newExtension() (*extension, error) {
	// The GetConfig function wrap our `min duration` config and retrieves it from the environment.
	// Extension config environment variables are passed in as "GOFER_PLUGIN_CONFIG_%s" so that they don't conflict
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

	return &extension{
		minDuration:          minDuration,
		events:               make(chan *proto.ExtensionWatchResponse, 100),
		quitAllSubscriptions: cancel,
		parentContext:        ctx,
		subscriptions:        map[subscriptionID]*subscription{},
	}, nil
}

// startInterval is the main logic of what enables the interval extension to work. Each pipeline that is subscribed runs
// this function which simply waits for the set duration and then pushes a "WatchResponse" event into the extension's main channel.
func (t *extension) startInterval(ctx context.Context, namespace, pipeline string, pipelineExtensionLabel string, duration time.Duration,
) {
	for {
		select {
		case <-ctx.Done():
			return
		case <-time.After(duration):
			t.events <- &proto.ExtensionWatchResponse{
				Details:                "Extensioned due to the passage of time.",
				PipelineExtensionLabel: pipelineExtensionLabel,
				NamespaceId:            namespace,
				PipelineId:             pipeline,
				Result:                 proto.ExtensionWatchResponse_SUCCESS,
				Metadata:               map[string]string{},
			}
			log.Debug().Str("namespaceID", namespace).Str("pipelineID", pipeline).
				Str("extension_label", pipelineExtensionLabel).Msg("new tick for specified interval; new event spawned")
		}
	}
}

// Gofer calls subscribe when a pipeline configuration is being registered and a pipeline wants to subscribe to this extension.
// The logic here is simple:
//   - We retrieve the pipeline's requested parameters.
//   - We validate the parameters.
//   - We create a new subscription object and enter it into our map.
//   - We call the `startInterval` function in a goroutine for that specific pipeline and return.
func (t *extension) Subscribe(ctx context.Context, request *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error) {
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

	subctx, quit := context.WithCancel(t.parentContext)
	t.subscriptions[subID] = &subscription{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
		quit:                   quit,
	}

	go t.startInterval(subctx, request.NamespaceId, request.PipelineId, request.PipelineExtensionLabel, duration)

	log.Debug().Str("namespace_id", request.NamespaceId).Str("extension_label", request.PipelineExtensionLabel).
		Str("pipeline_id", request.PipelineId).Msg("subscribed pipeline")
	return &proto.ExtensionSubscribeResponse{}, nil
}

// Gofer continuously calls the watch endpoint to receive events from the Extension. Below we simply block until we
// are told to shutdown or we have an event to give.
func (t *extension) Watch(ctx context.Context, request *proto.ExtensionWatchRequest) (*proto.ExtensionWatchResponse, error) {
	select {
	case <-ctx.Done():
		return &proto.ExtensionWatchResponse{}, nil
	case event := <-t.events:
		return event, nil
	}
}

// Pipelines change and this means that sometimes they will no longer want to be executed by a particular extension or maybe
// they want to change the previous settings on that extension. Because of this we need a way to remove pipelines that were
// previously subscribed.
func (t *extension) Unsubscribe(ctx context.Context, request *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error) {
	subscription, exists := t.subscriptions[subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
	}]

	// It is perfectly possible for Gofer to attempt to unsubscribe an already unsubscribed pipeline. In this case,
	// we can simply ignore the request.
	if !exists {
		log.Debug().Str("namespace_id", request.NamespaceId).Str("extension_label", request.PipelineExtensionLabel).
			Str("pipeline_id", request.PipelineId).Msg("no subscription found for pipeline")
		return &proto.ExtensionUnsubscribeResponse{}, nil
	}

	subscription.quit()
	delete(t.subscriptions, subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
	})
	return &proto.ExtensionUnsubscribeResponse{}, nil
}

// Info is mostly used as a health check endpoint. It returns some basic info about a extension, the most important
// being where to get more documentation about that specific extension.
func (t *extension) Info(ctx context.Context, request *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error) {
	return sdk.InfoResponse("https://clintjedwards.com/gofer/ref/extensions/provided/interval.html")
}

// The ExternalEvent endpoint tells the extension what to do if they get messages from Gofer's external event system.
// This system is set up to facilitate webhook interactions like those that occur for github
// (A user pushes a branch, Gofer gets an event from github).
// The ExternalEvent will come with a payload which the extension can then authenticate, process, and take action on.
func (t *extension) ExternalEvent(ctx context.Context, request *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error) {
	return &proto.ExtensionExternalEventResponse{}, nil
}

// A graceful shutdown for a extension should clean up any resources it was working with that might be left hanging.
// Sometimes that means sending requests to third parties that it is shutting down, sometimes that just means
// reaping its personal goroutines.
func (t *extension) Shutdown(ctx context.Context, request *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error) {
	t.quitAllSubscriptions()
	close(t.events)

	return &proto.ExtensionShutdownResponse{}, nil
}

// InstallInstructions are Gofer's way to allowing the extension to guide Gofer administrators through their
// personal installation process. This is needed because some extensions might require special auth tokens and information
// in a way that might be confusing for extension administrators.
func installInstructions() sdk.InstallInstructions {
	instructions := sdk.NewInstructionsBuilder()
	instructions = instructions.AddMessage(":: The interval extension allows users to extension their pipelines on the passage"+
		" of time by setting a particular duration.").
		AddMessage("").
		AddMessage("First, let's prevent users from setting too low of an interval by setting a minimum duration. "+
			"Durations are set via Golang duration strings. For example, entering a duration of '10h' would be 10 hours, "+
			"meaning that users can only run their pipeline at most every 10 hours. "+
			"You can find more documentation on valid strings here: https://pkg.go.dev/time#ParseDuration.").
		AddQuery("Set a minimum duration for all pipelines", ConfigMinDuration)

	return instructions
}

// Lastly we call our personal NewExtension function, which now implements the ExtensionServerInterface and then we
// pass it to the NewExtension function within the SDK.
//
// From here the SDK will use the given interface and run a GRPC service whenever this program is called with the
// positional parameter "server". Ex. "./extension server"
//
// Whenever this program is called with the parameter "installer" then it will print out the installation instructions
// instead.
func main() {
	extension, err := newExtension()
	if err != nil {
		panic(err)
	}
	sdk.NewExtension(extension, installInstructions())
}
