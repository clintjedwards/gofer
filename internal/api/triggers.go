package api

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

const TRIGGERCONTAINERIDFORMAT = "trigger_%s" // trigger_<triggerKind>

// startTriggers attempts to start each trigger from config on the provided scheduler. Once scheduled it then collects
// the initial trigger information so it can check for connectivity and store the network location.
// This information will eventually be used in other parts of the API to communicate with said triggers.
func (api *API) startTriggers() error {
	cert, key, err := api.getTLSFromFile(api.config.Triggers.TLSCertPath, api.config.Triggers.TLSKeyPath)
	if err != nil {
		return err
	}

	triggerKey := generateToken(32)

	for _, trigger := range api.config.Triggers.RegisteredTriggers {
		// We need to first populate the triggers with their required environment variables.
		// Order is important here maps later in the list will overwrite earlier maps.
		// We first include the Gofer defined environment variables and then the operator configured environment
		// variables.
		envVars := mergeMaps(map[string]string{
			"GOFER_TRIGGER_TLS_CERT":  string(cert),
			"GOFER_TRIGGER_TLS_KEY":   string(key),
			"GOFER_TRIGGER_KIND":      trigger.Kind,
			"GOFER_TRIGGER_LOG_LEVEL": api.config.LogLevel,
			"GOFER_TRIGGER_KEY":       triggerKey,
		}, trigger.EnvVars)

		log.Info().Str("name", trigger.Kind).Msg("starting trigger")
		sc := scheduler.StartContainerRequest{
			ID:               fmt.Sprintf(TRIGGERCONTAINERIDFORMAT, trigger.Kind),
			ImageName:        trigger.Image,
			EnvVars:          envVars,
			RegistryUser:     trigger.User,
			RegistryPass:     trigger.Pass,
			EnableNetworking: true,
		}

		resp, err := api.scheduler.StartContainer(sc)
		if err != nil {
			log.Error().Err(err).Str("trigger", trigger.Kind).Msg("could not start trigger")
			return err
		}

		var info *sdkProto.InfoResponse

		// For some reason I can't get GRPC's retry to properly handle this, so instead we resort to a simple for loop.
		//
		// There is a race condition where we schedule the container, but the actual container application might not
		// have gotten a chance to start before we issue a query.
		// So instead of baking in some arbitrary sleep time between these two actions instead we retry
		// until we get a good state.
		attempts := 0
		for {
			if attempts >= 30 {
				log.Error().Msg("maximum amount of attempts reached for starting trigger; could not connect to trigger")
				return fmt.Errorf("could not connect to trigger; maximum amount of attempts reached")
			}

			conn, err := grpcDial(resp.URL)
			if err != nil {
				log.Debug().Err(err).Str("trigger", trigger.Kind).Msg("could not connect to trigger")
				time.Sleep(time.Millisecond * 300)
				attempts++
				continue
			}

			client := sdkProto.NewTriggerClient(conn)

			ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(triggerKey))

			info, err = client.Info(ctx, &sdkProto.InfoRequest{})
			if err != nil {
				if status.Code(err) == codes.Canceled {
					return nil
				}

				conn.Close()
				log.Debug().Err(err).Msg("failed to communicate with trigger startup")
				time.Sleep(time.Millisecond * 300)
				attempts++
				continue
			}

			conn.Close()
			break
		}

		// Add the trigger to the in-memory registry so we can refer to its variable network location later.
		api.triggers[trigger.Kind] = &models.Trigger{
			Key:           triggerKey,
			Kind:          trigger.Kind,
			URL:           resp.URL,
			SchedulerID:   resp.SchedulerID,
			Started:       time.Now().UnixMilli(),
			State:         models.ContainerStateRunning,
			Documentation: info.Documentation,
		}

		log.Info().
			Str("kind", trigger.Kind).
			Str("id", resp.SchedulerID).
			Str("url", resp.URL).Msg("started trigger")

		go api.collectLogs(resp.SchedulerID)
	}

	go api.monitorTriggers(api.context.ctx)

	return nil
}

// monitorTriggers periodically healthchecks registered Gofer triggers and updates their status.
// This allows Gofer to not only report the connectivity between the service and the triggers
// but alert admins when triggers aren't connected properly.
func (api *API) monitorTriggers(ctx context.Context) {
	// Since we'll be constantly pinging the triggers create a pool of connections that we can reuse.
	connectionPool := map[string]*grpc.ClientConn{}
	for _, trigger := range api.triggers {
		conn, err := grpcDial(trigger.URL)
		if err != nil {
			log.Debug().Str("trigger", trigger.Kind).Err(err).Msg("healthcheck failed; could not connect to trigger")
			api.triggers[trigger.Kind].State = models.ContainerStateFailed
			continue
		}
		connectionPool[trigger.Kind] = conn
	}

	for {
		select {
		case <-ctx.Done():
			log.Debug().Msg("cleaning up healthcheck connections")
			for _, conn := range connectionPool {
				conn.Close()
			}
			return
		case <-time.After(api.config.Triggers.HealthcheckInterval):
			for triggerKind, conn := range connectionPool {
				client := sdkProto.NewTriggerClient(conn)

				ctx := metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+string(api.triggers[triggerKind].Key))
				_, err := client.Info(ctx, &sdkProto.InfoRequest{})
				if err != nil {
					if status.Code(err) == codes.Canceled {
						return
					}

					api.triggers[triggerKind].State = models.ContainerStateFailed
					log.Debug().Err(err).Msg("healthcheck failed")
					continue
				}

				api.triggers[triggerKind].State = models.ContainerStateRunning
			}
		}
	}
}

