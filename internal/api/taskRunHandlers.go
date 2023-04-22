package api

import (
	"bufio"
	"context"
	"errors"
	"os"
	"strings"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
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

func (api *API) AttachToTaskRun(stream proto.Gofer_AttachToTaskRunServer) error {
	// Get the first message so we can attempt to set up the connection with the proper docker container.
	initMessageRaw, err := stream.Recv()
	if err != nil {
		log.Error().Err(err).Msg("could not set up stream")
		return status.Errorf(codes.Internal, "could not set up stream: %v", err)
	}

	initMessage, ok := initMessageRaw.RequestType.(*proto.AttachToTaskRunRequest_Init)
	if !ok {
		return status.Error(codes.FailedPrecondition, "first message must be init message, received input message")
	}

	// Validate input
	if initMessage.Init.Id == "" {
		return status.Error(codes.FailedPrecondition, "id required")
	}

	if initMessage.Init.PipelineId == "" {
		return status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if initMessage.Init.RunId == 0 {
		return status.Error(codes.FailedPrecondition, "run id required")
	}

	namespace, err := api.resolveNamespace(stream.Context(), initMessage.Init.NamespaceId)
	if err != nil {
		return status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v",
			initMessage.Init.NamespaceId, err.Error())
	}

	if !hasAccess(stream.Context(), namespace) {
		return status.Error(codes.PermissionDenied, "access denied")
	}

	taskRun, err := api.db.GetPipelineTaskRun(api.db, namespace, initMessage.Init.PipelineId,
		initMessage.Init.RunId, initMessage.Init.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return status.Error(codes.FailedPrecondition, "task run not found")
		}
		log.Error().Err(err).Msg("could not get task run")
		return status.Error(codes.Internal, "failed to retrieve task run from database")
	}

	// Attempt to drop the user into a shell if the user hasn't entered any explicit command
	cmd := []string{"sh"}
	if len(initMessage.Init.Command) != 0 {
		cmd = initMessage.Init.Command
	}

	// A channel to buffer the messages incoming from the container.
	incomingMsgChannel := make(chan string)

	// A general channel that means we should stop what we're doing and cleanly exit.
	stopChan := make(chan struct{})

	resp, err := api.scheduler.AttachContainer(scheduler.AttachContainerRequest{
		ID:      taskContainerID(namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID),
		Command: cmd,
	})
	if err != nil {
		return status.Errorf(codes.Internal, "could not connect to specified container; %v", err)
	}
	defer resp.Conn.Close()

	// Start a goroutine to receive incoming messages from the client and insert them into the container.
	go func() {
		for {
			select {
			case <-stopChan:
				close(incomingMsgChannel)
				return
			case <-stream.Context().Done():
				close(incomingMsgChannel)
				return
			case <-api.context.ctx.Done():
				close(incomingMsgChannel)
				return
			default:
				msgRaw, err := stream.Recv()
				if err != nil {
					// If the context was cancelled, that means that the client abandoned the connect; exit cleanly.
					if strings.Contains(err.Error(), "context canceled") {
						close(incomingMsgChannel)
						return
					}

					// If the client disconnected, exit cleanly.
					if strings.Contains(err.Error(), "client disconnected") {
						close(incomingMsgChannel)
						close(stopChan)
						return
					}

					log.Error().Err(err).Msg("encountered error while streaming messages during task run attach")
					close(incomingMsgChannel)
					close(stopChan)
					return
				}

				msg, ok := msgRaw.RequestType.(*proto.AttachToTaskRunRequest_Input)
				if !ok {
					log.Error().Msg("skipping incorrect message type encountered while streaming messages during task run attach")
					continue
				}

				incomingMsgChannel <- msg.Input.Input
			}
		}
	}()

	taskRunCompletedEvents, err := api.events.Subscribe(events.EventTypeTaskRunCompleted)
	if err != nil {
		// We don't actually have to fail here since the worse that happens is that that user gets
		// a confusing EOF error instead.if err != nil {
		log.Error().Err(err).Str("namespace", namespace).
			Str("pipeline", initMessage.Init.PipelineId).
			Int64("run", initMessage.Init.RunId).
			Str("task_run_id", initMessage.Init.Id).
			Msg("could not listen for task run completed events")
	} else {
		go func() {
			for {
				select {
				case <-stopChan:
					return
				case <-stream.Context().Done():
					return
				case <-api.context.ctx.Done():
					return
				case event := <-taskRunCompletedEvents.Events:
					evt, ok := event.Details.(events.EventTaskRunCompleted)
					if !ok {
						continue
					}

					if evt.NamespaceID == namespace &&
						evt.PipelineID == initMessage.Init.PipelineId &&
						evt.RunID == initMessage.Init.RunId &&
						evt.TaskRunID == initMessage.Init.Id {

						close(stopChan)

						log.Debug().Str("namespace", namespace).
							Str("pipeline", initMessage.Init.PipelineId).
							Int64("run", initMessage.Init.RunId).
							Str("task_run_id", initMessage.Init.Id).Msg("closed task run attach connection due to task run being complete")
						return
					}
				}
			}
		}()
	}

	// Start a goroutine to send messages that we get back from the container to the client.
	// Unfortunately it's a known problem that this leaks goroutines because io.Reader doesn't have a close
	// method.
	//
	// https://benjamincongdon.me/blog/2020/04/23/Cancelable-Reads-in-Go/
	go func() {
		scanner := bufio.NewScanner(resp.Reader)
		scanner.Split(scanWordsWithWhitespace)

		for scanner.Scan() {
			select {
			case <-stopChan:
				return
			case <-stream.Context().Done():
				return
			case <-api.context.ctx.Done():
				return
			default:
				chunk := strings.ToValidUTF8(scanner.Text(), "")

				err := stream.Send(&proto.AttachToTaskRunOutput{
					Output: chunk,
				})
				if err != nil {
					log.Error().Err(err).Str("last_line", chunk).
						Msg("encountered error while sending messages to container during task run attach")
					return
				}
			}
		}

		if err := scanner.Err(); err != nil {
			if strings.Contains(err.Error(), "use of closed network connection") {
				return
			}
			log.Error().Err(err).
				Msg("encountered error while reading messages from container during task run attach")
			return
		}
	}()

	for {
		select {
		case <-stopChan:
			return nil
		case <-stream.Context().Done():
			return nil
		case <-api.context.ctx.Done():
			return nil
		case input := <-incomingMsgChannel:
			_, err := resp.Conn.Write([]byte(input))
			if err != nil {
				log.Error().Err(err).Msg("encountered error while writing messages during task run attach")
				return err
			}
		}
	}
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
