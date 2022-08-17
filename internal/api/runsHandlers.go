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

func (api *API) GetRun(ctx context.Context, request *proto.GetRunRequest) (*proto.GetRunResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	run, err := api.db.GetRun(request.NamespaceId, request.PipelineId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetRunResponse{}, status.Error(codes.FailedPrecondition, "run not found")
		}
		log.Error().Err(err).Int64("Run", request.Id).Msg("could not get run")
		return &proto.GetRunResponse{}, status.Error(codes.Internal, "failed to retrieve run from database")
	}

	return &proto.GetRunResponse{Run: run.ToProto()}, nil
}

func (api *API) ListRuns(ctx context.Context, request *proto.ListRunsRequest) (*proto.ListRunsResponse, error) {
	if request.PipelineId == "" {
		return &proto.ListRunsResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	runs, err := api.db.ListRuns(nil, int(request.Offset), int(request.Limit), request.NamespaceId, request.PipelineId)
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

	pipeline, err := api.db.GetPipeline(nil, request.NamespaceId, request.PipelineId)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition, "pipeline not found")
		}
		return &proto.StartRunResponse{}, status.Error(codes.Internal, "pipeline does not exist")
	}

	if pipeline.State != models.PipelineStateActive {
		return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition, "could not create run; pipeline is not active")
	}

	// Create the new run and retrieve it's ID.
	newRun := models.NewRun(request.NamespaceId, request.PipelineId, models.TriggerInfo{
		Name:  "manual",
		Label: "api",
	}, convertVarsToSlice(request.Variables, models.VariableSourceRunOptions))

	runID, err := api.db.InsertRun(newRun)
	if err != nil {
		log.Error().Err(err).Msg("could not insert pipeline into db")
		return &proto.StartRunResponse{}, status.Error(codes.Internal, "internal database error")
	}

	newRun.ID = runID

	// Publish that the run has started
	go api.events.Publish(models.EventStartedRun{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		RunID:       runID,
	})

	// Publish a fake trigger event for the manual run
	go api.events.Publish(models.EventResolvedTriggerEvent{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		Name:        "manual",
		Label:       "api",
		Result: models.TriggerResult{
			Details: "triggered via API",
			Status:  models.TriggerResultStateSuccess,
		},
	})

	runStateMachine := api.newRunStateMachine(&pipeline, newRun)

	// Make sure the pipeline is ready for a new run.
	for runStateMachine.parallelismLimitExceeded() {
		time.Sleep(time.Second * 1)
	}

	// Finally, launch the thread that will launch all the task runs for a job.
	go runStateMachine.executeTaskTree()

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

	if request.RunId == 0 {
		return &proto.RetryRunResponse{}, status.Error(codes.FailedPrecondition, "run id required")
	}

	run, err := api.db.GetRun(request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.RetryRunResponse{}, status.Errorf(codes.FailedPrecondition,
				"run %d not found", request.RunId)
		}
		log.Error().Err(err).Int64("Run", request.RunId).Msg("could not get run")
		return &proto.RetryRunResponse{}, status.Errorf(codes.Internal,
			"failed to retrieve run %d from database", request.RunId)
	}

	variables := map[string]string{}
	for _, variable := range run.Variables {
		variables[variable.Key] = variable.Value
	}

	response, err := api.StartRun(ctx, &proto.StartRunRequest{
		NamespaceId: request.NamespaceId,
		PipelineId:  request.PipelineId,
		Variables:   variables,
	})
	if err != nil {
		return &proto.RetryRunResponse{}, status.Error(codes.Internal, "could not start run")
	}

	return &proto.RetryRunResponse{
		Run: response.Run,
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

	if request.RunId == 0 {
		return &proto.CancelRunResponse{}, status.Error(codes.FailedPrecondition, "run id required")
	}

	run, err := api.db.GetRun(request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.CancelRunResponse{}, status.Errorf(codes.FailedPrecondition, "run %d not found", request.RunId)
		}
		log.Error().Err(err).Int64("Run", request.RunId).Msg("could not get run")
		return &proto.CancelRunResponse{}, status.Errorf(codes.Internal, "failed to retrieve run %d from database", request.RunId)
	}

	err = api.cancelRun(&run, "Run has been cancelled via API", request.Force)
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
