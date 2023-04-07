package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/jmoiron/sqlx"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) RegisterPipelineConfig(ctx context.Context, request *proto.RegisterPipelineConfigRequest) (*proto.RegisterPipelineConfigResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return nil, status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if request.PipelineConfig == nil {
		return nil, status.Error(codes.FailedPrecondition, "pipeline configuration required but not found")
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return nil, status.Error(codes.PermissionDenied, "access denied")
	}

	var metadata *models.PipelineMetadata
	var config *models.PipelineConfig

	err = storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
		metadata = models.NewPipelineMetadata(request.NamespaceId, request.PipelineConfig.Id)

		// First check if the pipeline already exists, if it doesn't we'll register it for the user automatically.
		err = api.db.InsertPipelineMetadata(tx, metadata.ToStorage())
		if err != nil {
			if !errors.Is(err, storage.ErrEntityExists) {
				log.Error().Err(err).Msg("could not insert pipeline")
				return err
			}
		}

		var latestVersion int64

		latestConfig, err := api.db.GetLatestPipelineConfig(tx, request.NamespaceId, request.PipelineConfig.Id)
		if err != nil {
			if !errors.Is(err, storage.ErrEntityNotFound) {
				return err
			}
		} else {
			latestVersion = latestConfig.Version
		}

		config = models.NewPipelineConfig(request.NamespaceId, request.PipelineConfig.Id, latestVersion+1, request.PipelineConfig)
		mainConfig, commonTaskConfigs, customTaskConfigs := config.ToStorage()

		err = api.db.InsertPipelineConfig(tx, mainConfig)
		if err != nil {
			return err
		}

		for _, commonTaskConfig := range commonTaskConfigs {
			err = api.db.InsertPipelineCommonTaskSettings(tx, commonTaskConfig)
			if err != nil {
				return err
			}
		}

		for _, customTaskConfig := range customTaskConfigs {
			err = api.db.InsertPipelineCustomTask(tx, customTaskConfig)
			if err != nil {
				return err
			}
		}

		return nil
	})
	if err != nil {
		log.Error().Err(err).Msg("could not register pipeline due to database error")
		return nil, status.Errorf(codes.Internal, "could not register pipeline %v", err)
	}

	if config.Version == 1 {
		go api.events.Publish(models.EventPipelineRegistered{
			NamespaceID: metadata.Namespace,
			PipelineID:  metadata.ID,
		})
	}

	go api.events.Publish(models.EventPipelineConfigRegistered{
		NamespaceID: metadata.Namespace,
		PipelineID:  metadata.ID,
		Version:     config.Version,
	})

	return &proto.RegisterPipelineConfigResponse{
		Pipeline: &proto.Pipeline{
			Metadata: metadata.ToProto(),
			Config:   config.ToProto(),
		},
	}, nil
}

func (api *API) ListPipelineConfigs(ctx context.Context, request *proto.ListPipelineConfigsRequest) (
	*proto.ListPipelineConfigsResponse, error,
) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListPipelineConfigsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	configsRaw, err := api.db.ListPipelineConfigs(api.db, int(request.Offset), int(request.Limit),
		request.NamespaceId, request.PipelineId)
	if err != nil {
		log.Error().Err(err).Msg("could not get configs")
		return &proto.ListPipelineConfigsResponse{}, status.Error(codes.Internal, "failed to retrieve configs from database")
	}

	protoConfigs := []*proto.PipelineConfig{}
	for _, configRaw := range configsRaw {

		commonTasks, err := api.db.ListPipelineCommonTaskSettings(api.db, request.NamespaceId, request.PipelineId, configRaw.Version)
		if err != nil {
			log.Error().Err(err).Msg("could not get configs")
			return &proto.ListPipelineConfigsResponse{}, status.Error(codes.Internal, "failed to retrieve configs from database")
		}

		customTasks, err := api.db.ListPipelineCustomTasks(api.db, request.NamespaceId, request.PipelineId, configRaw.Version)
		if err != nil {
			log.Error().Err(err).Msg("could not get configs")
			return &proto.ListPipelineConfigsResponse{}, status.Error(codes.Internal, "failed to retrieve configs from database")
		}

		var config models.PipelineConfig
		config.FromStorage(&configRaw, &commonTasks, &customTasks)
		protoConfigs = append(protoConfigs, config.ToProto())
	}

	return &proto.ListPipelineConfigsResponse{
		Configs: protoConfigs,
	}, nil
}

func (api *API) GetPipelineConfig(ctx context.Context, request *proto.GetPipelineConfigRequest) (
	*proto.GetPipelineConfigResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.GetPipelineConfigResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetPipelineConfigResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	configRaw, err := api.db.GetPipelineConfig(api.db, request.NamespaceId, request.PipelineId, request.Version)
	if err != nil {
		log.Error().Err(err).Msg("could not get config")
		return &proto.GetPipelineConfigResponse{}, status.Error(codes.Internal, "failed to retrieve config from database")
	}

	commonTasks, err := api.db.ListPipelineCommonTaskSettings(api.db, request.NamespaceId, request.PipelineId, request.Version)
	if err != nil {
		log.Error().Err(err).Msg("could not get config")
		return &proto.GetPipelineConfigResponse{}, status.Error(codes.Internal, "failed to retrieve config from database")
	}

	customTasks, err := api.db.ListPipelineCustomTasks(api.db, request.NamespaceId, request.PipelineId, request.Version)
	if err != nil {
		log.Error().Err(err).Msg("could not get config")
		return &proto.GetPipelineConfigResponse{}, status.Error(codes.Internal, "failed to retrieve config from database")
	}

	var config models.PipelineConfig
	config.FromStorage(&configRaw, &commonTasks, &customTasks)

	return &proto.GetPipelineConfigResponse{Config: config.ToProto()}, nil
}

func (api *API) DeletePipelineConfig(ctx context.Context, request *proto.DeletePipelineConfigRequest) (
	*proto.DeletePipelineConfigResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.DeletePipelineConfigResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeletePipelineConfigResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineConfigResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	latestConfigRaw, err := api.db.GetLatestLivePipelineConfig(api.db, request.NamespaceId, request.PipelineId)
	if err != nil {
		return nil, err
	}

	var latestConfig models.PipelineConfig
	latestConfig.FromStorage(&latestConfigRaw, &[]storage.PipelineCommonTaskSettings{}, &[]storage.PipelineCustomTask{})

	if latestConfig.Version == request.Version {
		return nil, status.Errorf(codes.FailedPrecondition, "Cannot delete latest version of a pipeline configuration; Please upload a new config and then delete the older version")
	}

	if latestConfig.State == models.PipelineConfigStateLive {
		return nil, status.Errorf(codes.FailedPrecondition, "Cannot delete a live configuration; Please deploy a new config and then delete the old one.")
	}

	err = api.db.DeletePipelineConfig(api.db, request.NamespaceId, request.PipelineId, request.Version)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeletePipelineConfigResponse{}, status.Error(codes.FailedPrecondition, "config not found")
		}
		log.Error().Err(err).Msg("could not get config")
		return &proto.DeletePipelineConfigResponse{}, status.Error(codes.Internal, "failed to retrieve config from database")
	}

	go api.events.Publish(models.EventPipelineConfigDeleted{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		Version:     request.Version,
	})

	return &proto.DeletePipelineConfigResponse{}, nil
}