// stopTriggers sends a shutdown request to each trigger, initiating a graceful shutdown for each one.
func (api *API) stopTriggers() {
	for _, trigger := range api.triggers {
		conn, err := grpcDial(trigger.URL)
		if err != nil {
			continue
		}
		defer conn.Close()

		client := sdkProto.NewTriggerClient(conn)

		ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(trigger.Key))
		_, err = client.Shutdown(ctx, &sdkProto.ShutdownRequest{})
		if err != nil {
			continue
		}

	}
}

// restoreTriggerSubscriptions iterates through all pipelines and subscribes them all back to their defined triggers.
// We need to do this because of the fact that triggers are stateless and ephemeral and the only way they even know
// of the existence of pipelines is through the "subscribe" function.
func (api *API) restoreTriggerSubscriptions() error {
	pipelines, err := api.collectAllPipelines()
	if err != nil {
		return fmt.Errorf("could not restore trigger subscriptions; %w", err)
	}

	for _, pipeline := range pipelines {
		for label, subscription := range pipeline.Triggers {
			trigger, exists := api.triggers[subscription.Kind]
			if !exists {
				pipeline.State = models.PipelineStateDisabled
				subscription.State = models.PipelineTriggerStateDisabled
				pipeline.Triggers[subscription.Label] = subscription
				storageErr := api.storage.UpdatePipeline(storage.UpdatePipelineRequest{
					Pipeline: pipeline,
				})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("trigger_label", subscription.Label).Str("trigger_kind", subscription.Kind).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; trigger requested does not exist within Gofer service")
				continue
			}

			conn, err := grpcDial(trigger.URL)
			if err != nil {
				return fmt.Errorf("could not subscribe trigger %q for pipeline %q - namespace %q; %w",
					subscription.Label, pipeline.ID, pipeline.Namespace, err)
			}
			defer conn.Close()

			client := sdkProto.NewTriggerClient(conn)

			config, err := api.populateSecrets(pipeline.Namespace, pipeline.ID, subscription.Config)
			if err != nil {
				pipeline.State = models.PipelineStateDisabled
				subscription.State = models.PipelineTriggerStateDisabled
				pipeline.Triggers[subscription.Label] = subscription
				storageErr := api.storage.UpdatePipeline(storage.UpdatePipelineRequest{
					Pipeline: pipeline,
				})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("trigger_label", subscription.Label).Str("trigger_kind", subscription.Kind).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; could not find appropriate secrets")
				continue
			}

			ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(trigger.Key))
			_, err = client.Subscribe(ctx, &sdkProto.SubscribeRequest{
				NamespaceId:          pipeline.Namespace,
				PipelineTriggerLabel: label,
				PipelineId:           pipeline.ID,
				Config:               config,
			})
			if err != nil {
				pipeline.State = models.PipelineStateDisabled
				subscription.State = models.PipelineTriggerStateDisabled
				pipeline.Triggers[subscription.Label] = subscription
				storageErr := api.storage.UpdatePipeline(storage.UpdatePipelineRequest{
					Pipeline: pipeline,
				})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("trigger_label", subscription.Label).Str("trigger_kind", subscription.Kind).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; error contacting trigger")
				continue
			}

			log.Debug().Str("pipeline", pipeline.ID).Str("trigger_label", subscription.Label).
				Str("trigger_kind", trigger.Kind).Msg("restored subscription")
		}
	}

	return nil
}

