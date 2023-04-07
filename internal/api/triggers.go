package api

import (
	"bufio"
	"context"
	"fmt"
	"os"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

func (api *API) startExtension(extension models.ExtensionRegistration, cert, key string) error {
	extensionKey := generateToken(32)

	// We need to first populate the extensions with their required environment variables.
	// Order is important here maps later in the list will overwrite earlier maps.
	// We first include the Gofer defined environment variables and then the operator configured environment
	// variables.
	systemExtensionVars := []models.Variable{
		{
			Key:    "GOFER_PLUGIN_SYSTEM_TLS_CERT",
			Value:  cert,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_TLS_KEY",
			Value:  key,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_NAME",
			Value:  extension.Name,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_LOG_LEVEL",
			Value:  api.config.LogLevel,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_KEY",
			Value:  extensionKey,
			Source: models.VariableSourceSystem,
		},
	}

	log.Info().Str("name", extension.Name).Msg("starting extension")

	systemExtensionVarsMap := convertVarsToMap(systemExtensionVars)
	extensionVarsMap := convertVarsToMap(extension.Variables)
	envVars := mergeMaps(systemExtensionVarsMap, extensionVarsMap)

	sc := scheduler.StartContainerRequest{
		ID:               extensionContainerID(extension.Name),
		ImageName:        extension.Image,
		EnvVars:          envVars,
		RegistryAuth:     extension.RegistryAuth,
		EnableNetworking: true,
	}

	resp, err := api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("extension", extension.Name).Msg("could not start extension")
		return err
	}

	var info *proto.ExtensionInfoResponse

	// For some reason I can't get GRPC's retry to properly handle this, so instead we resort to a simple for loop.
	//
	// There is a race condition where we schedule the container, but the actual container application might not
	// have gotten a chance to start before we issue a query.
	// So instead of baking in some arbitrary sleep time between these two actions instead we retry
	// until we get a good state.
	attempts := 0
	for {
		if attempts >= 30 {
			log.Error().Msg("maximum amount of attempts reached for starting extension; could not connect to extension")
			return fmt.Errorf("could not connect to extension; maximum amount of attempts reached")
		}

		conn, err := grpcDial(resp.URL)
		if err != nil {
			log.Debug().Err(err).Str("extension", extension.Name).Msg("could not connect to extension")
			time.Sleep(time.Millisecond * 300)
			attempts++
			continue
		}

		client := proto.NewExtensionServiceClient(conn)

		ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(extensionKey))

		info, err = client.Info(ctx, &proto.ExtensionInfoRequest{})
		if err != nil {
			if status.Code(err) == codes.Canceled {
				return nil
			}

			conn.Close()
			log.Debug().Err(err).Msg("failed to communicate with extension startup")
			time.Sleep(time.Millisecond * 300)
			attempts++
			continue
		}

		conn.Close()
		break
	}

	// Add the extension to the in-memory registry so we can refer to its variable network location later.
	api.extensions.Set(extension.Name, &models.Extension{
		Registration:  extension,
		URL:           resp.URL,
		Started:       time.Now().UnixMilli(),
		State:         models.ExtensionStateRunning,
		Documentation: info.Documentation,
		Key:           &extensionKey,
	})

	log.Info().
		Str("kind", extension.Name).
		Str("url", resp.URL).Msg("started extension")

	go api.collectLogs(extensionContainerID(extension.Name))

	return nil
}

// startExtensions attempts to start each extension from config on the provided scheduler. Once scheduled it then collects
// the initial extension information so it can check for connectivity and store the network location.
// This information will eventually be used in other parts of the API to communicate with said extensions.
func (api *API) startExtensions() error {
	cert, key, err := api.getTLSFromFile(api.config.Extensions.TLSCertPath, api.config.Extensions.TLSKeyPath)
	if err != nil {
		return err
	}

	registeredExtensions, err := api.db.ListGlobalExtensionRegistrations(api.db, 0, 0)
	if err != nil {
		return err
	}

	for _, extensionRaw := range registeredExtensions {
		var extension models.ExtensionRegistration
		extension.FromStorage(&extensionRaw)
		err := api.startExtension(extension, string(cert), string(key))
		if err != nil {
			return err
		}
	}

	return nil
}

