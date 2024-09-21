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
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	// The two sdk packages contains a bunch of convenience functions that we use to build our extension.
	// It is possible to build a extension without using the SDK, but the SDK makes the process much
	// less cumbersome. The sdk package contains assets for communicating with the main Gofer API, the
	// extension sdk (or extsdk for short) contains assets for building an extension.
	sdk "github.com/clintjedwards/gofer/sdk/go"
	extsdk "github.com/clintjedwards/gofer/sdk/go/extensions"

	// Golang doesn't have a standardized logging interface* and as such Gofer extensions can technically
	// use any logging package, but because Gofer and provided extensions use zerolog, it is heavily encouraged
	// to use zerolog. The log level for extensions is set by Gofer on extension start via Gofer's configuration.
	// And logs are interleaved in the stdout for the main program.
	//
	// * Golang now does have a log interface! But I haven't had a chance to implement it yet.
	"github.com/rs/zerolog/log"
)

// Extensions have two types of variables they can be passed.
//   - They take variables called "config" when they are first installed. This config gets passed on every startup.
//   - They take variables called "parameters" these are passed by the pipeline when subscribing.

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
	systemConfig extsdk.ExtensionSystemConfig
}

func newExtension() *extension {
	config, err := extsdk.GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not parse system configuration")
	}

	// When the extension starts Gofer injects the config settings into the environment under the prefix
	// `GOFER_EXTENSION_CONFIG_`. First we retrieve those config values so that we can store them for later use.
	minDurationStr := extsdk.GetConfigFromEnv(ConfigMinInterval)
	minDuration := time.Minute * 1
	if minDurationStr != "" {
		parsedDuration, err := time.ParseDuration(minDurationStr)
		if err != nil {
			log.Fatal().Err(err).Str("value", minDurationStr).Msg("could not parse min_duration given")
		}
		minDuration = parsedDuration
	}

	// Since this extension spins up go-routines we define a way that it can quickly shut them all down.
	parentContext, quitAllSubscriptions := context.WithCancel(context.Background())

	extension := &extension{
		minInterval:          minDuration,
		parentContext:        parentContext,
		quitAllSubscriptions: quitAllSubscriptions,
		subscriptions:        map[subscriptionID]*subscription{},
		systemConfig:         config,
	}

	subscriptions, err := sdk.ListExtensionSubscriptions(config.ID, config.GoferHost, config.Secret, config.UseTLS, sdk.GoferAPIVersion0)
	if err != nil {
		log.Fatal().Err(err).Msg("Could not query subscriptions from Gofer host")
	}

	// TODO: Eventually we should make this more intelligent to prevent thundering herd problems. But for right now
	// this should suffice.
	for _, subscription := range subscriptions {
		// We just call the internal subscribe function here since it does all the validation we'd have to redo either
		// way.
		err := extension.Subscribe(context.Background(), extsdk.SubscriptionRequest{
			NamespaceId:                subscription.NamespaceId,
			PipelineId:                 subscription.PipelineId,
			PipelineSubscriptionId:     subscription.SubscriptionId,
			PipelineSubscriptionParams: subscription.Settings,
		})
		if err != nil {
			log.Fatal().Str("err", err.Message).Msg("Could not restore subscription")
		}
	}

	return extension
}

// startInterval is the main logic of what enables the interval extension to work. Each pipeline that is subscribed runs
// this function which simply waits for the set duration and then calls the StartRun endpoint for Gofer.
func (e *extension) startInterval(ctx context.Context,
	namespace, pipeline string, pipelineExtensionLabel string, duration time.Duration,
) {
	log := log.With().Str("namespace_id", namespace).Str("pipeline_id", pipeline).
		Str("pipeline_subscription_id", pipelineExtensionLabel).Logger()

	client, err := sdk.NewClientWithHeaders(e.systemConfig.GoferHost, e.systemConfig.Secret, e.systemConfig.UseTLS, sdk.GoferAPIVersion0)
	if err != nil {
		log.Error().Err(err).Msg("Could not establish Gofer client")
	}

	for {
		select {
		case <-ctx.Done():
			return
		case <-time.After(duration):
			resp, err := client.StartRun(ctx, namespace, pipeline, sdk.StartRunRequest{
				Variables: map[string]string{},
			})
			if err != nil {
				log.Error().Err(err).Msg("could not start new run")
				continue
			}
			defer resp.Body.Close()

			body, err := io.ReadAll(resp.Body)
			if err != nil {
				log.Error().Err(err).Msg("could not read response body while attempting to start run")
				continue
			}

			if resp.StatusCode < 200 || resp.StatusCode > 299 {
				log.Error().Bytes("message", body).Int("status_code", resp.StatusCode).Msg("could not start new run; received non 2xx status code")
				continue
			}

			startRunResponse := sdk.StartRunResponse{}
			if err := json.Unmarshal(body, &startRunResponse); err != nil {
				log.Error().Err(err).Msg("could not parse response body while attempting to read start run response")
				continue
			}

			log.Debug().Int64("run_id", int64(startRunResponse.Run.RunId)).
				Msg("new tick for specified interval; new event spawned")
		}
	}
}

// A simple healthcheck endpoint used by Gofer to make sure the extension is still in good health and reachable.
func (e *extension) Health(_ context.Context) *extsdk.HttpError {
	return nil
}

