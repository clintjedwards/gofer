package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"sync"
	"time"

	"github.com/clintjedwards/avail/v2"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	extsdk "github.com/clintjedwards/gofer/sdk/go/extensions"
	"github.com/rs/zerolog/log"
)

// ParameterExpression is the cron expression for pipeline scheduling.
const ParameterExpression = "expression"

// maxCatchup is how far behind the scheduler is allowed to fall before it gives up on replaying missed minutes.
//
// Falling a little behind is normal: the host was under load, the process got paused for a bit, or NTP nudged the
// clock forward. In those cases we want to go back and fire whatever we missed, since a run that's a minute or two
// late is better than one that never happens.
//
// Falling a lot behind is different. If the machine is offline for six hours or someone fixed a clock that was a day
// off, replaying every missed minute would dump a huge burst of runs on Gofer all at once, most of which nobody wants
// anymore. Past this limit we just evaluate the current minute and move on.
const maxCatchup = 5 * time.Minute

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
	mu            sync.Mutex
	subscriptions map[subscriptionID]*subscription
}

func newExtension() *extension {
	extension := &extension{
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

	go extension.run()

	return extension
}

func (e *extension) Health(_ context.Context) *extsdk.HttpError {
	return nil
}

var documentation = extsdk.Documentation{
	Body: "You can find more information on this extension at the official Gofer docs site: https://gofer.clintjedwards.com/docs/ref/extensions/provided/cron.html",
	PipelineSubscriptionParams: []extsdk.Parameter{
		{
			Key:           ParameterExpression,
			Documentation: "The cron expression to run on. You can find more information on crafting this expression at https://gofer.clintjedwards.com/docs/ref/extensions/provided/cron.html",
			Required:      true,
		},
	},
	ConfigParams: []extsdk.Parameter{},
}

func (e *extension) Debug(_ context.Context) extsdk.DebugResponse {
	registered := []string{}
	e.mu.Lock()
	for _, sub := range e.subscriptions {
		registered = append(registered, fmt.Sprintf("%s/%s", sub.namespace, sub.pipeline))
	}
	e.mu.Unlock()

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

	e.mu.Lock()
	defer e.mu.Unlock()

	// It is perfectly possible for Gofer to attempt to subscribe an already subscribed pipeline with the same ID.
	// This is definitely a mistake so we simply ignore the request.
	_, exists = e.subscriptions[subID]
	if exists {
		log.Debug().Str("namespace_id", request.NamespaceId).Str("pipeline_subscription_id", request.PipelineSubscriptionId).
			Str("pipeline_id", request.PipelineId).Msg("pipeline already subscribed; ignoring request")
		return nil
	}

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

	e.mu.Lock()
	delete(e.subscriptions, subID)
	e.mu.Unlock()

	log.Debug().Str("pipeline_subscription_id", request.PipelineSubscriptionId).Str("pipeline_id", request.PipelineId).
		Str("namespace_id", request.NamespaceId).Msg("unsubscribed pipeline")

	return nil
}

func (e *extension) Shutdown(_ context.Context) {}

func (e *extension) ExternalEvent(_ context.Context, _ extsdk.ExternalEventRequest) *extsdk.HttpError {
	// We don't support external events.
	return nil
}

// run is the scheduler loop.
//
// The naive way to write this is "sleep a minute, check what's due, repeat". That has a few problems:
//
//   - The check happens at whatever second the process started on, so a job scheduled for 01:00 might fire at
//     01:00:37.
//   - The sleep starts after the check finishes, so every loop drifts a little later. Eventually the check slides past
//     a minute boundary and that minute is never looked at. Any job scheduled for it silently doesn't run.
//   - If the wall clock gets moved backwards, the same minute can be looked at twice and jobs run twice.
//
// Instead we keep a cursor, last, which is the most recent minute we've fully handled. It's always truncated to the
// minute (seconds and below zeroed out), so it's a label for a minute rather than a precise instant. Each time we
// wake up we ask minutesToCheck for every minute after the cursor up to now, handle each one, and move the cursor
// forward. Because the cursor only ever moves forward one handled minute at a time, every minute gets evaluated
// exactly once no matter how late we wake up or what the clock does.
//
// The cursor lives only in memory. On startup it begins at the current minute, which means:
//
//   - We never re-fire the minute we started in, even if a previous instance already fired it before restarting.
//   - We don't replay minutes that passed while the extension was down. That's a deliberate choice; we have no record
//     of what the last instance already did, so guessing would risk duplicate runs.
//
// A subscription added partway through a minute isn't evaluated for that minute since the cursor has already moved
// past it from the scheduler's point of view, so its first run is its next matching minute.
func (e *extension) run() {
	last := time.Now().Truncate(time.Minute)
	for {
		// Sleep until the start of the minute after the cursor. Truncate strips Go's monotonic clock reading from
		// last, so time.Until compares wall clock times, which is what cron expressions are about.
		//
		// Sleeping isn't perfectly precise, and the wall clock can be adjusted while we're asleep, so we might wake a
		// hair early or quite late. Neither is a problem: if we're early minutesToCheck returns nothing and we loop
		// back around to sleep the last few milliseconds, and if we're late it hands us every minute we missed.
		//
		// If the clock was moved backwards, last.Add(time.Minute) can be well in the future. That's intended: we
		// wait for the clock to catch back up to minutes we haven't handled yet rather than redoing ones we have.
		time.Sleep(time.Until(last.Add(time.Minute)))

		// Advance the cursor after each minute rather than jumping straight to now, so it always reflects exactly
		// what's been handled.
		for _, minute := range minutesToCheck(last, time.Now()) {
			e.checkTimeFrames(minute)
			last = minute
		}
	}
}

// minutesToCheck works out which minutes the scheduler still owes an evaluation for, given the cursor (last) and the
// current time. It's kept separate from run so the decision logic can be tested without sleeping or a real clock.
//
//   - now is in the same minute as last, or earlier (the clock went backwards): nothing is owed, return nil.
//   - now is a few minutes past last (a normal tick, or a short stall): return each minute after last, oldest
//     first, up to and including now's minute.
//   - now is more than maxCatchup past last (a long stall or a big clock jump): skip the backlog and return only
//     now's minute. See maxCatchup for why.
func minutesToCheck(last, now time.Time) []time.Time {
	now = now.Truncate(time.Minute)
	if !now.After(last) {
		return nil
	}

	if now.Sub(last) > maxCatchup {
		return []time.Time{now}
	}

	minutes := []time.Time{}
	for m := last.Add(time.Minute); !m.After(now); m = m.Add(time.Minute) {
		minutes = append(minutes, m)
	}
	return minutes
}

// checkTimeFrames starts a run for every subscription whose expression matches the given minute. Note that it
// evaluates the minute it's handed, not time.Now(); when catching up, that minute may be in the past and we still
// want to know what was scheduled for it.
func (e *extension) checkTimeFrames(minute time.Time) {
	// Copy out what's due while holding the lock, then release it before making any network calls. Holding it
	// across slow StartRun requests would block Subscribe and Unsubscribe calls from Gofer until they finished.
	e.mu.Lock()
	due := []subscription{}
	for _, sub := range e.subscriptions {
		if sub.timeframe.Able(minute) {
			due = append(due, *sub)
		}
	}
	e.mu.Unlock()

	if len(due) == 0 {
		return
	}

	config, _ := extsdk.GetExtensionSystemConfig()

	client, err := sdk.NewClientWithHeaders(config.GoferHost, config.Secret, config.UseTLS, sdk.GoferAPIVersion0)
	if err != nil {
		log.Fatal().Err(err).Msg("Could not initialize client while attempting to check time frames")
	}

	// Each run is started in its own goroutine so one slow request doesn't delay the rest. It also means the
	// scheduler loop gets back to sleeping right away, so a slow Gofer can't push us past the next minute boundary.
	for _, sub := range due {
		go startRun(client, sub, minute)
	}
}

func startRun(client *sdk.Client, sub subscription, minute time.Time) {
	log := log.With().Str("namespace_id", sub.namespace).Str("pipeline_id", sub.pipeline).
		Str("pipeline_subscription_id", sub.pipelineExtensionLabel).Time("scheduled_for", minute).Logger()

	resp, err := client.StartRun(context.Background(), sub.namespace, sub.pipeline, sdk.StartRunRequest{
		Variables: map[string]string{},
	})
	if err != nil {
		log.Error().Err(err).Msg("could not start new run")
		return
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		log.Error().Err(err).Msg("could not read response body while attempting to start run")
		return
	}

	if resp.StatusCode < 200 || resp.StatusCode > 299 {
		log.Error().Bytes("message", body).Int("status_code", resp.StatusCode).
			Msg("could not start new run; received non 2xx status code")
		return
	}

	startRunResponse := sdk.StartRunResponse{}
	if err := json.Unmarshal(body, &startRunResponse); err != nil {
		log.Error().Err(err).Msg("could not parse response body while attempting to read start run response")
		return
	}

	log.Debug().Int64("run_id", int64(startRunResponse.Run.RunId)).Msg("Pipeline within timeframe; new event spawned")
}

func main() {
	extsdk.Run(documentation, func() extsdk.ExtensionServiceInterface { return newExtension() })
}
