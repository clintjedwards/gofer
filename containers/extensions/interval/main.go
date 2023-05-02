// Extension interval simply runs the subscribed pipeline at the given interval.
//
// This package is commented in such a way to make it easy to deduce what is going on, making it
// a perfect example of how to build other extensions.
//
// What is going on below is relatively simple:
//   - All extensions are run as long-running containers.
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

	// The sdk package contains a bunch of convenience functions that we use to build our extension.
	// It is possible to build a extension without using the SDK, but the SDK makes the process much
	// less cumbersome.
	sdk "github.com/clintjedwards/gofer/sdk/go/extensions"

	// Golang doesn't have a standardized logging interface and as such Gofer extensions can technically
	// use any logging package, but because Gofer and provided extensions use zerolog, it is heavily encouraged
	// to use zerolog. The log level for extensions is set by Gofer on extension start via Gofer's configuration.
	// And logs are interleaved in the stdout for the main program.
	"github.com/rs/zerolog/log"
)

// Extensions have two types of variables they can be passed.
//   - They take variables called "config" when they are installed.
//   - They take variables called "parameters" for each pipeline that subscribes to them.

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
	// The minimum interval pipelines can set for the "every" parameter.
	ConfigMinInterval = "min_interval"
)

// Extensions are subscribed to by pipelines. Gofer will call the `subscribe` function for the extension and
// pass it details about the pipeline and the parameters it wants.
// This structure is meant to keep details about those subscriptions so that we may
// perform the extension's duties on those pipeline subscriptions.
type subscription struct {
	namespace              string
	pipeline               string
	pipelineExtensionLabel string
	quit                   context.CancelFunc
}

// SubscriptionID is simply a composite key of the many things that make a single subscription unique.
// We use this as the key in a hash table to lookup subscriptions. Some might wonder why label is part
// of this unique key. That is because extensions should expect that pipelines might
// want to subscribe more than once.
type subscriptionID struct {
	namespace              string
	pipeline               string
	pipelineExtensionLabel string
}

// Extension is a structure that every Gofer extension should have. It is essentially a God struct that coordinates things
// for the extension as a whole. It contains all information about our extension that we might want to reference.
type extension struct {
	// Extensions can be run without "initializing" them. This allows Gofer to run things like the installer without
	// having to pass the extension everything it needs to work for normal cases.
	// It might be useful to track whether the extension was initialized or not.
	isInitialized bool

	// The lower limit for how often a pipeline can request to be run.
	minInterval time.Duration

	// During shutdown the extension will want to stop all intervals immediately. Having the ability to stop all goroutines
	// is very useful.
	quitAllSubscriptions context.CancelFunc

	// The parent context is stored here so that we have a common parent for all goroutines we spin up.
	// This enables us to manipulate all goroutines at the same time.
	parentContext context.Context

	// Mapping of subscription id to actual subscription. The subscription in this case also contains the goroutine
	// cancel context for the specified extension. This is important, as when a pipeline unsubscribes from a this extension
	// we will need a way to stop that specific goroutine from running.
	subscriptions map[subscriptionID]*subscription

	// Generic extension configuration set by Gofer at startup. Useful for interacting with Gofer.
	systemConfig sdk.ExtensionSystemConfig
}

// Init serves to set up the extension for it's main functionality. It is needed mostly because we sometimes need
// extensions to launch, but not actually serve all requests(like when we're running the extensions install endpoints).
//
// The Gofer server when launching an extension will call the Init endpoint and then wait until
// a successful response is returned to mark the extension ready to take subscriptions.
func (e *extension) Init(ctx context.Context, request *proto.ExtensionInitRequest) (*proto.ExtensionInitResponse, error) {
	minDurationStr := request.Config[ConfigMinInterval]
	minDuration := time.Minute * 1
	if minDurationStr != "" {
		parsedDuration, err := time.ParseDuration(minDurationStr)
		if err != nil {
			return nil, err
		}
		minDuration = parsedDuration
	}

	e.parentContext, e.quitAllSubscriptions = context.WithCancel(context.Background())
	e.minInterval = minDuration
	e.subscriptions = map[subscriptionID]*subscription{}

	config, _ := sdk.GetExtensionSystemConfig()
	e.systemConfig = config
	e.isInitialized = true

	return &proto.ExtensionInitResponse{}, nil
}