// Gofer calls subscribe when a pipeline wants to subscribe to this extension.
// The logic here is simple:
//   - Retrieve the pipeline's requested parameters.
//   - Validate the parameters.
//   - Create a new subscription object and enter it into our map.
//   - Call the `startInterval` function in a goroutine for that specific pipeline and return.
func (e *extension) Subscribe(_ context.Context, request extsdk.SubscriptionRequest) *extsdk.HttpError {
	interval, exists := request.PipelineSubscriptionParams[ParameterEvery]
	if !exists {
		return &extsdk.HttpError{
			Message: fmt.Sprintf("could not find required pipeline subscription parameter %q; received params: %+v",
				ParameterEvery, request.PipelineSubscriptionParams),
			StatusCode: http.StatusBadRequest,
		}
	}

	duration, err := time.ParseDuration(interval)
	if err != nil {
		return &extsdk.HttpError{
			Message:    fmt.Sprintf("could not parse interval string: %v", err),
			StatusCode: http.StatusBadRequest,
		}
	}

	if duration < e.minInterval {
		return &extsdk.HttpError{
			Message:    fmt.Sprintf("durations cannot be less than %s", e.minInterval),
			StatusCode: http.StatusBadRequest,
		}
	}

	subID := subscriptionID{
		request.NamespaceId,
		request.PipelineId,
		request.PipelineSubscriptionId,
	}

	log := log.With().Str("namespace_id", request.NamespaceId).Str("pipeline_id", request.PipelineId).
		Str("pipeline_subscription_id", request.PipelineSubscriptionId).Logger()

	// It is perfectly possible for Gofer to attempt to subscribe an already subscribed pipeline. In this case,
	// we can simply ignore the request.
	_, exists = e.subscriptions[subID]
	if exists {
		log.Debug().Msg("pipeline already subscribed; ignoring request")
		return nil
	}

	subctx, quit := context.WithCancel(e.parentContext)
	e.subscriptions[subID] = &subscription{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineSubscriptionId,
		quit:                   quit,
	}

	go e.startInterval(subctx, request.NamespaceId, request.PipelineId, request.PipelineSubscriptionId, duration)

	log.Debug().Msg("subscribed pipeline")
	return nil
}

// Pipelines change and this means that sometimes they will no longer want to be executed by a particular extension or maybe
// they want to change the previous settings on that extension. Because of this we need a way to remove pipelines that were
// previously subscribed.
func (e *extension) Unsubscribe(_ context.Context, request extsdk.UnsubscriptionRequest) *extsdk.HttpError {
	subscription, exists := e.subscriptions[subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineSubscriptionId,
	}]
	// It is perfectly possible for Gofer to attempt to unsubscribe an already unsubscribed pipeline. In this case,
	// we can simply ignore the request.
	if !exists {
		return nil
	}

	subscription.quit()
	delete(e.subscriptions, subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineSubscriptionId,
	})
	return nil
}

// Info is mostly used as a health check endpoint. It returns some basic info about a extension, the most important
// being where to get more documentation about that specific extension.
func (e *extension) Info(_ context.Context) (*extsdk.InfoResponse, *extsdk.HttpError) {
	return &extsdk.InfoResponse{
		ExtensionId: "", // The extension wrapper automagically fills this in.
		Documentation: extsdk.Documentation{
			Body: "You can find more information on this extension at the official Gofer docs site: https://clintjedwards.com/gofer/ref/extensions/provided/interval.html",
			PipelineSubscriptionParams: []extsdk.Parameter{
				{
					Key:           ParameterEvery,
					Documentation: "'every' is the time between pipeline runs. Supports golang native duration strings: https://pkg.go.dev/time#ParseDuration. Examples: '1m', '60s', '3h', '3m30s'",
					Required:      true,
				},
			},
			ConfigParams: []extsdk.Parameter{
				{
					Key:           ConfigMinInterval,
					Documentation: "The minimum interval pipelines can set for the 'every' parameter. Supports golang native duration strings: https://pkg.go.dev/time#ParseDuration. Examples: '1m', '60s', '3h', '3m30s'. Defaults to 1 minute.",
					Required:      false,
				},
			},
		},
	}, nil
}

// The ExternalEvent endpoint tells the extension what to do if they get messages from Gofer's external event system.
// This system is set up to facilitate webhook interactions like those that occur for github
// (A user pushes a branch, Gofer gets an event from github).
// The ExternalEvent will come with a payload which the extension can then authenticate, process, and take action on.
func (e *extension) ExternalEvent(_ context.Context, _ extsdk.ExternalEventRequest) *extsdk.HttpError {
	return nil
}

// A graceful shutdown for a extension should clean up any resources it was working with that might be left hanging.
// Sometimes that means sending requests to third parties that it is shutting down, sometimes that just means
// reaping its personal goroutines.
func (e *extension) Shutdown(_ context.Context) {
	e.quitAllSubscriptions()
}

func (e *extension) Debug(_ context.Context) extsdk.DebugResponse {
	registered := []string{}
	for _, sub := range e.subscriptions {
		registered = append(registered, fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline))
	}

	config, _ := extsdk.GetExtensionSystemConfig()

	debug := struct {
		RegisteredPipelines []string `json:"registered_pipelines"`
		Config              extsdk.ExtensionSystemConfig
	}{
		RegisteredPipelines: registered,
		Config:              config,
	}

	data, jsonErr := json.Marshal(debug)
	if jsonErr != nil {
		log.Error().Err(jsonErr).Msg("Could not serialize response for debug endpoint")
	}

	return extsdk.DebugResponse{
		Info: string(data),
	}
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
	extension := newExtension()
	extsdk.NewExtension(extension)
}
