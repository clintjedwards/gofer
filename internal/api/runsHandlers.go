package api

import (
	"context"
	"errors"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/jmoiron/sqlx"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetRun(ctx context.Context, request *proto.GetRunRequest) (*proto.GetRunResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetRunResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	runRaw, err := api.db.GetPipelineRun(api.db, request.NamespaceId, request.PipelineId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetRunResponse{}, status.Error(codes.FailedPrecondition, "run not found")
		}
		log.Error().Err(err).Int64("Run", request.Id).Msg("could not get run")
		return &proto.GetRunResponse{}, status.Error(codes.Internal, "failed to retrieve run from database")
	}

	var run models.Run
	run.FromStorage(&runRaw)

	return &proto.GetRunResponse{Run: run.ToProto()}, nil
}

func (api *API) ListRuns(ctx context.Context, request *proto.ListRunsRequest) (*proto.ListRunsResponse, error) {
	if request.PipelineId == "" {
		return &proto.ListRunsResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListRunsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	runsRaw, err := api.db.ListPipelineRuns(api.db, int(request.Offset), int(request.Limit), request.NamespaceId, request.PipelineId)
	if err != nil {
		log.Error().Err(err).Msg("could not get runs")
		return &proto.ListRunsResponse{}, status.Error(codes.Internal, "failed to retrieve runs from database")
	}

	protoRuns := []*proto.Run{}
	for _, runRaw := range runsRaw {

		var run models.Run
		run.FromStorage(&runRaw)

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

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.StartRunResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.StartRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if api.ignorePipelineRunEvents.Load() {
		return &proto.StartRunResponse{}, status.Error(codes.FailedPrecondition, "api is not accepting new events at this time")
	}

	var newRunID int64
	var pipeline models.PipelineMetadata
	var newRun *models.Run
	var latestConfig models.PipelineConfig

	err = storage.InsideTx(api.db.DB, func(tx *sqlx.Tx) error {
		pipelineRaw, err := api.db.GetPipelineMetadata(tx, request.NamespaceId, request.PipelineId)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return err
			}
			return err
		}

		pipeline.FromStorage(&pipelineRaw)

		if pipeline.State != models.PipelineStateActive {
			return ErrPipelineNotActive
		}

		latestConfigRaw, err := api.db.GetLatestLivePipelineConfig(tx, request.NamespaceId, pipeline.ID)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return ErrNoValidConfiguration
			}
			return err
		}

		latestVersion := latestConfigRaw.Version

		commonTasks, err := api.db.ListPipelineCommonTaskSettings(tx, request.NamespaceId, pipeline.ID, latestVersion)
		if err != nil {
			return err
		}

		customTasks, err := api.db.ListPipelineCustomTasks(tx, request.NamespaceId, pipeline.ID, latestVersion)
		if err != nil {
			return err
		}

		latestConfig.FromStorage(&latestConfigRaw, &commonTasks, &customTasks)

		latestRun, err := api.db.ListPipelineRuns(api.db, 0, 1, request.NamespaceId, pipeline.ID)
		if err != nil {
			return err
		}

		var latestRunID int64

		if len(latestRun) > 0 {
			latestRunID = latestRun[0].ID
		}

		newRunID = latestRunID + 1

		// Create the new run and retrieve it's ID.
		newRun = models.NewRun(request.NamespaceId, request.PipelineId, latestVersion, newRunID, models.ExtensionInfo{
			Name:  "manual",
			Label: "api",
		}, convertVarsToSlice(request.Variables, models.VariableSourceRunOptions))

		err = api.db.InsertPipelineRun(api.db, newRun.ToStorage())
		if err != nil {
			log.Error().Err(err).Msg("could not insert pipeline into db")
			return err
		}

		return nil
	})
	if err != nil {
		if errors.Is(err, ErrNoValidConfiguration) {
			return nil, status.Errorf(codes.FailedPrecondition,
				"Could not start pipeline run; no valid configuration found. Run `gofer up <path> to register a new pipeline configuration`")
		}
		if errors.Is(err, ErrPipelineNotActive) {
			return &proto.StartRunResponse{}, status.Errorf(codes.FailedPrecondition,
				"Could not start pipeline run; pipeline is not in state Active. Run `gofer pipeline enable %s`",
				request.PipelineId)
		}
		return &proto.StartRunResponse{}, status.Error(codes.Internal, "could not start pipeline run")
	}

	// Publish that the run has started
	go api.events.Publish(models.EventStartedRun{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		RunID:       newRunID,
	})

	// Publish a fake extension event for the manual run
	go api.events.Publish(models.EventResolvedExtensionEvent{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		Name:        "manual",
		Label:       "api",
		Result: models.ExtensionResult{
			Details: "triggered via API",
			Status:  models.ExtensionResultStateSuccess,
		},
	})

	runStateMachine := api.newRunStateMachine(&pipeline, &latestConfig, newRun)

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

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.RetryRunResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.RetryRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if request.RunId == 0 {
		return &proto.RetryRunResponse{}, status.Error(codes.FailedPrecondition, "run id required")
	}

	runRaw, err := api.db.GetPipelineRun(api.db, request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.RetryRunResponse{}, status.Errorf(codes.FailedPrecondition,
				"run %d not found", request.RunId)
		}
		log.Error().Err(err).Int64("Run", request.RunId).Msg("could not get run")
		return &proto.RetryRunResponse{}, status.Errorf(codes.Internal,
			"failed to retrieve run %d from database", request.RunId)
	}

	var run models.Run
	run.FromStorage(&runRaw)

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

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.CancelRunResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.CancelRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	if request.RunId == 0 {
		return &proto.CancelRunResponse{}, status.Error(codes.FailedPrecondition, "run id required")
	}

	runRaw, err := api.db.GetPipelineRun(api.db, request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.CancelRunResponse{}, status.Errorf(codes.FailedPrecondition, "run %d not found", request.RunId)
		}
		log.Error().Err(err).Int64("Run", request.RunId).Msg("could not get run")
		return &proto.CancelRunResponse{}, status.Errorf(codes.Internal, "failed to retrieve run %d from database", request.RunId)
	}

	var run models.Run
	run.FromStorage(&runRaw)

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

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.CancelAllRunsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

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
