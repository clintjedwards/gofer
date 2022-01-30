package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetRun(ctx context.Context, request *proto.GetRunRequest) (*proto.GetRunResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	run, err := api.storage.GetRun(storage.GetRunRequest{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		ID:          request.Id,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetRunResponse{}, status.Error(codes.FailedPrecondition, "run not found")
		}
		log.Error().Err(err).Int64("Run", request.Id).Msg("could not get run")
		return &proto.GetRunResponse{}, status.Error(codes.Internal, "failed to retrieve run from database")
	}

	return &proto.GetRunResponse{Run: run.ToProto()}, nil
}

func (api *API) BatchGetRuns(ctx context.Context, request *proto.BatchGetRunsRequest) (*proto.BatchGetRunsResponse, error) {
	if request.PipelineId == "" {
		return &proto.BatchGetRunsResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if len(request.Ids) == 0 {
		return &proto.BatchGetRunsResponse{}, status.Error(codes.FailedPrecondition, "at least one ID required")
	}

	runs := []*proto.Run{}

	for _, id := range request.Ids {
		run, err := api.storage.GetRun(storage.GetRunRequest{
			NamespaceID: request.NamespaceId,
			PipelineID:  request.PipelineId,
			ID:          id,
		})
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return &proto.BatchGetRunsResponse{}, status.Errorf(codes.FailedPrecondition, "run %d not found", id)
			}
			log.Error().Err(err).Int64("Run", id).Msg("could not get run")
			return &proto.BatchGetRunsResponse{}, status.Errorf(codes.Internal,
				"failed to retrieve run %d from database", id)
		}

		runs = append(runs, run.ToProto())
	}

	return &proto.BatchGetRunsResponse{Runs: runs}, nil
}

func (api *API) ListRuns(ctx context.Context, request *proto.ListRunsRequest) (*proto.ListRunsResponse, error) {
	if request.PipelineId == "" {
		return &proto.ListRunsResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	runs, err := api.storage.GetAllRuns(storage.GetAllRunsRequest{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		Offset:      int(request.Offset),
		Limit:       int(request.Limit),
	})
	if err != nil {
		log.Error().Err(err).Msg("could not get runs")
		return &proto.ListRunsResponse{}, status.Error(codes.Internal, "failed to retrieve runs from database")
	}

	protoRuns := []*proto.Run{}
	for _, run := range runs {
		protoRuns = append(protoRuns, run.ToProto())
	}

	return &proto.ListRunsResponse{
		Runs: protoRuns,
	}, nil
}

func (api *API) StartRun(ctx context.Context, request *proto.StartRunRequest) (*proto.StartRunResponse, error) {
	if request.PipelineId == "" {
		return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.StartRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if api.ignorePipelineRunEvents.Load() {
		return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition, "api is not accepting new events at this time")
	}

	newRun, err := api.createNewRun(request.NamespaceId,
		request.PipelineId, "manual", "via_api", sliceToSet(request.Only), request.Variables)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.StartRunResponse{}, status.Errorf(codes.NotFound, "could not create run; %v", err)
		}
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.StartRunResponse{}, status.Errorf(codes.Internal, "could not create run; %v", err)
		}
		if errors.Is(err, ErrPipelineNotActive) {
			return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition,
				"could not create run; pipeline is not active")
		}
		if errors.Is(err, ErrPipelineRunsInProgress) {
			return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition, "could not create run; pipeline is in sequential mode and a run is already in progress")
		}
		log.Error().Err(err).Msg("could not create run")
		return &proto.StartRunResponse{}, status.Errorf(codes.Internal, "could not create run; %v", err)
	}

	// Emit a new resolve trigger so that manually initiated runs still count as a trigger.
	resolvedTriggerEvent := models.NewEventResolvedTrigger(request.NamespaceId, request.PipelineId, newRun.TriggerName,
		models.TriggerResult{
			Details: "triggered via API",
			State:   models.TriggerResultStateSuccess,
		}, request.Variables)

	api.events.Publish(resolvedTriggerEvent)

	return &proto.StartRunResponse{
		Run: newRun.ToProto(),
	}, nil
}

func (api *API) RetryRun(ctx context.Context, request *proto.RetryRunRequest) (*proto.RetryRunResponse, error) {
	if request.PipelineId == "" {
		return &proto.RetryRunResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.RetryRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if request.Id == 0 {
		return &proto.RetryRunResponse{}, status.Error(codes.FailedPrecondition, "run id required")
	}

	run, err := api.storage.GetRun(storage.GetRunRequest{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		ID:          request.Id,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.RetryRunResponse{}, status.Errorf(codes.FailedPrecondition,
				"run %d not found", request.Id)
		}
		log.Error().Err(err).Int64("Run", request.Id).Msg("could not get run")
		return &proto.RetryRunResponse{}, status.Errorf(codes.Internal,
			"failed to retrieve run %d from database", request.Id)
	}

	newRun, err := api.createNewRun(request.NamespaceId, request.PipelineId, "manual", "via_api", run.Only, run.Variables)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.RetryRunResponse{}, status.Error(codes.NotFound, "could not create run; pipeline not found")
		}
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.RetryRunResponse{}, status.Errorf(codes.Internal, "could not create run; %v", err)
		}
		if errors.Is(err, ErrPipelineNotActive) {
			return &proto.RetryRunResponse{}, status.Error(codes.FailedPrecondition,
				"could not create run; pipeline is not active")
		}
		log.Error().Err(err).Msg("could not create run")
		return &proto.RetryRunResponse{}, status.Errorf(codes.Internal, "could not create run; %v", err)
	}

	return &proto.RetryRunResponse{
		Run: newRun.ToProto(),
	}, nil
}

func (api *API) CancelRun(ctx context.Context, request *proto.CancelRunRequest) (*proto.CancelRunResponse, error) {
	if request.PipelineId == "" {
		return &proto.CancelRunResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.CancelRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if request.Id == 0 {
		return &proto.CancelRunResponse{}, status.Error(codes.FailedPrecondition, "run id required")
	}

	run, err := api.storage.GetRun(storage.GetRunRequest{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		ID:          request.Id,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.CancelRunResponse{}, status.Errorf(codes.FailedPrecondition, "run %d not found", request.Id)
		}
		log.Error().Err(err).Int64("Run", request.Id).Msg("could not get run")
		return &proto.CancelRunResponse{}, status.Errorf(codes.Internal, "failed to retrieve run %d from database", request.Id)
	}

	err = api.cancelRun(run, "Run has been cancelled via API", request.Force)
	if err != nil {
		return &proto.CancelRunResponse{}, status.Errorf(codes.Internal, "could not cancel run: %v", err)
	}

	return &proto.CancelRunResponse{}, nil
}

func (api *API) CancelAllRuns(ctx context.Context, request *proto.CancelAllRunsRequest) (*proto.CancelAllRunsResponse, error) {
	if request.PipelineId == "" {
		return &proto.CancelAllRunsResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.CancelAllRunsResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	runList, err := api.cancelAllRuns(request.NamespaceId, request.PipelineId, "Run cancelled via API.", request.Force)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.CancelAllRunsResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		log.Error().Err(err).Msg("could not get pipeline")
		return &proto.CancelAllRunsResponse{}, status.Error(codes.Internal, "failed to retrieve pipeline from database")
	}

	return &proto.CancelAllRunsResponse{
		Runs: runList,
	}, nil
}