// stopExtensions sends a shutdown request to each extension, initiating a graceful shutdown for each one.
func (api *API) stopExtensions() {
	for _, extensionKey := range api.extensions.Keys() {
		extension, exists := api.extensions.Get(extensionKey)
		if !exists {
			continue
		}

		conn, err := grpcDial(extension.URL)
		if err != nil {
			continue
		}
		defer conn.Close()

		client := proto.NewExtensionServiceClient(conn)

		ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*extension.Key))
		_, err = client.Shutdown(ctx, &proto.ExtensionShutdownRequest{})
		if err != nil {
			continue
		}

	}
}

// restoreExtensionSubscriptions iterates through all pipelines and subscribes them all back to their defined extensions.
// We need to do this because of the fact that extensions are stateless and ephemeral and the only way they even know
// of the existence of pipelines is through the "subscribe" function.
func (api *API) restoreExtensionSubscriptions() error {
	pipelines, err := api.collectAllPipelines()
	if err != nil {
		return fmt.Errorf("could not restore extension subscriptions; %w", err)
	}

	for _, pipeline := range pipelines {
		extensionSubscriptions, err := api.db.ListPipelineExtensionSubscriptions(api.db, pipeline.Namespace, pipeline.ID)
		if err != nil {
			return fmt.Errorf("could not restore extension subscriptions; %w", err)
		}

		for _, subscriptionRaw := range extensionSubscriptions {
			var subscription models.PipelineExtensionSubscription
			subscription.FromStorage(&subscriptionRaw)

			extension, exists := api.extensions.Get(subscription.Name)
			if !exists {
				statusReason := models.ExtensionSubscriptionStatusReason{
					Reason:      models.ExtensionSubscriptionStatusReasonExtensionNotFound,
					Description: "Could not find extension while attempting to restore subscription",
				}

				storageErr := api.db.UpdatePipelineExtensionSubscription(api.db, pipeline.Namespace, pipeline.ID,
					subscription.Name, subscription.Label, storage.UpdateablePipelineExtensionSubscriptionFields{
						Status:       ptr(string(models.ExtensionSubscriptionStatusError)),
						StatusReason: ptr(statusReason.ToJSON()),
					})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("extension_label", subscription.Label).Str("extension_name", subscription.Name).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; extension requested does not exist within Gofer service")
				continue
			}

			conn, err := grpcDial(extension.URL)
			if err != nil {
				return fmt.Errorf("could not subscribe extension %q for pipeline %q - namespace %q; %w",
					subscription.Label, pipeline.ID, pipeline.Namespace, err)
			}
			defer conn.Close()

			client := proto.NewExtensionServiceClient(conn)

			convertedSettings := convertVarsToSlice(subscription.Settings, models.VariableSourcePipelineConfig)
			config, err := api.interpolateVars(pipeline.Namespace, pipeline.ID, nil, convertedSettings)
			if err != nil {
				statusReason := models.ExtensionSubscriptionStatusReason{
					Reason:      models.ExtensionSubscriptionStatusReasonExtensionSubscriptionFailed,
					Description: fmt.Sprintf("Could not properly pass settings during subscription: %v", err),
				}

				storageErr := api.db.UpdatePipelineExtensionSubscription(api.db, pipeline.Namespace, pipeline.ID,
					subscription.Name, subscription.Label, storage.UpdateablePipelineExtensionSubscriptionFields{
						Status:       ptr(string(models.ExtensionSubscriptionStatusError)),
						StatusReason: ptr(statusReason.ToJSON()),
					})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("extension_label", subscription.Label).Str("extension_name", subscription.Name).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; extension requested does not exist within Gofer service")
				continue
			}

			ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*extension.Key))
			_, err = client.Subscribe(ctx, &proto.ExtensionSubscribeRequest{
				NamespaceId:            pipeline.Namespace,
				PipelineExtensionLabel: subscription.Label,
				PipelineId:             pipeline.ID,
				Config:                 convertVarsToMap(config),
			})
			if err != nil {
				statusReason := models.ExtensionSubscriptionStatusReason{
					Reason:      models.ExtensionSubscriptionStatusReasonExtensionSubscriptionFailed,
					Description: fmt.Sprintf("Could not properly subscribe to extension: %v", err),
				}

				storageErr := api.db.UpdatePipelineExtensionSubscription(api.db, pipeline.Namespace, pipeline.ID,
					subscription.Name, subscription.Label, storage.UpdateablePipelineExtensionSubscriptionFields{
						Status:       ptr(string(models.ExtensionSubscriptionStatusError)),
						StatusReason: ptr(statusReason.ToJSON()),
					})
				if storageErr != nil {
					log.Error().Err(storageErr).Msg("could not update pipeline")
				}
				log.Error().Err(err).Str("extension_label", subscription.Label).Str("extension_name", subscription.Name).
					Str("pipeline", pipeline.ID).Str("namespace", pipeline.Namespace).
					Msg("could not restore subscription; failed to contact extension subscription endpoint")
				continue
			}

			log.Debug().Str("pipeline", pipeline.ID).Str("extension_label", subscription.Label).
				Str("extension_name", extension.Registration.Name).Msg("restored subscription")
		}
	}

	return nil
}

