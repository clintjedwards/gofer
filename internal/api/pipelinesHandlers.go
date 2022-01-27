package api

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
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

	pipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{NamespaceID: request.NamespaceId, ID: request.Id})
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

	currentPipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{NamespaceID: request.NamespaceId, ID: request.Id})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Msg("could not get pipeline from storage")
		return &proto.DisablePipelineResponse{}, status.Errorf(codes.Internal, "could not get pipeline %q", request.Id)
	}

	if currentPipeline.State == models.PipelineStateAbandoned {
		return &proto.DisablePipelineResponse{}, status.Error(codes.FailedPrecondition, "cannot change the state of an abandoned pipeline")
	}

	if currentPipeline.State == models.PipelineStateDisabled {
		return &proto.DisablePipelineResponse{}, nil
	}

	currentPipeline.State = models.PipelineStateDisabled
	currentPipeline.Updated = time.Now().UnixMilli()

	err = api.storage.UpdatePipeline(storage.UpdatePipelineRequest{Pipeline: currentPipeline})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Str("id", request.Id).Msg("could not save updated pipeline to storage")
		return &proto.DisablePipelineResponse{}, status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
	}

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

	currentPipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{
		NamespaceID: request.NamespaceId,
		ID:          request.Id,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Msg("could not get pipeline from storage")
		return &proto.EnablePipelineResponse{}, status.Errorf(codes.Internal, "could not get pipeline %q", request.Id)
	}

	if currentPipeline.State == models.PipelineStateAbandoned {
		return &proto.EnablePipelineResponse{}, status.Error(codes.FailedPrecondition,
			"cannot change the state of an abandoned pipeline")
	}

	if currentPipeline.State == models.PipelineStateActive {
		return &proto.EnablePipelineResponse{}, nil
	}

	currentPipeline.State = models.PipelineStateActive
	currentPipeline.Updated = time.Now().UnixMilli()

	err = api.storage.UpdatePipeline(storage.UpdatePipelineRequest{Pipeline: currentPipeline})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnablePipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Str("id", request.Id).Msg("could not save updated pipeline to storage")
		return &proto.EnablePipelineResponse{},
			status.Errorf(codes.Internal, "could not save updated pipeline %q", request.Id)
	}

	return &proto.EnablePipelineResponse{}, nil
}

func (api *API) ListPipelines(ctx context.Context, request *proto.ListPipelinesRequest) (*proto.ListPipelinesResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	pipelines, err := api.storage.GetAllPipelines(storage.GetAllPipelinesRequest{
		NamespaceID: request.NamespaceId,
		Offset:      int(request.Offset),
		Limit:       int(request.Limit),
	})
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

func (api *API) CreatePipelineRaw(ctx context.Context, request *proto.CreatePipelineRawRequest) (*proto.CreatePipelineRawResponse, error) {
	if len(request.Content) == 0 {
		return &proto.CreatePipelineRawResponse{},
			status.Error(codes.FailedPrecondition, "content not found (byte length 0); please upload a valid pipeline config file.")
	}

	hclConfig := models.HCLPipelineConfig{}
	err := hclConfig.FromBytes(request.Content, request.Path)
	if err != nil {
		return &proto.CreatePipelineRawResponse{},
			status.Errorf(codes.FailedPrecondition, "could not parse config file; %v", err)
	}

	if hclConfig.Namespace == "" {
		hclConfig.Namespace = namespaceDefaultID
	}

	if !hasAccess(ctx, hclConfig.Namespace) {
		return &proto.CreatePipelineRawResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = hclConfig.Validate()
	if err != nil {
		return &proto.CreatePipelineRawResponse{},
			status.Errorf(codes.FailedPrecondition, "config file validation errors; %v", err)
	}

	config, err := models.FromHCL(&hclConfig)
	if err != nil {
		return &proto.CreatePipelineRawResponse{},
			status.Errorf(codes.FailedPrecondition, "could not parse config file; %v", err)
	}

	newPipeline, err := api.createPipeline(request.Path, config)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			log.Debug().Err(err).Msg("pipeline id conflict")
			return &proto.CreatePipelineRawResponse{}, status.Errorf(codes.AlreadyExists,
				"pipeline id already exists; please try again.")
		}
		if errors.Is(err, ErrTriggerNotFound) {
			return &proto.CreatePipelineRawResponse{}, status.Errorf(codes.FailedPrecondition,
				"could not create pipeline; %v;", err)
		}
		if errors.Is(err, ErrPipelineConfigNotValid) {
			return &proto.CreatePipelineRawResponse{}, status.Errorf(codes.FailedPrecondition,
				"pipeline creation encountered errors due to configuration; the pipeline has been created, but put into"+
					" disabled mode. please fix the configuration and then run 'pipeline update'; %v;", err)
		}
		return &proto.CreatePipelineRawResponse{}, status.Errorf(codes.Internal, "could not create pipeline: %v", err)
	}

	log.Info().Interface("pipeline", newPipeline).Msg("created new pipeline")
	return &proto.CreatePipelineRawResponse{
		Pipeline: newPipeline.ToProto(),
	}, nil
}