// startInterval is the main logic of what enables the interval extension to work. Each pipeline that is subscribed runs
// this function which simply waits for the set duration and then calls the StartRun endpoint for Gofer.
func (e *extension) startInterval(ctx context.Context, namespace, pipeline string, pipelineExtensionLabel string, duration time.Duration,
) {
	for {
		select {
		case <-ctx.Done():
			return
		case <-time.After(duration):
			client, ctx, err := sdk.Connect()
			if err != nil {
				log.Error().Err(err).Str("namespaceID", namespace).Str("pipelineID", pipeline).
					Str("extension_label", pipelineExtensionLabel).Msg("could not connect to Gofer")

				continue
			}

			config, _ := sdk.GetExtensionSystemConfig()

			resp, err := client.StartRun(ctx, &proto.StartRunRequest{
				NamespaceId: namespace,
				PipelineId:  pipeline,
				Variables:   map[string]string{},
				Initiator: &proto.Initiator{
					Type:   proto.Initiator_EXTENSION,
					Name:   fmt.Sprintf("%s (%s)", config.Name, pipelineExtensionLabel),
					Reason: "Triggered due to the passage of time",
				},
			})
			if err != nil {
				log.Error().Err(err).Str("namespaceID", namespace).Str("pipelineID", pipeline).
					Str("extension_label", pipelineExtensionLabel).Msg("could not start new run")

				continue
			}

			log.Debug().Str("namespace_id", namespace).Str("pipeline_id", pipeline).
				Str("extension_label", pipelineExtensionLabel).Int64("run_id", resp.Run.Id).
				Msg("new tick for specified interval; new event spawned")
		}
	}
}

// Gofer calls subscribe when a pipeline wants to subscribe to this extension.
// The logic here is simple:
//   - Retrieve the pipeline's requested parameters.
//   - Validate the parameters.
//   - Create a new subscription object and enter it into our map.
//   - Call the `startInterval` function in a goroutine for that specific pipeline and return.
func (e *extension) Subscribe(ctx context.Context, request *proto.ExtensionSubscribeRequest) (*proto.ExtensionSubscribeResponse, error) {
	interval, exists := request.Config[strings.ToUpper(ParameterEvery)]
	if !exists {
		return nil, fmt.Errorf("could not find required configuration parameter %q; received config params: %+v", ParameterEvery, request.Config)
	}

	duration, err := time.ParseDuration(interval)
	if err != nil {
		return nil, fmt.Errorf("could not parse interval string: %w", err)
	}

	if duration < e.minInterval {
		return nil, fmt.Errorf("durations cannot be less than %s", e.minInterval)
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
		return &proto.ExtensionSubscribeResponse{}, nil
	}

	subctx, quit := context.WithCancel(e.parentContext)
	e.subscriptions[subID] = &subscription{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
		quit:                   quit,
	}

	go e.startInterval(subctx, request.NamespaceId, request.PipelineId, request.PipelineExtensionLabel, duration)

	log.Debug().Str("namespace_id", request.NamespaceId).Str("extension_label", request.PipelineExtensionLabel).
		Str("pipeline_id", request.PipelineId).Msg("subscribed pipeline")
	return &proto.ExtensionSubscribeResponse{}, nil
}

// Pipelines change and this means that sometimes they will no longer want to be executed by a particular extension or maybe
// they want to change the previous settings on that extension. Because of this we need a way to remove pipelines that were
// previously subscribed.
func (e *extension) Unsubscribe(ctx context.Context, request *proto.ExtensionUnsubscribeRequest) (*proto.ExtensionUnsubscribeResponse, error) {
	subscription, exists := e.subscriptions[subscriptionID{
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
	delete(e.subscriptions, subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineExtensionLabel,
	})
	return &proto.ExtensionUnsubscribeResponse{}, nil
}

// Info is mostly used as a health check endpoint. It returns some basic info about a extension, the most important
// being where to get more documentation about that specific extension.
func (e *extension) Info(ctx context.Context, request *proto.ExtensionInfoRequest) (*proto.ExtensionInfoResponse, error) {
	registered := []string{}
	for _, sub := range e.subscriptions {
		registered = append(registered, fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline))
	}

	return &proto.ExtensionInfoResponse{
		Name:          e.systemConfig.Name,
		Documentation: "https://clintjedwards.com/gofer/ref/extensions/provided/interval.html",
		Registered:    registered,
	}, nil
}

