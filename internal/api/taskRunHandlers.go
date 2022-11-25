package api

import (
	"context"
	"errors"
	"os"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/nxadm/tail"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetTaskRun(ctx context.Context, request *proto.GetTaskRunRequest) (*proto.GetTaskRunResponse, error) {
	if request.Id == "" {
		return &proto.GetTaskRunResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetTaskRunResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	taskRunRaw, err := api.db.GetPipelineTaskRun(api.db, request.NamespaceId, request.PipelineId, request.RunId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetTaskRunResponse{}, status.Error(codes.FailedPrecondition, "task run not found")
		}
		log.Error().Err(err).Msg("could not get run")
		return &proto.GetTaskRunResponse{}, status.Error(codes.Internal, "failed to retrieve task run from database")
	}

	var taskRun models.TaskRun
	taskRun.FromStorage(&taskRunRaw)

	return &proto.GetTaskRunResponse{TaskRun: taskRun.ToProto()}, nil
}

func (api *API) ListTaskRuns(ctx context.Context, request *proto.ListTaskRunsRequest) (*proto.ListTaskRunsResponse, error) {
	if request.PipelineId == "" {
		return &proto.ListTaskRunsResponse{}, status.Error(codes.FailedPrecondition, "pipeline required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListTaskRunsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	taskRunsRaw, err := api.db.ListPipelineTaskRuns(api.db, 0, 0, request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		log.Error().Err(err).Msg("could not get task runs")
		return &proto.ListTaskRunsResponse{}, status.Error(codes.Internal, "failed to retrieve runs from database")
	}

	protoTaskRuns := []*proto.TaskRun{}
	for _, taskRunRaw := range taskRunsRaw {
		var taskRun models.TaskRun
		taskRun.FromStorage(&taskRunRaw)
		protoTaskRuns = append(protoTaskRuns, taskRun.ToProto())
	}

	return &proto.ListTaskRunsResponse{
		TaskRuns: protoTaskRuns,
	}, nil
}

func (api *API) CancelTaskRun(ctx context.Context, request *proto.CancelTaskRunRequest) (*proto.CancelTaskRunResponse, error) {
	if request.Id == "" {
		return &proto.CancelTaskRunResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.PipelineId == "" {
		return &proto.CancelTaskRunResponse{}, status.Error(codes.FailedPrecondition, "pipeline required")
	}

	if request.RunId == 0 {
		return &proto.CancelTaskRunResponse{}, status.Error(codes.FailedPrecondition, "run required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.CancelTaskRunResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.CancelTaskRunResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	taskRunRaw, err := api.db.GetPipelineTaskRun(api.db, request.NamespaceId, request.PipelineId, request.RunId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.CancelTaskRunResponse{}, status.Error(codes.FailedPrecondition, "task run not found")
		}
		log.Error().Err(err).Msg("could not get run")
		return &proto.CancelTaskRunResponse{}, status.Error(codes.Internal, "failed to retrieve task run from database")
	}

	var taskRun models.TaskRun
	taskRun.FromStorage(&taskRunRaw)

	err = api.cancelTaskRun(&taskRun, request.Force)
	if err != nil {
		return &proto.CancelTaskRunResponse{}, status.Error(codes.Internal, "could not cancel container")
	}

	return &proto.CancelTaskRunResponse{}, nil
}

func (api *API) GetTaskRunLogs(request *proto.GetTaskRunLogsRequest, stream proto.Gofer_GetTaskRunLogsServer) error {
	if request.Id == "" {
		return status.Error(codes.FailedPrecondition, "id required")
	}

	if request.PipelineId == "" {
		return status.Error(codes.FailedPrecondition, "pipeline required")
	}

	if request.RunId == 0 {
		return status.Error(codes.FailedPrecondition, "run required")
	}

	namespace, err := api.resolveNamespace(stream.Context(), request.NamespaceId)
	if err != nil {
		return status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	taskRun, err := api.db.GetPipelineTaskRun(api.db, request.NamespaceId, request.PipelineId, request.RunId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return status.Error(codes.FailedPrecondition, "task run not found")
		}
		log.Error().Err(err).Msg("could not get task run")
		return status.Error(codes.Internal, "failed to retrieve task run from database")
	}

	if taskRun.LogsExpired {
		return status.Error(codes.FailedPrecondition, "task run logs have expired and are no longer available.")
	}

	if taskRun.LogsRemoved {
		return status.Error(codes.FailedPrecondition, "task run logs have been removed and are no longer available.")
	}

	logFilePath := taskRunLogFilePath(api.config.TaskRunLogsDir,
		request.NamespaceId, request.PipelineId, request.RunId, request.Id)

	file, err := tail.TailFile(logFilePath, tail.Config{Follow: true, Logger: tail.DiscardingLogger})
	if err != nil {
		log.Error().Err(err).
			Str("pipeline", taskRun.Pipeline).Int64("run", taskRun.Run).
			Str("task", taskRun.ID).Msg("error opening task run log file")
		return status.Errorf(codes.Internal, "error opening task run log file: %v", err)
	}

	for {
		select {
		case <-stream.Context().Done():
			_ = file.Stop()
			return nil
		case <-api.context.ctx.Done():
			_ = file.Stop()
			return nil
		case line := <-file.Lines:
			// We insert a special EOF delimiter at the end of each file to signify that there are no more logs to be
			// written. When reading these files from other applications this is an indicator that
			// we have reached the end of the log file and no more logs will be added.
			// In this case when streaming the file back to the client we look out for this marker to understand when
			// to stop the stream.
			if line.Text == GOFEREOF {
				_ = file.Stop()
				return nil
			}

			// Otherwise stream the file line by line to the client
			err = stream.Send(&proto.GetTaskRunLogsResponse{
				LogLine: line.Text,
				LineNum: int64(line.Num),
			})
			if err != nil {
				log.Error().Err(err).Int("line_number", int(line.Num)).
					Str("pipeline", taskRun.Pipeline).Int64("run", taskRun.Run).
					Str("task", taskRun.ID).Msg("error sending log stream to client")
				return status.Errorf(codes.Internal, "error sending log stream: %v", err)
			}
		}
	}
}

func (api *API) DeleteTaskRunLogs(ctx context.Context, request *proto.DeleteTaskRunLogsRequest) (*proto.DeleteTaskRunLogsResponse, error) {
	if request.Id == "" {
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.PipelineId == "" {
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "pipeline required")
	}

	if request.RunId == 0 {
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "run required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeleteTaskRunLogsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	taskRunRaw, err := api.db.GetPipelineTaskRun(api.db, request.NamespaceId, request.PipelineId, request.RunId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "task run not found")
		}
		log.Error().Err(err).Msg("could not get task run")
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.Internal, "failed to retrieve task run from database")
	}

	var taskRun models.TaskRun
	taskRun.FromStorage(&taskRunRaw)

	if taskRun.State != models.TaskRunStateComplete {
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "can not delete logs for a task currently in progress")
	}

	taskRun.LogsRemoved = true

	logFilePath := taskRunLogFilePath(api.config.TaskRunLogsDir, taskRun.Namespace,
		taskRun.Pipeline, taskRun.Run, taskRun.ID)

	err = os.Remove(logFilePath)
	if err != nil {
		return &proto.DeleteTaskRunLogsResponse{}, status.Errorf(codes.Internal, "could not remove task run log file: %v", err)
	}

	err = api.db.UpdatePipelineTaskRun(api.db, taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID,
		storage.UpdatablePipelineTaskRunFields{
			LogsRemoved: ptr(true),
		})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "task run not found")
		}
		log.Error().Err(err).Msg("could not get task run")
		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.Internal, "failed to retrieve task run from database")
	}

	return &proto.DeleteTaskRunLogsResponse{}, nil
}
