package api

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/jmoiron/sqlx"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetPipeline(ctx context.Context, request *proto.GetPipelineRequest) (*proto.GetPipelineResponse, error) {
	if request.Id == "" {
		return &proto.GetPipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetPipelineResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	pipeline, err := api.getPipelineFromDB(request.NamespaceId, request.Id, request.Version)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetPipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.GetPipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	response := &proto.Pipeline{
		Metadata: pipeline.Metadata.ToProto(),
		Config:   pipeline.Config.ToProto(),
	}

	return &proto.GetPipelineResponse{Pipeline: response}, nil
}

func (api *API) DisablePipeline(ctx context.Context, request *proto.DisablePipelineRequest) (*proto.DisablePipelineResponse, error) {
	if request.Id == "" {
		return &proto.DisablePipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DisablePipelineResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DisablePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	metadataRaw, err := api.db.GetPipelineMetadata(api.db, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Msg("could not get pipeline from storage")
		return &proto.DisablePipelineResponse{}, status.Errorf(codes.Internal, "could not get pipeline %q", request.Id)
	}

	var metadata models.PipelineMetadata
	metadata.FromStorage(&metadataRaw)

	err = api.disablePipeline(&metadata)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return nil, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Str("id", request.Id).Msg("could not save updated pipeline to storage")
		return nil, status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
	}

	return &proto.DisablePipelineResponse{}, nil
}