// watchForExtensionEvents spawns a goroutine for every extension that is responsible for collecting the extension events
// on that extension. The "Watch" method for receiving events from a extension is a blocking RPC, so each
// go routine essentially blocks until they receive an event and then immediately pushes it into the receiving channel.
func (api *API) watchForExtensionEvents(ctx context.Context) {
	for _, extensionKey := range api.extensions.Keys() {
		extension, exists := api.extensions.Get(extensionKey)
		if !exists {
			continue
		}

		go func(name string, extension models.Extension) {
			conn, err := grpcDial(extension.URL)
			if err != nil {
				log.Error().Err(err).Str("extension", name).Msg("could not connect to extension")
			}
			defer conn.Close()

			client := proto.NewExtensionServiceClient(conn)

			for {
				select {
				case <-ctx.Done():
					return
				default:
					ctx := metadata.AppendToOutgoingContext(ctx, "authorization", "Bearer "+string(*extension.Key))
					resp, err := client.Watch(ctx, &proto.ExtensionWatchRequest{})
					if err != nil {
						if status.Code(err) == codes.Canceled {
							return
						}

						log.Error().Err(err).Str("extension", name).Msg("could not connect to extension")
						extension.State = models.ExtensionStateUnknown
						api.extensions.Set(*extension.Key, &extension)
						time.Sleep(time.Second * 5) // Don't DOS ourselves if we can't connect
						continue
					}

					// If the watch command didn't return an error then the extension must be working.
					extension.State = models.ExtensionStateRunning
					api.extensions.Set(*extension.Key, &extension)

					// We need to account for what happens if the check exits without returning anything.
					// For instance, when the extension gracefully shuts down it may close the channel providing
					// events, resulting in a nil as the final object.
					if resp.PipelineExtensionLabel == "" {
						continue
					}

					log.Debug().Str("extension", name).Interface("response", resp).Msg("new extension event found")
				}
			}
		}(extensionKey, *extension)
	}
}

