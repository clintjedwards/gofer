package api

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

func (api *API) startTrigger(trigger models.TriggerRegistration, cert, key string) error {
	triggerKey := generateToken(32)

	// We need to first populate the triggers with their required environment variables.
	// Order is important here maps later in the list will overwrite earlier maps.
	// We first include the Gofer defined environment variables and then the operator configured environment
	// variables.
	systemTriggerVars := []models.Variable{
		{
			Key:    "GOFER_TRIGGER_TLS_CERT",
			Value:  cert,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_TLS_KEY",
			Value:  key,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_NAME",
			Value:  trigger.Name,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_LOG_LEVEL",
			Value:  api.config.LogLevel,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_KEY",
			Value:  triggerKey,
			Source: models.VariableSourceSystem,
		},
	}

	log.Info().Str("name", trigger.Name).Msg("starting trigger")

	systemTriggerVarsMap := convertVarsToMap(systemTriggerVars)
	triggerVarsMap := convertVarsToMap(trigger.Variables)
	envVars := mergeMaps(systemTriggerVarsMap, triggerVarsMap)

	sc := scheduler.StartContainerRequest{
		ID:               triggerContainerID(trigger.Name),
		ImageName:        trigger.Image,
		EnvVars:          envVars,
		RegistryAuth:     trigger.RegistryAuth,
		EnableNetworking: true,
	}

	resp, err := api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("trigger", trigger.Name).Msg("could not start trigger")
		return err
	}

	var info *proto.TriggerInfoResponse

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
			log.Debug().Err(err).Str("trigger", trigger.Name).Msg("could not connect to trigger")
			time.Sleep(time.Millisecond * 300)
			attempts++
			continue
		}

		client := proto.NewTriggerServiceClient(conn)

		ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(triggerKey))

		info, err = client.Info(ctx, &proto.TriggerInfoRequest{})
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
	api.triggers.Set(trigger.Name, &models.Trigger{
		Registration:  trigger,
		URL:           resp.URL,
		SchedulerID:   resp.SchedulerID,
		Started:       time.Now().UnixMilli(),
		State:         models.TriggerStateRunning,
		Documentation: info.Documentation,
		Key:           &triggerKey,
	})

	log.Info().
		Str("kind", trigger.Name).
		Str("id", resp.SchedulerID).
		Str("url", resp.URL).Msg("started trigger")

	go api.collectLogs(resp.SchedulerID)

	return nil
}

// startTriggers attempts to start each trigger from config on the provided scheduler. Once scheduled it then collects
// the initial trigger information so it can check for connectivity and store the network location.
// This information will eventually be used in other parts of the API to communicate with said triggers.
func (api *API) startTriggers() error {
	cert, key, err := api.getTLSFromFile(api.config.Triggers.TLSCertPath, api.config.Triggers.TLSKeyPath)
	if err != nil {
		return err
	}

	registeredTriggers, err := api.db.ListTriggerRegistrations(0, 0)
	if err != nil {
		return err
	}

	for _, trigger := range registeredTriggers {
		err := api.startTrigger(trigger, string(cert), string(key))
		if err != nil {
			return err
		}
	}

	return nil
}

