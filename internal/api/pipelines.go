package api

import (
	"context"
	"fmt"
	"time"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/jmoiron/sqlx"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/metadata"
)

// unsubscribeExtension contacts the extension and remove the pipeline subscription.
func (api *API) unsubscribeExtension(namespace, pipeline, name, label string) error {
	extension, exists := api.extensions.Get(name)
	if !exists {
		return fmt.Errorf("could not find extension name %q in registered extension list", name)
	}

	conn, err := grpcDial(extension.URL)
	if err != nil {
		log.Error().Err(err).Str("name", extension.Registration.Name).Msg("could not connect to extension")
	}
	defer conn.Close()

	client := proto.NewExtensionServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*extension.Key))
	_, err = client.Unsubscribe(ctx, &proto.ExtensionUnsubscribeRequest{
		PipelineExtensionLabel: label,
		NamespaceId:            namespace,
		PipelineId:             pipeline,
	})
	if err != nil {
		log.Error().Err(err).Str("namespace", namespace).
			Str("pipeline", pipeline).Str("extension_label", label).
			Str("name", extension.Registration.Name).Msg("could not unsubscribe from extension")
		return err
	}

	log.Debug().Str("namespace", namespace).
		Str("pipeline", pipeline).Str("extension_label", label).
		Str("name", extension.Registration.Name).Msg("unsubscribed extension")

	return nil
}

// subscribeExtension takes a pipeline config requested extension and communicates with the extension container
// in order appropriately make sure the extension is aware for the pipeline.
func (api *API) subscribeExtension(subscription *models.PipelineExtensionSubscription) error {
	extension, exists := api.extensions.Get(subscription.Name)
	if !exists {
		return fmt.Errorf("extension %q not found;", subscription.Name)
	}

	convertedSettings := convertVarsToSlice(subscription.Settings, models.VariableSourcePipelineConfig)
	interpolatedSettings, err := api.interpolateVars(subscription.Namespace, subscription.Pipeline, nil, convertedSettings)
	if err != nil {
		return fmt.Errorf("could not subscribe extension %q for pipeline %q - namespace %q; %w",
			subscription.Label, subscription.Pipeline, subscription.Namespace, err)
	}
	parsedSettings := convertVarsToMap(interpolatedSettings)

	conn, err := grpcDial(extension.URL)
	if err != nil {
		log.Error().Err(err).Str("name", extension.Registration.Name).Msg("could not connect to extension")
	}
	defer conn.Close()

	client := proto.NewExtensionServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*extension.Key))
	_, err = client.Subscribe(ctx, &proto.ExtensionSubscribeRequest{
		NamespaceId:            subscription.Namespace,
		PipelineExtensionLabel: subscription.Label,
		PipelineId:             subscription.Pipeline,
		Config:                 parsedSettings,
	})
	if err != nil {
		log.Error().Err(err).Str("name", extension.Registration.Name).Msg("could not subscribe to extension")
		return err
	}

	log.Debug().Str("pipeline", subscription.Pipeline).Str("extension_name", subscription.Name).
		Str("extension_label", subscription.Label).Msg("subscribed pipeline to extension")

	return nil
}

// collectAllPipelines attempts to return a single list of all pipelines within the gofer service.
// Useful for functions where we must operate globally.
func (api *API) collectAllPipelines() ([]storage.PipelineMetadata, error) {
	allNamespaces := []storage.Namespace{}

	offset := 0
	for {
		namespaces, err := api.db.ListNamespaces(api.db, offset, 0)
		if err != nil {
			return []storage.PipelineMetadata{}, fmt.Errorf("could not get all namespaces; %w", err)
		}

		if len(namespaces) == 0 {
			break
		}

		allNamespaces = append(allNamespaces, namespaces...)
		offset += 100
	}

	allPipelines := []storage.PipelineMetadata{}

	for _, namespace := range allNamespaces {
		offset := 0
		for {
			pipelines, err := api.db.ListPipelineMetadata(api.db, offset, 0, namespace.ID)
			if err != nil {
				return []storage.PipelineMetadata{}, fmt.Errorf("could not get all pipelines; %w", err)
			}

			if len(pipelines) == 0 {
				break
			}

			allPipelines = append(allPipelines, pipelines...)
			offset += 100
		}
	}

	return allPipelines, nil
}

func (api *API) disablePipeline(pipeline *models.PipelineMetadata) error {
	if pipeline.State == models.PipelineStateDisabled {
		return nil
	}

	err := api.db.UpdatePipelineMetadata(api.db, pipeline.Namespace, pipeline.ID, storage.UpdatablePipelineMetadataFields{
		State:    ptr(string(models.PipelineStateDisabled)),
		Modified: ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		return err
	}

	go api.events.Publish(events.EventPipelineDisabled{
		NamespaceID: pipeline.Namespace,
		PipelineID:  pipeline.ID,
	})

	return nil
}

// Return pipeline object from DB. Passing 0 as the version gets the latest version.
func (api *API) getPipelineFromDB(namespace, id string, version int64) (*models.Pipeline, error) {
	var metadata models.PipelineMetadata
	var config models.PipelineConfig

	err := storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
		if version == 0 {
			latestConfig, err := api.db.GetLatestLivePipelineConfig(tx, namespace, id)
			if err != nil {
				return err
			}

			version = latestConfig.Version
		}

		configRaw, err := api.db.GetPipelineConfig(tx, namespace, id, version)
		if err != nil {
			return err
		}

		commonTasksRaw, err := api.db.ListPipelineCommonTaskSettings(tx, namespace, id, version)
		if err != nil {
			return err
		}

		customTasksRaw, err := api.db.ListPipelineCustomTasks(tx, namespace, id, version)
		if err != nil {
			return err
		}

		metadataRaw, err := api.db.GetPipelineMetadata(tx, namespace, id)
		if err != nil {
			return err
		}

		config.FromStorage(&configRaw, &commonTasksRaw, &customTasksRaw)
		metadata.FromStorage(&metadataRaw)

		return nil
	})
	if err != nil {
		return nil, err
	}

	return &models.Pipeline{
		Metadata: metadata,
		Config:   config,
	}, nil
}