func (api *API) CreatePipelineByURL(ctx context.Context, request *proto.CreatePipelineByURLRequest) (*proto.CreatePipelineByURLResponse, error) {
	if request.Url == "" {
		return &proto.CreatePipelineByURLResponse{}, status.Error(codes.FailedPrecondition, "config url required")
	}

	hclConfig, err := api.processConfigurationByURL(request.Url)
	if err != nil {
		return &proto.CreatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition,
			"could not parse config file; %v", err)
	}

	if hclConfig.Namespace == "" {
		hclConfig.Namespace = namespaceDefaultID
	}

	if !hasAccess(ctx, hclConfig.Namespace) {
		return &proto.CreatePipelineByURLResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = hclConfig.Validate()
	if err != nil {
		return &proto.CreatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition,
			"config file validate errors; %v", err)
	}

	config, err := models.FromHCL(hclConfig)
	if err != nil {
		return &proto.CreatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition,
			"could not parse config file; %v", err)
	}

	newPipeline, err := api.createPipeline(request.Url, config)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			log.Debug().Err(err).Msg("pipeline id conflict")
			return &proto.CreatePipelineByURLResponse{}, status.Errorf(codes.AlreadyExists,
				"pipeline id already exists; please try again.")
		}
		if errors.Is(err, ErrTriggerNotFound) {
			return &proto.CreatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition,
				"could not create pipeline; %v;", err)
		}
		if errors.Is(err, ErrPipelineConfigNotValid) {
			return &proto.CreatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition,
				"pipeline creation encountered errors due to configuration; the pipeline has been created, but put into"+
					" disabled mode. please fix the configuration and then run 'pipeline update'; %v;", err)
		}
		return &proto.CreatePipelineByURLResponse{}, status.Error(codes.Internal, "could not create pipeline")
	}

	log.Info().Interface("pipeline", newPipeline).Msg("created new pipeline")
	return &proto.CreatePipelineByURLResponse{
		Pipeline: newPipeline.ToProto(),
	}, nil
}

func (api *API) UpdatePipelineRaw(ctx context.Context, request *proto.UpdatePipelineRawRequest) (*proto.UpdatePipelineRawResponse, error) {
	if request.Id == "" {
		return &proto.UpdatePipelineRawResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.UpdatePipelineRawResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if len(request.Content) == 0 {
		return &proto.UpdatePipelineRawResponse{}, status.Error(codes.FailedPrecondition, "content required")
	}

	hclConfig := models.HCLPipelineConfig{}
	err := hclConfig.FromBytes(request.Content, request.Path)
	if err != nil {
		return nil, fmt.Errorf("could not parse config file; %w", err)
	}

	updatedPipeline, err := api.updatePipeline(request.Path, request.NamespaceId, request.Id, &hclConfig)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.UpdatePipelineRawResponse{}, status.Errorf(codes.NotFound, "pipeline with id %q does not exist;", request.Id)
		}
		if errors.Is(err, ErrPipelineActive) {
			return &proto.UpdatePipelineRawResponse{}, status.Error(codes.FailedPrecondition, "pipeline must be in state 'disabled' before running update;")
		}
		if errors.Is(err, ErrPipelineAbandoned) {
			return &proto.UpdatePipelineRawResponse{}, status.Error(codes.FailedPrecondition, "pipeline cannot be abandoned")
		}
		if errors.Is(err, ErrPipelineRunsInProgress) {
			return &proto.UpdatePipelineRawResponse{}, status.Error(codes.FailedPrecondition, "pipeline must have no in progress runs before running update;")
		}
		if errors.Is(err, ErrTriggerNotFound) {
			return &proto.UpdatePipelineRawResponse{}, status.Errorf(codes.FailedPrecondition, "could not update pipeline; %v", err)
		}
		if errors.Is(err, ErrPipelineConfigNotValid) {
			return &proto.UpdatePipelineRawResponse{}, status.Errorf(codes.FailedPrecondition, "could not update pipeline; %v", err)
		}
		return &proto.UpdatePipelineRawResponse{}, status.Errorf(codes.Internal, "could not update pipeline; %v", err)
	}

	log.Info().Interface("pipeline", updatedPipeline).Msg("updated pipeline")
	return &proto.UpdatePipelineRawResponse{
		Pipeline: updatedPipeline.ToProto(),
	}, nil
}