// stopTriggers sends a shutdown request to each trigger, initiating a graceful shutdown for each one.
func (api *API) stopTriggers() {
	for _, triggerKey := range api.triggers.Keys() {
		trigger, exists := api.triggers.Get(triggerKey)
		if !exists {
			continue
		}

		conn, err := grpcDial(trigger.URL)
		if err != nil {
			continue
		}
		defer conn.Close()

		client := proto.NewTriggerServiceClient(conn)

		ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*trigger.Key))
		_, err = client.Shutdown(ctx, &proto.TriggerShutdownRequest{})
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
			trigger, exists := api.triggers.Get(subscription.Name)
			if !exists {
				storageErr := api.db.UpdatePipeline(pipeline.Namespace, pipeline.ID, storage.UpdatablePipelineFields{
					State: ptr(models.PipelineStateDisabled),
					Errors: &[]models.PipelineError{
						{
							Kind: models.PipelineErrorKindTriggerSubscriptionFailure,
							Description: fmt.Sprintf("Could not restore trigger subscription for trigger %s(%s); Trigger does not exist in Gofer's registered triggers.",
								label, trigger.Registration.Name),
						},
					},
				})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("trigger_label", subscription.Label).Str("trigger_name", subscription.Name).
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

			client := proto.NewTriggerServiceClient(conn)

			convertedSettings := convertVarsToSlice(subscription.Settings, models.VariableSourcePipelineConfig)
			config, err := api.interpolateVars(pipeline.Namespace, pipeline.ID, nil, convertedSettings)
			if err != nil {
				storageErr := api.db.UpdatePipeline(pipeline.Namespace, pipeline.ID, storage.UpdatablePipelineFields{
					State: ptr(models.PipelineStateDisabled),
					Errors: &[]models.PipelineError{
						{
							Kind: models.PipelineErrorKindTriggerSubscriptionFailure,
							Description: fmt.Sprintf("Could not restore trigger subscription for trigger %s(%s); Could not find appropriate secret in secret store for key.",
								label, trigger.Registration.Name),
						},
					},
				})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("trigger_label", subscription.Label).Str("trigger_name", subscription.Name).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; trigger requested does not exist within Gofer service")
				continue
			}

			ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*trigger.Key))
			_, err = client.Subscribe(ctx, &proto.TriggerSubscribeRequest{
				NamespaceId:          pipeline.Namespace,
				PipelineTriggerLabel: label,
				PipelineId:           pipeline.ID,
				Config:               convertVarsToMap(config),
			})
			if err != nil {
				storageErr := api.db.UpdatePipeline(pipeline.Namespace, pipeline.ID, storage.UpdatablePipelineFields{
					State: ptr(models.PipelineStateDisabled),
					Errors: &[]models.PipelineError{
						{
							Kind: models.PipelineErrorKindTriggerSubscriptionFailure,
							Description: fmt.Sprintf("Could not restore trigger subscription for trigger %s(%s); Could not subscribe to trigger %v.",
								label, trigger.Registration.Name, err),
						},
					},
				})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("trigger_label", subscription.Label).Str("trigger_name", subscription.Name).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; failed to contact trigger subscription endpoint")
				continue
			}

			log.Debug().Str("pipeline", pipeline.ID).Str("trigger_label", subscription.Label).
				Str("trigger_name", trigger.Registration.Name).Msg("restored subscription")
		}
	}

	return nil
}

// TODO(clintjedwards): change to watchFor
// checkForTriggerEvents spawns a goroutine for every trigger that is responsible for collecting the trigger events
// on that trigger. The "Watch" method for receiving events from a trigger is a blocking RPC, so each
// go routine essentially blocks until they receive an event and then immediately pushes it into the receiving channel.
func (api *API) checkForTriggerEvents(ctx context.Context) {
	for _, triggerKey := range api.triggers.Keys() {
		trigger, exists := api.triggers.Get(triggerKey)
		if !exists {
			continue
		}

		go func(name string, trigger models.Trigger) {
			conn, err := grpcDial(trigger.URL)
			if err != nil {
				log.Error().Err(err).Str("trigger", name).Msg("could not connect to trigger")
			}
			defer conn.Close()

			client := proto.NewTriggerServiceClient(conn)

			for {
				select {
				case <-ctx.Done():
					return
				default:
					ctx := metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+string(*trigger.Key))
					resp, err := client.Watch(ctx, &proto.TriggerWatchRequest{})
					if err != nil {
						if status.Code(err) == codes.Canceled {
							return
						}

						log.Error().Err(err).Str("trigger", name).Msg("could not connect to trigger")
						trigger.State = models.TriggerStateUnknown
						api.triggers.Set(*trigger.Key, &trigger)
						time.Sleep(time.Second * 5) // Don't DOS ourselves if we can't connect
						continue
					}

					// We need to account for what happens if the check exits without returning anything.
					// For instance, when the trigger gracefully shuts down it may close the channel providing
					// events, resulting in a nil as the final object.
					if resp.PipelineTriggerLabel == "" {
						continue
					}

					log.Debug().Str("trigger", name).Interface("response", resp).Msg("new trigger event found")

					result := models.TriggerResult{
						Details: resp.Details,
						Status:  models.TriggerResultStatus(resp.Result.String()),
					}

					go api.events.Publish(models.EventFiredTriggerEvent{
						NamespaceID: resp.NamespaceId,
						PipelineID:  resp.PipelineId,
						Name:        trigger.Registration.Name,
						Label:       resp.PipelineTriggerLabel,
						Result:      result,
						Metadata:    resp.Metadata,
					})
				}
			}
		}(triggerKey, *trigger)
	}
}

func (api *API) resolveFiredTriggerEvent(evt *models.EventFiredTriggerEvent, result models.TriggerResult, metadata map[string]string) {
	go api.events.Publish(models.EventResolvedTriggerEvent{
		NamespaceID: evt.NamespaceID,
		PipelineID:  evt.PipelineID,
		Name:        evt.Name,
		Label:       evt.Label,
		Result:      result,
		Metadata:    metadata,
	})
}