// The ExternalEvent endpoint tells the extension what to do if they get messages from Gofer's external event system.
// This system is set up to facilitate webhook interactions like those that occur for github
// (A user pushes a branch, Gofer gets an event from github).
// The ExternalEvent will come with a payload which the extension can then authenticate, process, and take action on.
func (e *extension) ExternalEvent(ctx context.Context, request *proto.ExtensionExternalEventRequest) (*proto.ExtensionExternalEventResponse, error) {
	return &proto.ExtensionExternalEventResponse{}, nil
}

// A graceful shutdown for a extension should clean up any resources it was working with that might be left hanging.
// Sometimes that means sending requests to third parties that it is shutting down, sometimes that just means
// reaping its personal goroutines.
func (e *extension) Shutdown(ctx context.Context, request *proto.ExtensionShutdownRequest) (*proto.ExtensionShutdownResponse, error) {
	if e.isInitialized {
		e.quitAllSubscriptions()
	}

	return &proto.ExtensionShutdownResponse{}, nil
}

// The ExtensionInstaller is a small script that gets piped to the admin who is trying to set up this particular
// extension. The installer is meant to guide the user through the different configuration options that the
// installer has globally.
func (e *extension) RunExtensionInstaller(stream proto.ExtensionService_RunExtensionInstallerServer) error {
	err := sdk.SendInstallerMessageToClient(stream, "The interval extension allows users to run their pipelines on the passage of "+
		"time by setting a particular duration.\n")
	if err != nil {
		return err
	}

	err = sdk.SendInstallerMessageToClient(stream, "First, let's prevent users from setting too low of an interval by "+
		"setting a minimum duration. Durations are set via Golang duration strings. For example, entering a duration "+
		"of '10h' would be 10 hours, meaning that users can only run their pipeline at most every 10 hours. "+
		"You can find more documentation on valid strings here: https://pkg.go.dev/time#ParseDuration.")
	if err != nil {
		return err
	}

	for {
		err = sdk.SendInstallerQueryToClient(stream, "Set a minimum duration for all pipelines: ")
		if err != nil {
			return err
		}

		clientMsg, err := stream.Recv()
		if err != nil {
			return err
		}

		_, err = time.ParseDuration(clientMsg.Msg)
		if err != nil {
			err = sdk.SendInstallerMessageToClient(stream, fmt.Sprintf("Malformed duration %q; %v", clientMsg.Msg, err))
			if err != nil {
				return err
			}
			continue
		}

		err = sdk.SendInstallerConfigSettingToClient(stream, ConfigMinInterval, clientMsg.Msg)
		if err != nil {
			return err
		}

		break
	}

	err = sdk.SendInstallerMessageToClient(stream, "Interval extension configuration finished")
	if err != nil {
		return err
	}

	return nil
}

// The PipelineConfigurator is a small script that a pipeline owner can run when subscribing to this extension.
// It's meant to guide the pipeline owner through the different options of the extension.
func (e *extension) RunPipelineConfigurator(stream proto.ExtensionService_RunPipelineConfiguratorServer) error {
	err := sdk.SendConfiguratorMessageToClient(stream, "The interval extension allows you to run your pipelines on the passage of "+
		"time by setting a particular duration.\n")
	if err != nil {
		return err
	}

	err = sdk.SendConfiguratorMessageToClient(stream, "Durations are set via Golang duration strings. "+
		"For example, entering a duration of '10h' would be 10 hours, meaning that your pipeline would run once every 10 hours. "+
		"You can find more documentation on valid strings here: https://pkg.go.dev/time#ParseDuration.")
	if err != nil {
		return err
	}

	for {
		err = sdk.SendConfiguratorQueryToClient(stream, "Set your pipeline run interval: ")
		if err != nil {
			return err
		}

		clientMsg, err := stream.Recv()
		if err != nil {
			return err
		}

		_, err = time.ParseDuration(clientMsg.Msg)
		if err != nil {
			err = sdk.SendConfiguratorMessageToClient(stream, fmt.Sprintf("Malformed duration %q; %v", clientMsg.Msg, err))
			if err != nil {
				return err
			}
			continue
		}

		err = sdk.SendConfiguratorParamSettingToClient(stream, ParameterEvery, clientMsg.Msg)
		if err != nil {
			return err
		}

		break
	}

	err = sdk.SendConfiguratorMessageToClient(stream, "Interval extension configuration finished")
	if err != nil {
		return err
	}

	return nil
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
	extension := extension{}
	sdk.NewExtension(&extension)
}