func (api *API) processExtensionEvent() {
	return

	// // If the extension event status != success then we should log that and skip it.
	// if event.Result.Status != models.ExtensionResultStateSuccess {
	// 	api.resolveFiredExtensionEvent(event, event.Result, map[string]string{})
	// 	return
	// }

	// // If the pipeline isn't accepting any new runs we skip the extension event.
	// if api.ignorePipelineRunEvents.Load() {
	// 	log.Debug().Msg("skipped event due to IgnorePipelineRunEvents set to false")

	// 	api.resolveFiredExtensionEvent(event, models.ExtensionResult{
	// 		Details: "API not accepting new events; This is due to operator controlled setting 'IgnorePipelineRunEvents'.",
	// 		Status:  models.ExtensionResultStateSkipped,
	// 	}, map[string]string{})
	// 	return
	// }

	// var pipeline *models.Pipeline
	// var newRun *models.Run

	// err := storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
	// 	fullPipeline, err := api.getPipelineFromDB(event.NamespaceID, event.PipelineID, -1)
	// 	if err != nil {
	// 		if errors.Is(err, storage.ErrEntityNotFound) {
	// 			log.Error().Err(err).Msg("Pipeline not found")
	// 			api.resolveFiredExtensionEvent(event, models.ExtensionResult{
	// 				Details: "Could not process extension event; pipeline not found.",
	// 				Status:  models.ExtensionResultStateFailure,
	// 			}, map[string]string{})
	// 			return err
	// 		}

	// 		api.resolveFiredExtensionEvent(event, models.ExtensionResult{
	// 			Details: fmt.Sprintf("Internal error; %v", err),
	// 			Status:  models.ExtensionResultStateFailure,
	// 		}, map[string]string{})
	// 		log.Error().Err(err).Msg("could not process extension event")
	// 		return err
	// 	}

	// 	pipeline = fullPipeline

	// 	extensionSubscriptionRaw, err := api.db.GetPipelineExtensionSubscription(tx,
	// 		event.NamespaceID, event.PipelineID, event.Name, event.Label)
	// 	if err != nil {
	// 		if errors.Is(err, storage.ErrEntityNotFound) {
	// 			log.Error().Err(err).Msg("Extension not found")
	// 			api.resolveFiredExtensionEvent(event, models.ExtensionResult{
	// 				Details: "Extension subscription not found in pipeline config.",
	// 				Status:  models.ExtensionResultStateFailure,
	// 			}, map[string]string{})
	// 			return err
	// 		}

	// 		log.Error().Str("extension_label", event.Label).
	// 			Msg("could not process extension event; could not find extension label within pipeline")
	// 	}

	// 	if fullPipeline.Metadata.State != models.PipelineStateActive {
	// 		log.Debug().Str("extension_label", event.Label).Str("namespace", event.NamespaceID).
	// 			Str("pipeline", event.PipelineID).Int64("pipeline_version", fullPipeline.Config.Version).
	// 			Msg("skipped extension event; pipeline is not active.")
	// 		api.resolveFiredExtensionEvent(event, models.ExtensionResult{
	// 			Details: "Pipeline is not active",
	// 			Status:  models.ExtensionResultStateSkipped,
	// 		}, map[string]string{})
	// 		return err
	// 	}

	// 	latestRun, err := api.db.ListPipelineRuns(tx, 0, 1, fullPipeline.Metadata.Namespace, fullPipeline.Metadata.ID)
	// 	if err != nil {
	// 		return err
	// 	}

	// 	var latestRunID int64

	// 	if len(latestRun) > 1 {
	// 		latestRunID = latestRun[0].ID
	// 	}

	// 	newRunID := latestRunID + 1

	// 	// Create the new run and retrieve it's ID.
	// 	newRun = models.NewRun(fullPipeline.Metadata.Namespace, fullPipeline.Metadata.ID, fullPipeline.Config.Version, newRunID, models.ExtensionInfo{
	// 		Name:  extensionSubscriptionRaw.Name,
	// 		Label: extensionSubscriptionRaw.Label,
	// 	}, convertVarsToSlice(event.Metadata, models.VariableSourceExtension))

	// 	err = api.db.InsertPipelineRun(tx, newRun.ToStorage())
	// 	if err != nil {
	// 		log.Error().Err(err).Msg("could not insert run into db")
	// 		api.resolveFiredExtensionEvent(event, models.ExtensionResult{
	// 			Details: fmt.Sprintf("Internal error; %v", err),
	// 			Status:  models.ExtensionResultStateFailure,
	// 		}, map[string]string{})
	// 		return err
	// 	}

	// 	return nil
	// })
	// if err != nil {
	// 	return
	// }

	// runStateMachine := api.newRunStateMachine(&pipeline.Metadata, &pipeline.Config, newRun)

	// // Make sure the pipeline is ready for a new run.
	// for runStateMachine.parallelismLimitExceeded() {
	// 	time.Sleep(time.Second * 1)
	// }

	// // Finally, launch the thread that will launch all the task runs for a job.
	// go runStateMachine.executeTaskTree()

	// api.resolveFiredExtensionEvent(event, event.Result, event.Metadata)
}

// processExtensionEvents listens to and consumes all events from the ExtensionEventReceived channel and starts the
// appropriate pipeline.
func (api *API) processExtensionEvents() error {
	return nil

	// // Subscribe to all fired extension events so we can watch for them.
	// subscription, err := api.events.Subscribe(models.EventKindFiredExtensionEvent)
	// if err != nil {
	// 	return fmt.Errorf("could not subscribe to extension events: %w", err)
	// }
	// defer api.events.Unsubscribe(subscription)

	// for eventRaw := range subscription.Events {
	// 	event, ok := eventRaw.Details.(models.EventFiredExtensionEvent)
	// 	if !ok {
	// 		continue
	// 	}

	// 	go api.processExtensionEvent(&event)
	// }

	// return nil
}

// collectLogs simply streams a container's log right to stderr. This is useful when pipeing extension logs to the main
// application logs. Blocks until the logs have been fully read(essentially only when the container is shutdown).
func (api *API) collectLogs(containerID string) {
	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{
		ID: containerID,
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
