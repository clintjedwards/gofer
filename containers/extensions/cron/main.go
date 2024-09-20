package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
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
	subscriptions map[subscriptionID]*subscription
}

func newExtension() extension {
	extension := extension{
		subscriptions: map[subscriptionID]*subscription{},
	}

	config, err := extsdk.GetExtensionSystemConfig()
	if err != nil {
		log.Fatal().Err(err).Msg("could not parse system configuration")
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

	go func() {
		for {
			time.Sleep(checkInterval)
			extension.checkTimeFrames()
		}
	}()

	return extension
}

func (e *extension) Health(_ context.Context) *extsdk.HttpError {
	return nil
}

func (e *extension) Info(_ context.Context) (*extsdk.InfoResponse, *extsdk.HttpError) {
	return &extsdk.InfoResponse{
		ExtensionId: "", // The extension wrapper automagically fills this in.
		Documentation: extsdk.Documentation{
			Body: "You can find more information on this extension at the official Gofer docs site: https://clintjedwards.com/gofer/ref/extensions/provided/cron.html",
			PipelineSubscriptionParams: []extsdk.Parameter{
				{
					Key:           ParameterExpression,
					Documentation: "The cron expression to run on. You can find more information on crafting this expression at https://clintjedwards.com/gofer/ref/extensions/provided/cron.html",
					Required:      true,
				},
			},
			ConfigParams: []extsdk.Parameter{},
		},
	}, nil
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

func (e *extension) Subscribe(_ context.Context, request extsdk.SubscriptionRequest) *extsdk.HttpError {
	expression, exists := request.PipelineSubscriptionParams[ParameterExpression]
	if !exists {
		return &extsdk.HttpError{
			StatusCode: http.StatusBadRequest,
			Message:    fmt.Sprintf("Required parameter %q missing", ParameterExpression),
		}
	}

	timeframe, err := avail.New(expression)
	if err != nil {
		return &extsdk.HttpError{
			StatusCode: http.StatusBadRequest,
			Message:    fmt.Sprintf("Could not parse expression: %q; %v", expression, err),
		}
	}

	subID := subscriptionID{
		request.NamespaceId,
		request.PipelineId,
		request.PipelineSubscriptionId,
	}

	// It is perfectly possible for Gofer to attempt to subscribe an already subscribed pipeline. In this case,
	// we can simply ignore the request.
	_, exists = e.subscriptions[subID]
	if exists {
		log.Debug().Str("namespace_id", request.NamespaceId).Str("pipeline_subscription_id", request.PipelineSubscriptionId).
			Str("pipeline_id", request.PipelineId).Msg("pipeline already subscribed; ignoring request")
		return nil
	}

	// While it might result in a faster check to start a goroutine for each subscription the interval
	// for most of these expressions should be on the order of minutes. So one event loop checking the
	// result for all of them should still result in no missed checks.
	e.subscriptions[subID] = &subscription{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineSubscriptionId,
		timeframe:              timeframe,
	}

	log.Debug().Str("pipeline_subscription_id", request.PipelineSubscriptionId).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("subscribed pipeline")
	return nil
}

func (e *extension) Unsubscribe(_ context.Context, request extsdk.UnsubscriptionRequest) *extsdk.HttpError {
	subID := subscriptionID{
		namespace:              request.NamespaceId,
		pipeline:               request.PipelineId,
		pipelineExtensionLabel: request.PipelineSubscriptionId,
	}

	delete(e.subscriptions, subID)

	log.Debug().Str("pipeline_subscription_id", request.PipelineSubscriptionId).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")
	return nil
}

func (e *extension) Shutdown(_ context.Context) {}

func (e *extension) ExternalEvent(_ context.Context, _ extsdk.ExternalEventRequest) *extsdk.HttpError {
	// We don't support external events.
	return nil
}

func (e *extension) checkTimeFrames() {
	config, _ := extsdk.GetExtensionSystemConfig()

	client, err := sdk.NewClientWithHeaders(config.GoferHost, config.Secret, config.UseTLS, sdk.GoferAPIVersion0)
	if err != nil {
		log.Fatal().Err(err).Msg("Could not initialize client while attempting to check time frames")
	}

	for _, subscription := range e.subscriptions {
		if subscription.timeframe.Able(time.Now()) {
			log := log.With().Str("namespace_id", subscription.namespace).Str("pipeline_id", subscription.pipeline).
				Str("pipeline_subscription_id", subscription.pipelineExtensionLabel).Logger()

			resp, err := client.StartRun(context.Background(), subscription.namespace, subscription.pipeline, sdk.StartRunRequest{
				Variables: map[string]string{},
			})
			if err != nil {
				log.Error().Err(err).Msg("could not start new run")
				continue
			}
			defer resp.Body.Close()

			if resp.StatusCode < 200 || resp.StatusCode > 299 {
				log.Error().Int("status_code", resp.StatusCode).Msg("could not start new run; received non 2xx status code")
				continue
			}

			body, err := io.ReadAll(resp.Body)
			if err != nil {
				log.Error().Err(err).Msg("could not read response body while attempting to start run")
				continue
			}

			startRunResponse := sdk.StartRunResponse{}
			if err := json.Unmarshal(body, &startRunResponse); err != nil {
				log.Error().Err(err).Msg("could not parse response body while attempting to read start run response")
				continue
			}

			log.Debug().Int64("run_id", int64(startRunResponse.Run.RunId)).Msg("Pipeline within timeframe; new event spawned")
		}
	}
}

func main() {
	extension := newExtension()
	extsdk.NewExtension(&extension)
}