func (api *API) EnablePipeline(ctx context.Context, request *proto.EnablePipelineRequest) (*proto.EnablePipelineResponse, error) {
	if request.Id == "" {
		return &proto.EnablePipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.EnablePipelineResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.EnablePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	metadataRaw, err := api.db.GetPipelineMetadata(api.db, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Msg("could not get pipeline from storage")
		return &proto.EnablePipelineResponse{}, status.Errorf(codes.Internal, "could not get pipeline %q", request.Id)
	}

	var metadata models.PipelineMetadata
	metadata.FromStorage(&metadataRaw)

	if metadata.State == models.PipelineStateActive {
		return &proto.EnablePipelineResponse{}, nil
	}

	err = api.db.UpdatePipelineMetadata(api.db, request.NamespaceId, request.Id, storage.UpdatablePipelineMetadataFields{
		State:    ptr(string(models.PipelineStateActive)),
		Modified: ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Str("id", request.Id).Msg("could not save updated pipeline to storage")
		return &proto.EnablePipelineResponse{},
			status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
	}

	go api.events.Publish(models.EventPipelineEnabled{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.Id,
	})

	return &proto.EnablePipelineResponse{}, nil
}

func (api *API) ListPipelines(ctx context.Context, request *proto.ListPipelinesRequest) (*proto.ListPipelinesResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListPipelinesResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	metadataRaw, err := api.db.ListPipelineMetadata(api.db, int(request.Offset), int(request.Limit), request.NamespaceId)
	if err != nil {
		log.Error().Err(err).Msg("could not get pipelines")
		return &proto.ListPipelinesResponse{}, status.Error(codes.Internal, "failed to retrieve pipelines from database")
	}

	protoPipelines := []*proto.PipelineMetadata{}
	for _, pipeline := range metadataRaw {
		var metadata models.PipelineMetadata
		metadata.FromStorage(&pipeline)
		protoPipelines = append(protoPipelines, metadata.ToProto())
	}

	return &proto.ListPipelinesResponse{
		Pipelines: protoPipelines,
	}, nil
}

func (api *API) DeployPipeline(ctx context.Context, request *proto.DeployPipelineRequest) (*proto.DeployPipelineResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return nil, status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if request.Id == "" {
		return nil, status.Error(codes.FailedPrecondition, "pipeline id required but not found")
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return nil, status.Error(codes.PermissionDenied, "access denied")
	}

	var startVersion int64
	var endVersion int64
	var deploymentID int64

	// Step 1: Insert the new deployment
	err = storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
		// Check that there are no currently running deployments
		deployments, err := api.db.ListRunningPipelineDeployments(tx, 0, 1, request.NamespaceId, request.Id)
		if err != nil {
			return err
		}

		if len(deployments) != 0 {
			log.Error().Str("namespace", request.NamespaceId).Str("pipeline", request.Id).
				Int("total_deployments", len(deployments)).Msgf("deployment failure; deployment is already in progress")
			return fmt.Errorf("deployment failure; deployment is already in progress")
		}

		// Get the latest live config so we can deprecate it.
		latestLiveConfig, err := api.db.GetLatestLivePipelineConfig(tx, request.NamespaceId, request.Id)
		if err != nil {
			if !errors.Is(err, storage.ErrEntityNotFound) {
				return err
			}
		}

		// Set start version; if there are no live pipeline configurations set the one being deployed to the
		// be the starting version.
		if errors.Is(err, storage.ErrEntityNotFound) {
			startVersion = request.Version
		} else {
			startVersion = latestLiveConfig.Version
		}

		// Finally get the latest deployment so we can increment the ID by one.
		latestDeployments, err := api.db.ListPipelineDeployments(tx, 0, 1, request.NamespaceId, request.Id)
		if err != nil {
			return err
		}

		var latestDeploymentID int64

		if len(latestDeployments) > 0 {
			latestDeploymentID = latestDeployments[0].ID
		}

		endVersion = request.Version
		deploymentID = latestDeploymentID + 1

		deployment := models.NewDeployment(request.NamespaceId, request.Id, deploymentID, startVersion, endVersion)

		err = api.db.InsertPipelineDeployment(tx, deployment.ToStorage())
		if err != nil {
			if errors.Is(err, storage.ErrEntityExists) {
				return status.Error(codes.AlreadyExists, "deployment already exists")
			}

			log.Error().Err(err).Msg("could not insert deployment")
			return status.Error(codes.Internal, "could not insert deployment")
		}

		return nil
	})
	if err != nil {
		return nil, err
	}

	// Step 2: Officially start the deployment.
	go api.events.Publish(models.EventPipelineDeployStarted{
		NamespaceID:  request.NamespaceId,
		PipelineID:   request.Id,
		StartVersion: startVersion,
		EndVersion:   endVersion,
	})

	// Step 3: We mark the new pipeline config as Live and Active, signifying that it is ready to take traffic.
	// If this wasn't a same version upgrade. We mark the old pipeline config as Deprecated and Disabled.
	// TODO(clintjedwards): Eventually this will become a more intricate function which will allow for more
	// complex deployment types.

	err = storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
		// Update end version config
		err = api.db.UpdatePipelineConfig(tx, request.NamespaceId, request.Id, endVersion,
			storage.UpdatablePipelineConfigFields{
				State:      ptr(string(models.PipelineConfigStateLive)),
				Deprecated: ptr(int64(0)),
			})
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
			}
			log.Error().Err(err).Str("namespace", request.NamespaceId).
				Str("pipeline", request.Id).Int64("deployment", deploymentID).
				Msg("could not save updated pipeline to storage during deployment")
			return status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
		}

		// Update start version config
		if startVersion != endVersion {
			err = api.db.UpdatePipelineConfig(tx, request.NamespaceId, request.Id, startVersion,
				storage.UpdatablePipelineConfigFields{
					State:      ptr(string(models.PipelineConfigStateDeprecated)),
					Deprecated: ptr(time.Now().UnixMilli()),
				})
			if err != nil {
				if errors.Is(err, storage.ErrEntityNotFound) {
					return status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
				}
				log.Error().Err(err).Str("namespace", request.NamespaceId).
					Str("pipeline", request.Id).Int64("deployment", deploymentID).
					Msg("could not save updated pipeline to storage during deployment")
				return status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
			}
		}

		return nil
	})
	if err != nil {
		statusReason := models.DeploymentStatusReason{
			Reason:      models.DeploymentStatusReasonUnknown,
			Description: fmt.Sprintf("Deployment has failed due to an internal error: %v", err),
		}

		// Mark deployment as failed
		err = api.db.UpdatePipelineDeployment(api.db, request.NamespaceId, request.Id, deploymentID,
			storage.UpdatablePipelineDeploymentFields{
				Ended:        ptr(time.Now().UnixMilli()),
				State:        ptr(string(models.DeploymentStateComplete)),
				Status:       ptr(string(models.DeploymentStatusFailed)),
				StatusReason: ptr(statusReason.ToJSON()),
			})
		if err != nil {
			log.Error().Err(err).Str("namespace", request.NamespaceId).
				Str("pipeline", request.Id).Int64("deployment", deploymentID).
				Msg("could not complete deployment for pipeline")
			return nil, status.Errorf(codes.Internal, "could not complete deployment for pipeline %q", request.Id)
		}
	}

	// Complete deployment
	err = api.db.UpdatePipelineDeployment(api.db, request.NamespaceId, request.Id, deploymentID,
		storage.UpdatablePipelineDeploymentFields{
			Ended:  ptr(time.Now().UnixMilli()),
			State:  ptr(string(models.DeploymentStateComplete)),
			Status: ptr(string(models.DeploymentStatusSuccessful)),
		})
	if err != nil {
		log.Error().Err(err).Str("namespace", request.NamespaceId).
			Str("pipeline", request.Id).Int64("deployment", deploymentID).
			Msg("could not complete deployment for pipeline")
		return nil, status.Errorf(codes.Internal, "could not complete deployment for pipeline %q", request.Id)
	}

	// Lastly: We're done. So now we just need to complete the deployment.
	go api.events.Publish(models.EventPipelineDeployCompleted{
		NamespaceID:  request.NamespaceId,
		PipelineID:   request.Id,
		StartVersion: startVersion,
		EndVersion:   endVersion,
	})

	return &proto.DeployPipelineResponse{
		DeploymentId: deploymentID,
	}, nil
}

func (api *API) DeletePipeline(ctx context.Context, request *proto.DeletePipelineRequest) (*proto.DeletePipelineResponse, error) {
	if request.Id == "" {
		return &proto.DeletePipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeletePipelineResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.db.DeletePipelineMetadata(api.db, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeletePipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.DeletePipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	go api.events.Publish(models.EventPipelineDeleted{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.Id,
	})

	return &proto.DeletePipelineResponse{}, nil
}