func (api *API) UpdatePipelineByURL(ctx context.Context, request *proto.UpdatePipelineByURLRequest) (*proto.UpdatePipelineByURLResponse, error) {
	if request.Id == "" {
		return &proto.UpdatePipelineByURLResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.Url == "" {
		return &proto.UpdatePipelineByURLResponse{}, status.Error(codes.FailedPrecondition, "url required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.UpdatePipelineByURLResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	hclConfig, err := api.processConfigurationByURL(request.Url)
	if err != nil {
		return &proto.UpdatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition, "could not parse config file; %v", err)
	}

	updatedPipeline, err := api.updatePipeline(request.Url, request.NamespaceId, request.Id, hclConfig)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.UpdatePipelineByURLResponse{}, status.Errorf(codes.NotFound, "pipeline with id %q does not exist", request.Id)
		}
		if errors.Is(err, ErrPipelineActive) {
			return &proto.UpdatePipelineByURLResponse{}, status.Error(codes.FailedPrecondition, "pipeline must be in state 'disabled' before running update")
		}
		if errors.Is(err, ErrPipelineAbandoned) {
			return &proto.UpdatePipelineByURLResponse{}, status.Error(codes.FailedPrecondition, "pipeline cannot be abandoned")
		}
		if errors.Is(err, ErrPipelineRunsInProgress) {
			return &proto.UpdatePipelineByURLResponse{}, status.Error(codes.FailedPrecondition, "pipeline must have no in progress runs before running update")
		}
		if errors.Is(err, ErrTriggerNotFound) {
			return &proto.UpdatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition, "could not update pipeline; %v", err)
		}
		if errors.Is(err, ErrPipelineConfigNotValid) {
			return &proto.UpdatePipelineByURLResponse{}, status.Errorf(codes.FailedPrecondition, "could not update pipeline; %v", err)
		}
		return &proto.UpdatePipelineByURLResponse{}, status.Errorf(codes.Internal, "could not update pipeline; %v", err)
	}

	log.Info().Interface("pipeline", updatedPipeline).Msg("updated pipeline")
	return &proto.UpdatePipelineByURLResponse{
		Pipeline: updatedPipeline.ToProto(),
	}, nil
}

func (api *API) AbandonPipeline(ctx context.Context, request *proto.AbandonPipelineRequest) (*proto.AbandonPipelineResponse, error) {
	if request.Id == "" {
		return &proto.AbandonPipelineResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.AbandonPipelineResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	pipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{NamespaceID: request.NamespaceId, ID: request.Id})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.AbandonPipelineResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.AbandonPipelineResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	err = api.unsubscribeAllTriggers(pipeline)
	if err != nil {
		return nil, err
	}

	pipeline.State = models.PipelineStateAbandoned
	pipeline.Updated = time.Now().UnixMilli()

	err = api.storage.UpdatePipeline(storage.UpdatePipelineRequest{Pipeline: pipeline})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.AbandonPipelineResponse{}, status.Errorf(codes.NotFound, "pipeline %q not found", request.Id)
		}
		log.Error().Err(err).Str("id", request.Id).Msg("could not abandon pipeline")
		return &proto.AbandonPipelineResponse{}, status.Errorf(codes.Internal, "could not abandon pipeline: %q", request.Id)
	}

	return &proto.AbandonPipelineResponse{}, nil
}
