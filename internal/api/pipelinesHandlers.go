package api

import (
	"context"
	"errors"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetPipeline(ctx context.Context, request *proto.GetPipelineRequest) (*proto.GetPipelineResponse, error) {
	if request.Id == "" {
		return &proto.GetPipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	pipeline, err := api.db.GetPipeline(nil, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetPipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.GetPipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	return &proto.GetPipelineResponse{Pipeline: pipeline.ToProto()}, nil
}

func (api *API) DisablePipeline(ctx context.Context, request *proto.DisablePipelineRequest) (*proto.DisablePipelineResponse, error) {
	if request.Id == "" {
		return &proto.DisablePipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DisablePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	currentPipeline, err := api.db.GetPipeline(nil, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Msg("could not get pipeline from storage")
		return &proto.DisablePipelineResponse{}, status.Errorf(codes.Internal, "could not get pipeline %q", request.Id)
	}

	if currentPipeline.State == models.PipelineStateDisabled {
		return &proto.DisablePipelineResponse{}, nil
	}

	err = api.db.UpdatePipeline(request.NamespaceId, request.Id, storage.UpdatablePipelineFields{
		State:    ptr(models.PipelineStateDisabled),
		Modified: ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Str("id", request.Id).Msg("could not save updated pipeline to storage")
		return &proto.DisablePipelineResponse{}, status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
	}

	go api.events.Publish(models.EventDisabledPipeline{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.Id,
	})

	return &proto.DisablePipelineResponse{}, nil
}

func (api *API) EnablePipeline(ctx context.Context, request *proto.EnablePipelineRequest) (*proto.EnablePipelineResponse, error) {
	if request.Id == "" {
		return &proto.EnablePipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.EnablePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	currentPipeline, err := api.db.GetPipeline(nil, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Msg("could not get pipeline from storage")
		return &proto.EnablePipelineResponse{}, status.Errorf(codes.Internal, "could not get pipeline %q", request.Id)
	}

	if currentPipeline.State == models.PipelineStateActive {
		return &proto.EnablePipelineResponse{}, nil
	}

	err = api.db.UpdatePipeline(request.NamespaceId, request.Id, storage.UpdatablePipelineFields{
		State:    ptr(models.PipelineStateActive),
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

	go api.events.Publish(models.EventEnabledPipeline{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.Id,
	})

	return &proto.EnablePipelineResponse{}, nil
}

func (api *API) ListPipelines(ctx context.Context, request *proto.ListPipelinesRequest) (*proto.ListPipelinesResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	pipelines, err := api.db.ListPipelines(int(request.Offset), int(request.Limit), request.NamespaceId)
	if err != nil {
		log.Error().Err(err).Msg("could not get pipelines")
		return &proto.ListPipelinesResponse{}, status.Error(codes.Internal, "failed to retrieve pipelines from database")
	}

	protoPipelines := []*proto.Pipeline{}
	for _, pipeline := range pipelines {
		protoPipelines = append(protoPipelines, pipeline.ToProto())
	}

	return &proto.ListPipelinesResponse{
		Pipelines: protoPipelines,
	}, nil
}

func (api *API) CreatePipeline(ctx context.Context, request *proto.CreatePipelineRequest) (*proto.CreatePipelineResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if request.PipelineConfig == nil {
		return &proto.CreatePipelineResponse{},
			status.Error(codes.FailedPrecondition, "pipeline configuration required but not found")
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.CreatePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	newPipeline := models.NewPipeline(request.NamespaceId, request.PipelineConfig)

	err := api.configTriggersIsValid(newPipeline.Triggers)
	if err != nil {
		return &proto.CreatePipelineResponse{},
			status.Error(codes.FailedPrecondition, err.Error())
	}

	err = api.db.InsertPipeline(newPipeline)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.CreatePipelineResponse{},
				status.Error(codes.AlreadyExists, "pipeline already exists")
		}

		return &proto.CreatePipelineResponse{},
			status.Error(codes.Internal, "could not insert pipeline")
	}

	triggers := []models.PipelineTriggerSettings{}
	for _, value := range newPipeline.Triggers {
		triggers = append(triggers, value)
	}

	successfulSubscriptions, err := api.subscribeTriggers(newPipeline.Namespace, newPipeline.ID, triggers)
	if err != nil {
		// Rollback successful subscriptions
		triggersToUnsubscribe := map[string]string{}
		for _, subscription := range successfulSubscriptions {
			triggersToUnsubscribe[subscription.Label] = subscription.Name
		}

		_ = api.unsubscribeTriggers(newPipeline.Namespace, newPipeline.ID, triggersToUnsubscribe)
		storageErr := api.db.DeletePipeline(newPipeline.Namespace, newPipeline.ID)
		if storageErr != nil {
			log.Error().Err(err).Msg("could not delete pipeline while trying to rollback subscriptions")
		}
		return &proto.CreatePipelineResponse{},
			status.Errorf(codes.FailedPrecondition, "could not successfully register all subscriptions; %v", err)
	}

	go api.events.Publish(models.EventCreatedPipeline{
		NamespaceID: newPipeline.Namespace,
		PipelineID:  newPipeline.ID,
	})

	return &proto.CreatePipelineResponse{
		Pipeline: newPipeline.ToProto(),
	}, nil
}

func (api *API) UpdatePipeline(ctx context.Context, request *proto.UpdatePipelineRequest) (*proto.UpdatePipelineResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.UpdatePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if request.PipelineConfig == nil {
		return &proto.UpdatePipelineResponse{}, status.Error(codes.FailedPrecondition, "content required")
	}

	updatedPipeline := models.NewPipeline(request.NamespaceId, request.PipelineConfig)
	// TODO(clintjedwards): We need a validate here.

	triggers := []models.PipelineTriggerSettings{}
	for _, value := range updatedPipeline.Triggers {
		triggers = append(triggers, value)
	}

	currentPipeline, err := api.db.GetPipeline(nil, request.NamespaceId, updatedPipeline.ID)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.UpdatePipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.UpdatePipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	oldTriggers := map[string]string{}
	for _, trigger := range currentPipeline.Triggers {
		oldTriggers[trigger.Label] = trigger.Name
	}

	err = api.unsubscribeTriggers(request.NamespaceId, updatedPipeline.ID, oldTriggers)
	if err != nil {
		log.Error().Err(err).Str("pipeline", currentPipeline.ID).Msg("could not unsubscribe triggers")
		return &proto.UpdatePipelineResponse{}, status.Error(codes.Internal, "could not unsubscribe triggers")
	}

	successfulSubscriptions, err := api.subscribeTriggers(updatedPipeline.Namespace, updatedPipeline.ID, triggers)
	if err != nil {
		// Rollback successful subscriptions
		triggersToUnsubscribe := map[string]string{}
		for _, subscription := range successfulSubscriptions {
			triggersToUnsubscribe[subscription.Label] = subscription.Name
		}

		_ = api.unsubscribeTriggers(updatedPipeline.Namespace, updatedPipeline.ID, triggersToUnsubscribe)
		storageErr := api.db.DeletePipeline(updatedPipeline.Namespace, updatedPipeline.ID)
		if storageErr != nil {
			log.Error().Err(err).Msg("could not delete pipeline while trying to rollback subscriptions")
		}
		return &proto.UpdatePipelineResponse{},
			status.Errorf(codes.FailedPrecondition, "could not successfully register all subscriptions; %v", err)
	}

	err = api.db.UpdatePipeline(request.NamespaceId, updatedPipeline.ID, storage.UpdatablePipelineFields{
		Name:        &updatedPipeline.Name,
		Description: &updatedPipeline.Description,
		Parallelism: &updatedPipeline.Parallelism,
		Modified:    ptr(time.Now().UnixMilli()),
		Tasks:       &updatedPipeline.CustomTasks,
		Triggers:    &updatedPipeline.Triggers,
		CommonTasks: &updatedPipeline.CommonTasks,
	})
	if err != nil {
		return nil, err
	}

	return &proto.UpdatePipelineResponse{
		Pipeline: updatedPipeline.ToProto(),
	}, nil
}

func (api *API) DeletePipeline(ctx context.Context, request *proto.DeletePipelineRequest) (*proto.DeletePipelineResponse, error) {
	if request.Id == "" {
		return &proto.DeletePipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	pipeline, err := api.db.GetPipeline(nil, request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeletePipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.DeletePipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	triggers := map[string]string{}
	for _, triggerSetting := range pipeline.Triggers {
		triggers[triggerSetting.Label] = triggerSetting.Name
	}

	err = api.unsubscribeTriggers(pipeline.Namespace, pipeline.ID, triggers)
	if err != nil {
		log.Error().Err(err).Interface("pipeline", pipeline).Msg("could not unsubscribe all triggers from pipeline")
	}

	err = api.db.DeletePipeline(request.NamespaceId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeletePipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.DeletePipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	go api.events.Publish(models.EventDeletedPipeline{
		NamespaceID: pipeline.Namespace,
		PipelineID:  pipeline.ID,
	})

	return &proto.DeletePipelineResponse{}, nil
}