// checkForTriggerEvents spawns a goroutine for every trigger that is responsible for collecting the trigger events
// on that trigger. The "Check" method for receiving events from a trigger is a blocking RPC, so each
// go routine essentially blocks until they receive an event and then immediately pushes it into the receiving channel.
func (api *API) checkForTriggerEvents(ctx context.Context) {
	for id, trigger := range api.triggers {
		go func(id string, trigger models.Trigger) {
			conn, err := grpcDial(trigger.URL)
			if err != nil {
				log.Error().Err(err).Str("trigger", id).Msg("could not connect to trigger")
			}
			defer conn.Close()

			client := sdkProto.NewTriggerClient(conn)

			for {
				select {
				case <-ctx.Done():
					return
				default:
					ctx := metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+string(trigger.Key))
					resp, err := client.Check(ctx, &sdkProto.CheckRequest{})
					if err != nil {
						if status.Code(err) == codes.Canceled {
							return
						}

						log.Error().Err(err).Str("trigger", id).Msg("could not connect to trigger")
						time.Sleep(time.Second * 5) // Don't DOS ourselves if we can't connect
						continue
					}

					// We need to account for what happens if the check exits without returning anything.
					// For instance, when the trigger gracefully shuts down it may close the channel providing
					// events, resulting in a nil as the final object.
					if resp.PipelineTriggerLabel == "" {
						continue
					}

					log.Debug().Str("trigger", id).Interface("response", resp).Msg("new trigger event found")

					result := models.TriggerResult{
						Details: resp.Details,
						State:   models.TriggerResultState(resp.Result.String()),
					}

					api.events.Publish(models.NewEventFiredTrigger(resp.NamespaceId,
						resp.PipelineId,
						resp.PipelineTriggerLabel,
						result,
						resp.Metadata))
				}
			}
		}(id, *trigger)
	}
}

// processTriggerEvents listens to and consumes all events from the TriggerEventReceived channel and starts the
// appropriate pipeline.
func (api *API) processTriggerEvents() error {
	// Subscribe to all fired trigger events so we can watch for them.
	subscription, err := api.events.Subscribe(models.FiredTriggerEvent)
	if err != nil {
		return fmt.Errorf("could not subscribe to trigger events: %w", err)
	}
	defer api.events.Unsubscribe(subscription)

	for eventRaw := range subscription.Events {
		event, ok := eventRaw.(*models.EventFiredTrigger)
		if !ok {
			continue
		}

		result := models.TriggerResult{
			Details: event.Result.Details,
			State:   event.Result.State,
		}

		api.events.Publish(models.NewEventProcessedTrigger(event.Namespace,
			event.Pipeline,
			event.Label,
			result,
			event.TriggerMetadata))

		// If the trigger event state != success then we should log that and skip it.
		if event.Result.State != models.TriggerResultStateSuccess {
			api.events.Publish(models.NewEventResolvedTrigger(event.Namespace, event.Pipeline, event.Label,
				result,
				event.TriggerMetadata))
			return nil
		}

		if !api.ignorePipelineRunEvents.Load() {
			pipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{
				NamespaceID: event.Namespace,
				ID:          event.Pipeline,
			})
			if err != nil {
				if errors.Is(err, storage.ErrEntityNotFound) {
					log.Error().Err(err).Msg("could not process trigger event; pipeline not found")
					continue
				}

				log.Error().Err(err).Msg("could not process trigger event")
				continue
			}

			triggerSubscription, exists := pipeline.Triggers[event.Label]
			if !exists {
				log.Error().Str("trigger_label", event.Label).
					Msg("could not process trigger event; could not find trigger label within pipeline")
			}

			_, err = api.createNewRun(pipeline.Namespace, pipeline.ID, triggerSubscription.Kind,
				event.Label, map[string]struct{}{}, event.TriggerMetadata)
			if err != nil {
				if errors.Is(err, ErrPipelineNotActive) {
					log.Debug().Str("namespace", pipeline.Namespace).Str("pipeline", pipeline.ID).
						Str("trigger", triggerSubscription.Kind).Msg("pipeline trigger run skipped because it is not active")
					continue
				}

				log.Error().Err(err).Msg("could not create run from trigger event")
				continue
			}

			api.events.Publish(models.NewEventResolvedTrigger(event.Namespace, event.Pipeline, event.Label,
				result,
				event.TriggerMetadata))
		} else {
			result = models.TriggerResult{
				Details: "API not accepting new events; This is due to operator controlled setting 'IgnorePipelineRunEvents'.",
				State:   models.TriggerResultStateSkipped,
			}
			log.Debug().Msg("skipped event due to IgnorePipelineRunEvents set to false")

			api.events.Publish(models.NewEventResolvedTrigger(event.Namespace, event.Pipeline, event.Label,
				result,
				event.TriggerMetadata))
		}
	}

	return nil
}

// collectLogs simply streams a container's log right to stderr. This is useful when pipeing trigger logs to the main
// application logs. Blocks until the logs have been fully read(essentially only when the container is shutdown).
func (api *API) collectLogs(schedulerID string) {
	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{
		SchedulerID: schedulerID,
	})
	if err != nil {
		log.Error().Err(err).Msg("scheduler error; could not get logs")
		return
	}

	scanner := bufio.NewScanner(logReader)
	for scanner.Scan() {
		fmt.Fprintln(os.Stderr, scanner.Text())
	}

	err = scanner.Err()
	if err != nil {
		log.Error().Err(err).Msg("could not properly read from logging stream")
	}
}
