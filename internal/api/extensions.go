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
	grpc_retry "github.com/grpc-ecosystem/go-grpc-middleware/retry"
	"github.com/jmoiron/sqlx"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

func (api *API) startExtension(extension models.ExtensionRegistration, tlsCert, tlsKey string) error {
	// Regenerate token on every run so we can share the token with the container.
	token, hash := api.createNewAPIToken()
	newToken := models.NewToken(hash, models.TokenKindClient, []string{".*"}, map[string]string{"extension_token": "true"}, time.Hour*876600)

	err := storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
		_ = api.db.DeleteTokenByID(tx, extension.KeyID)

		keyID, err := api.db.InsertToken(tx, newToken.ToStorage())
		if err != nil {
			return err
		}

		err = api.db.UpdateGlobalExtensionRegistration(tx, extension.Name, storage.UpdatableGlobalExtensionRegistrationFields{
			KeyID: &keyID,
		})
		if err != nil {
			return err
		}

		return nil
	})
	if err != nil {
		return fmt.Errorf("could not create new token while starting extension; %w", err)
	}

	// We need to first populate the extensions with their required environment variables.
	// Order is important here maps later in the list will overwrite earlier maps.
	// We first include the Gofer defined environment variables and then the operator configured environment
	// variables.
	systemExtensionVars := []models.Variable{
		{
			Key:    "GOFER_EXTENSION_SYSTEM_TLS_CERT",
			Value:  tlsCert,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_TLS_KEY",
			Value:  tlsKey,
			Source: models.VariableSourceSystem,
		},
		{
			// The system_name is simply a human readable name for the extension.
			Key:    "GOFER_EXTENSION_SYSTEM_NAME",
			Value:  extension.Name,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_LOG_LEVEL",
			Value:  api.config.LogLevel,
			Source: models.VariableSourceSystem,
		},
		{
			// The system_key is a random string passed in by Gofer on the extensions init to act as a pre-shared
			// auth key between the two systems. When gofer makes a request to the extension, the extension verifies
			// that is has the correct auth and vice-versa.
			Key:    "GOFER_EXTENSION_SYSTEM_KEY",
			Value:  token,
			Source: models.VariableSourceSystem,
		},
		{
			// The gofer_host is the url of where the main gofer server. This is used by the extension to simply
			// communicate back to the original gofer host.
			Key:    "GOFER_EXTENSION_SYSTEM_GOFER_HOST",
			Value:  api.config.Server.Address,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_HOST",
			Value:  "0.0.0.0:8082",
			Source: models.VariableSourceSystem,
		},
	}

	if api.config.Development.ExtensionSkipTLSVerify {
		systemExtensionVars = append(systemExtensionVars, models.Variable{
			Key:    "GOFER_EXTENSION_SYSTEM_SKIP_TLS_VERIFY",
			Value:  "true",
			Source: models.VariableSourceSystem,
		})
	}

	log.Info().Str("name", extension.Name).Msg("starting extension")

	systemExtensionVarsMap := convertVarsToMap(systemExtensionVars)

	sc := scheduler.StartContainerRequest{
		ID:           extensionContainerID(extension.Name),
		ImageName:    extension.Image,
		EnvVars:      systemExtensionVarsMap,
		RegistryAuth: extension.RegistryAuth,
		Networking: &scheduler.Networking{
			Port: 8082,
		},
	}

	resp, err := api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("extension", extension.Name).Msg("could not start extension")
		return err
	}

	conn, err := grpcDial(resp.URL)
	if err != nil {
		log.Debug().Err(err).Str("extension", extension.Name).Msg("could not connect to extension")
		return err
	}
	defer conn.Close()

	client := proto.NewExtensionServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(token))

	_, err = client.Init(ctx, &proto.ExtensionInitRequest{
		Config: convertVarsToMap(extension.Variables),
	}, grpc_retry.WithMax(30))
	if err != nil {
		if status.Code(err) == codes.Canceled {
			return nil
		}

		log.Error().Err(err).Str("image", extension.Image).Str("name", extension.Name).
			Msg("failed to communicate with extension init phase")
		return err
	}

	info, err := client.Info(ctx, &proto.ExtensionInfoRequest{}, grpc_retry.WithMax(30))
	if err != nil {
		if status.Code(err) == codes.Canceled {
			return nil
		}

		log.Error().Err(err).Str("image", extension.Image).Str("name", extension.Name).
			Msg("failed to communicate with extension info phase")
		return err
	}

	// Add the extension to the in-memory registry so we can refer to its variable network location later.
	api.extensions.Set(extension.Name, &models.Extension{
		Registration:  extension,
		URL:           resp.URL,
		Started:       time.Now().UnixMilli(),
		State:         models.ExtensionStateRunning,
		Documentation: info.Documentation,
		Key:           &token,
	})

	log.Info().Str("name", extension.Name).Str("url", resp.URL).Msg("started extension")

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