func (api *API) processTriggerEvent(event *models.EventFiredTriggerEvent) {
	go api.events.Publish(models.EventProcessedTriggerEvent{
		NamespaceID: event.NamespaceID,
		PipelineID:  event.PipelineID,
		Name:        event.Name,
		Label:       event.Label,
	})

	// If the trigger event status != success then we should log that and skip it.
	if event.Result.Status != models.TriggerResultStateSuccess {
		api.resolveFiredTriggerEvent(event, event.Result, map[string]string{})
		return
	}

	// If the pipeline isn't accepting any new runs we skip the trigger event.
	if api.ignorePipelineRunEvents.Load() {
		log.Debug().Msg("skipped event due to IgnorePipelineRunEvents set to false")

		api.resolveFiredTriggerEvent(event, models.TriggerResult{
			Details: "API not accepting new events; This is due to operator controlled setting 'IgnorePipelineRunEvents'.",
			Status:  models.TriggerResultStateSkipped,
		}, map[string]string{})
		return
	}

	pipeline, err := api.db.GetPipeline(nil, event.NamespaceID, event.PipelineID)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			log.Error().Err(err).Msg("Pipeline not found")
			api.resolveFiredTriggerEvent(event, models.TriggerResult{
				Details: "Could not process trigger event; pipeline not found.",
				Status:  models.TriggerResultStateFailure,
			}, map[string]string{})
			return
		}

		api.resolveFiredTriggerEvent(event, models.TriggerResult{
			Details: fmt.Sprintf("Internal error; %v", err),
			Status:  models.TriggerResultStateFailure,
		}, map[string]string{})
		log.Error().Err(err).Msg("could not process trigger event")
		return
	}

	triggerSubscription, exists := pipeline.Triggers[event.Label]
	if !exists {
		log.Error().Str("trigger_label", event.Label).
			Msg("could not process trigger event; could not find trigger label within pipeline")
		api.resolveFiredTriggerEvent(event, models.TriggerResult{
			Details: "Trigger subscription no longer found in pipeline config.",
			Status:  models.TriggerResultStateFailure,
		}, map[string]string{})
		return
	}

	if pipeline.State != models.PipelineStateActive {
		log.Debug().Str("trigger_label", event.Label).
			Msg("skipped trigger event; pipeline is not active.")
		api.resolveFiredTriggerEvent(event, models.TriggerResult{
			Details: "Pipeline is not active",
			Status:  models.TriggerResultStateSkipped,
		}, map[string]string{})
		return
	}

	// Create the new run and retrieve it's ID.
	newRun := models.NewRun(pipeline.Namespace, pipeline.ID, models.TriggerInfo{
		Name:  triggerSubscription.Name,
		Label: triggerSubscription.Label,
	}, convertVarsToSlice(event.Metadata, models.VariableSourceTrigger))

	runID, err := api.db.InsertRun(newRun)
	if err != nil {
		log.Error().Err(err).Msg("could not insert pipeline into db")
		api.resolveFiredTriggerEvent(event, models.TriggerResult{
			Details: fmt.Sprintf("Internal error; %v", err),
			Status:  models.TriggerResultStateFailure,
		}, map[string]string{})
		return
	}

	newRun.ID = runID

	runStateMachine := api.newRunStateMachine(&pipeline, newRun)

	// Make sure the pipeline is ready for a new run.
	for runStateMachine.parallelismLimitExceeded() {
		time.Sleep(time.Second * 1)
	}

	// Finally, launch the thread that will launch all the task runs for a job.
	go runStateMachine.executeTaskTree()

	api.resolveFiredTriggerEvent(event, event.Result, event.Metadata)
}

// processTriggerEvents listens to and consumes all events from the TriggerEventReceived channel and starts the
// appropriate pipeline.
func (api *API) processTriggerEvents() error {
	// Subscribe to all fired trigger events so we can watch for them.
	subscription, err := api.events.Subscribe(models.EventKindFiredTriggerEvent)
	if err != nil {
		return fmt.Errorf("could not subscribe to trigger events: %w", err)
	}
	defer api.events.Unsubscribe(subscription)

	for eventRaw := range subscription.Events {
		event, ok := eventRaw.Details.(*models.EventFiredTriggerEvent)
		if !ok {
			continue
		}

		go api.processTriggerEvent(event)
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
