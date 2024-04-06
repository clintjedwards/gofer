package api

import (
	"bufio"
	"context"
	"errors"
	"net/http"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/danielgtaylor/huma/v2"
	"github.com/gorilla/websocket"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"

	"github.com/rs/zerolog/log"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		// Allow all connections by default
		return true
	},
}

// cancelTaskRun calls upon the scheduler to terminate a specific container. The urgency of this request is
// controlled by the force parameter. Normally scheduler will simply send a SIGTERM and wait for a
// graceful exit and on force they will instead send a SIGKILL.
// The associated timeout controls how long the containers are waited upon until they are sent a SIGKILL.
func (apictx *APIContext) cancelTaskExecution(taskExecution *models.TaskExecution, force bool) error {
	timeout := apictx.config.TaskRunStopTimeout

	if force {
		timeout = time.Millisecond * 500
	}

	containerID := taskContainerID(taskExecution.NamespaceID, taskExecution.PipelineID, taskExecution.RunID, taskExecution.TaskExecutionID)

	err := apictx.scheduler.StopContainer(scheduler.StopContainerRequest{
		ID:      containerID,
		Timeout: timeout,
	})
	if err != nil {
		return err
	}

	return nil
}

// scanWordsWithWhitespace is a split function for a Scanner that returns each
// space-separated word of text. The definition of space is set by unicode.IsSpace.
func scanWordsWithWhitespace(data []byte, atEOF bool) (advance int, token []byte, err error) {
	start := 0

	// Scan until space, marking end of word.
	for width, i := 0, start; i < len(data); i += width {
		var r rune
		r, width = utf8.DecodeRune(data[i:])
		if isSpace(r) {
			return i + width, data[start : i+1], nil
		}
	}

	// If we're at EOF, we have a final, non-empty, non-terminated word. Return it.
	if atEOF && len(data) > start {
		return len(data), data[start:], nil
	}

	// Request more data.
	return start, nil, nil
}

// isSpace reports whether the character is a Unicode white space character.
// We avoid dependency on the unicode package, but check validity of the implementation
// in the tests.
func isSpace(r rune) bool {
	if r <= '\u00FF' {
		// Obvious ASCII ones: \t through \r plus space. Plus two Latin-1 oddballs.
		switch r {
		case ' ', '\t', '\n', '\v', '\f', '\r':
			return true
		case '\u0085', '\u00A0':
			return true
		}
		return false
	}
	// High-valued ones.
	if '\u2000' <= r && r <= '\u200a' {
		return true
	}
	switch r {
	case '\u1680', '\u2028', '\u2029', '\u202f', '\u205f', '\u3000':
		return true
	}
	return false
}

type DescribeTaskExecutionRequest struct {
	Auth            string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	NamespaceID     string `query:"namespace" example:"example_namespace" default:"default" doc:"Unique identifier of the target namespace"`
	PipelineID      string `path:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	RunID           int64  `path:"run_id" example:"1" doc:"Unique identifier for the target run"`
	TaskExecutionID string `path:"task_execution_id" example:"3" doc:"Unique identifier for the target task execution"`
}
type DescribeTaskExecutionResponse struct {
	Body struct {
		TaskExecution *models.TaskExecution `json:"task_execution" doc:"Metadata for the task execution requested"`
	}
}

func (apictx *APIContext) registerDescribeTaskExecution(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DescribeTaskExecution",
		Method:      http.MethodGet,
		Path:        "/api/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_execution_id}",
		Summary:     "Retrieve information on a specific task execution",
		Description: "Task executions are just summaries of what happened on any particular run at the task level. This " +
			"endpoint allows users to retrieve task execution information about a targeted execution along with details on " +
			"the task itself",
		Tags: []string{"Tasks"},
		// Handler //
	}, func(ctx context.Context, request *DescribeTaskExecutionRequest) (*DescribeTaskExecutionResponse, error) {
		if !hasAccess(ctx, request.NamespaceID) {
			return nil, huma.NewError(http.StatusUnauthorized, "access denied")
		}

		taskExecutionRaw, err := apictx.db.GetPipelineTaskExecution(apictx.db, request.NamespaceID, request.PipelineID, request.RunID, request.TaskExecutionID)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return nil, huma.NewError(http.StatusBadRequest, "task run not found")
			}
			log.Error().Err(err).Msg("could not get run")
			return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve task run from database")
		}

		var taskExecution models.TaskExecution
		taskExecution.FromStorage(&taskExecutionRaw)
		resp := &DescribeTaskExecutionResponse{}
		resp.Body.TaskExecution = &taskExecution

		return resp, nil
	})
}

type ListTaskExecutionsRequest struct {
	Auth        string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	NamespaceID string `query:"namespace" example:"example_namespace" default:"default" doc:"Unique identifier of the target namespace"`

	PipelineID string `path:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	RunID      int64  `path:"run_id" example:"1" doc:"Unique identifier for the target run"`
}
type ListTaskExecutionsResponse struct {
	Body struct {
		TaskExecutions []*models.TaskExecution `json:"task_executions" doc:"List of metadata for the task executions requested"`
	}
}

func (apictx *APIContext) registerListTaskExecutions(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "ListTaskExecutions",
		Method:      http.MethodGet,
		Path:        "/api/pipelines/{pipeline_id}/runs/{run_id}/tasks",
		Summary:     "List all task executions",
		Description: "List all task executions for a specific run",
		Tags:        []string{"Tasks"},
		// Handler //
	}, func(ctx context.Context, request *ListTaskExecutionsRequest) (*ListTaskExecutionsResponse, error) {
		if !hasAccess(ctx, request.NamespaceID) {
			return nil, huma.NewError(http.StatusUnauthorized, "access denied")
		}

		taskExecutionsRaw, err := apictx.db.ListPipelineTaskExecutions(apictx.db, 0, 0, request.NamespaceID, request.PipelineID, request.RunID)
		if err != nil {
			log.Error().Err(err).Msg("could not get task runs")
			return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve executions from database", err)
		}

		taskExecutions := []*models.TaskExecution{}
		for _, taskExecutionRaw := range taskExecutionsRaw {
			var taskExecution models.TaskExecution
			taskExecution.FromStorage(&taskExecutionRaw)
			taskExecutions = append(taskExecutions, &taskExecution)
		}

		resp := &ListTaskExecutionsResponse{}
		resp.Body.TaskExecutions = taskExecutions

		return resp, nil
	})
}

type CancelTaskExecutionRequest struct {
	Auth        string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	NamespaceID string `query:"namespace" example:"example_namespace" default:"default" doc:"Unique identifier of the target namespace"`

	PipelineID      string `path:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	RunID           int64  `path:"run_id" example:"1" doc:"Unique identifier for the target run"`
	TaskExecutionID string `path:"task_execution_id" example:"3" doc:"Unique identifier for the target task execution"`

	Body struct {
		Force bool `json:"force" example:"true" default:"false" doc:"Causes Gofer to hard kill this task execution's container. Usually this means the container receives a SIGKILL"`
	}
}
type CancelTaskExecutionResponse struct{}

func (apictx *APIContext) registerCancelTaskExecution(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "CancelTaskExecution",
		Method:      http.MethodPost,
		Path:        "/api/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_execution_id}/cancel",
		Summary:     "Cancel a specific task execution",
		Description: "Cancels a specific task execution, sending the related container a SIGINT signal. If the force " +
			"flag is used we instead send the container a SIGKILL signal." + "\n" + "Task executions that are cancelled can " +
			"cause other downstream task executions to be skipped depending on those downstream task execution dependencies.",
		Tags: []string{"Tasks"},
		// Handler //
	}, func(ctx context.Context, request *CancelTaskExecutionRequest) (*CancelTaskExecutionResponse, error) {
		if !hasAccess(ctx, request.NamespaceID) {
			return nil, huma.NewError(http.StatusUnauthorized, "access denied")
		}

		taskExecutionRaw, err := apictx.db.GetPipelineTaskExecution(apictx.db, request.NamespaceID, request.PipelineID, request.RunID, request.TaskExecutionID)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return nil, huma.NewError(http.StatusBadRequest, "task run not found")
			}
			log.Error().Err(err).Msg("could not get run")
			return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve task run from database")
		}

		var taskExecution models.TaskExecution
		taskExecution.FromStorage(&taskExecutionRaw)

		err = apictx.cancelTaskExecution(&taskExecution, request.Body.Force)
		if err != nil {
			return nil, huma.NewError(http.StatusInternalServerError, "could not cancel container")
		}

		return nil, nil
	})
}

type AttachToTaskExecutionRequest struct {
	Auth        string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	NamespaceID string `query:"namespace" example:"example_namespace" default:"default" doc:"Unique identifier of the target namespace"`

	PipelineID      string `path:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	RunID           int64  `path:"run_id" example:"1" doc:"Unique identifier for the target run"`
	TaskExecutionID string `path:"task_execution_id" example:"3" doc:"Unique identifier for the target task execution"`

	Body struct {
		Command []string `json:"command,omitempty" example:"[\"ls\", \"-al\"]" default:"[\"sh\"]" doc:"Which command to execute first in the container. Normally you want this to be a shell process, enabling you to interact with the container."`
	}
}

type AttachToTaskExecutionResponse struct{}

func (apictx *APIContext) attachToTaskExecutionHandler(w http.ResponseWriter, req *http.Request) {
	if !hasAccess(req.Context(), request.NamespaceID) {
		return nil, huma.NewError(http.StatusUnauthorized, "access denied")
	}

	taskExecution, err := apictx.db.GetPipelineTaskExecution(apictx.db, request.NamespaceID, request.PipelineID,
		request.RunID, request.TaskExecutionID)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return nil, huma.NewError(http.StatusBadRequest, "task run not found")
		}
		log.Error().Err(err).Msg("could not get task run")
		return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve task run from database")
	}

	cmd := request.Body.Command

	// A channel to buffer the messages incoming from the container.
	incomingMsgChannel := make(chan string)

	// A general channel that means we should stop what we're doing and cleanly exit.
	stopChan := make(chan struct{})

	resp, err := apictx.scheduler.AttachContainer(scheduler.AttachContainerRequest{
		ID:      taskContainerID(taskExecution.Namespace, taskExecution.Pipeline, taskExecution.Run, taskExecution.ID),
		Command: cmd,
	})
	if err != nil {
		return nil, huma.NewError(http.StatusInternalServerError, "could not connect to specified container", err)
	}
	defer resp.Conn.Close()

	// Upgrade connection to a websocket connection so we can stream between the client and server
	conn, err := upgrader.Upgrade(request.writer, request.reader, nil) // error ignored for sake of simplicity
	if err != nil {
		return nil, huma.NewError(http.StatusInternalServerError, "could not upgrade websocket connection to communicate between client and server", err)
	}

	// Start a goroutine to receive incoming messages from the client and insert them into the container.
	go func() {
		for {
			select {
			case <-stopChan:
				close(incomingMsgChannel)
				return
			case <-ctx.Done():
				close(incomingMsgChannel)
				return
			case <-apictx.context.ctx.Done():
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

	taskRunCompletedEvents, err := apictx.events.Subscribe(events.EventTypeTaskRunCompleted)
	if err != nil {
		// We don't actually have to fail here since the worse that happens is that that user gets
		// a confusing EOF error instead.
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
				case <-apictx.context.ctx.Done():
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
			case <-apictx.context.ctx.Done():
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
		case <-apictx.context.ctx.Done():
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

// func (apictx *APIContext) registerAttachToTaskExecution(apiDesc huma.API) {
// 	apiDesc.OpenAPI.Webhooks["AttachToTaskExecution"] = &huma.PathItem{
// 		Summary:     "Attach to a running task container",
// 		Description: "Attach to a running task container. Useful for debugging.",
// 	}

// // Description //
// huma.Register(apiDesc, huma.Operation{
// 	OperationID: "AttachToTaskExecution",
// 	Method:      http.MethodPost,
// 	Path:        "/api/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_execution_id}/attach",
// 	Summary:     "Attach to a running task container",
// 	Description: "Attach to a running task container. Useful for debugging.",
// 	Tags:        []string{"Tasks"},
// 	// Handler //
// }, func(ctx context.Context, _ *AttachToTaskExecutionRequest) (*AttachToTaskExecutionResponse, error) {
// 	return nil, nil
// })
// }

// func (apictx *APIContext) GetTaskRunLogs(request *proto.GetTaskRunLogsRequest, stream proto.Gofer_GetTaskRunLogsServer) error {
// 	if request.Id == "" {
// 		return status.Error(codes.FailedPrecondition, "id required")
// 	}

// 	if request.PipelineId == "" {
// 		return status.Error(codes.FailedPrecondition, "pipeline required")
// 	}

// 	if request.RunId == 0 {
// 		return status.Error(codes.FailedPrecondition, "run required")
// 	}

// 	namespace, err := apictx.resolveNamespace(stream.Context(), request.NamespaceId)
// 	if err != nil {
// 		return status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
// 	}

// 	request.NamespaceId = namespace

// 	taskRun, err := apictx.db.GetPipelineTaskRun(apictx.db, request.NamespaceId, request.PipelineId, request.RunId, request.Id)
// 	if err != nil {
// 		if errors.Is(err, storage.ErrEntityNotFound) {
// 			return status.Error(codes.FailedPrecondition, "task run not found")
// 		}
// 		log.Error().Err(err).Msg("could not get task run")
// 		return status.Error(codes.Internal, "failed to retrieve task run from database")
// 	}

// 	if taskRun.LogsExpired {
// 		return status.Error(codes.FailedPrecondition, "task run logs have expired and are no longer available.")
// 	}

// 	if taskRun.LogsRemoved {
// 		return status.Error(codes.FailedPrecondition, "task run logs have been removed and are no longer available.")
// 	}

// 	logFilePath := taskRunLogFilePath(apictx.config.TaskRunLogsDir,
// 		request.NamespaceId, request.PipelineId, request.RunId, request.Id)

// 	file, err := tail.TailFile(logFilePath, tail.Config{Follow: true, Logger: tail.DiscardingLogger})
// 	if err != nil {
// 		log.Error().Err(err).
// 			Str("pipeline", taskRun.Pipeline).Int64("run", taskRun.Run).
// 			Str("task", taskRun.ID).Msg("error opening task run log file")
// 		return status.Errorf(codes.Internal, "error opening task run log file: %v", err)
// 	}

// 	for {
// 		select {
// 		case <-stream.Context().Done():
// 			_ = file.Stop()
// 			return nil
// 		case <-apictx.context.ctx.Done():
// 			_ = file.Stop()
// 			return nil
// 		case line := <-file.Lines:
// 			// We insert a special EOF delimiter at the end of each file to signify that there are no more logs to be
// 			// written. When reading these files from other applications this is an indicator that
// 			// we have reached the end of the log file and no more logs will be added.
// 			// In this case when streaming the file back to the client we look out for this marker to understand when
// 			// to stop the stream.
// 			if line.Text == GOFEREOF {
// 				_ = file.Stop()
// 				return nil
// 			}

// 			// Otherwise stream the file line by line to the client
// 			err = stream.Send(&proto.GetTaskRunLogsResponse{
// 				LogLine: line.Text,
// 				LineNum: int64(line.Num),
// 			})
// 			if err != nil {
// 				log.Error().Err(err).Int("line_number", int(line.Num)).
// 					Str("pipeline", taskRun.Pipeline).Int64("run", taskRun.Run).
// 					Str("task", taskRun.ID).Msg("error sending log stream to client")
// 				return status.Errorf(codes.Internal, "error sending log stream: %v", err)
// 			}
// 		}
// 	}
// }

// func (apictx *APIContext) DeleteTaskRunLogs(ctx context.Context, request *proto.DeleteTaskRunLogsRequest) (*proto.DeleteTaskRunLogsResponse, error) {
// 	if request.Id == "" {
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "id required")
// 	}

// 	if request.PipelineId == "" {
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "pipeline required")
// 	}

// 	if request.RunId == 0 {
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "run required")
// 	}

// 	namespace, err := apictx.resolveNamespace(ctx, request.NamespaceId)
// 	if err != nil {
// 		return &proto.DeleteTaskRunLogsResponse{},
// 			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
// 	}

// 	request.NamespaceId = namespace

// 	if !hasAccess(ctx, request.NamespaceId) {
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.PermissionDenied, "access denied")
// 	}

// 	taskRunRaw, err := apictx.db.GetPipelineTaskRun(apictx.db, request.NamespaceId, request.PipelineId, request.RunId, request.Id)
// 	if err != nil {
// 		if errors.Is(err, storage.ErrEntityNotFound) {
// 			return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "task run not found")
// 		}
// 		log.Error().Err(err).Msg("could not get task run")
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.Internal, "failed to retrieve task run from database")
// 	}

// 	var taskRun models.TaskRun
// 	taskRun.FromStorage(&taskRunRaw)

// 	if taskRun.State != models.TaskRunStateComplete {
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "can not delete logs for a task currently in progress")
// 	}

// 	taskRun.LogsRemoved = true

// 	logFilePath := taskRunLogFilePath(apictx.config.TaskRunLogsDir, taskRun.Namespace,
// 		taskRun.Pipeline, taskRun.Run, taskRun.ID)

// 	err = os.Remove(logFilePath)
// 	if err != nil {
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Errorf(codes.Internal, "could not remove task run log file: %v", err)
// 	}

// 	err = apictx.db.UpdatePipelineTaskRun(apictx.db, taskRun.Namespace, taskRun.Pipeline, taskRun.Run, taskRun.ID,
// 		storage.UpdatablePipelineTaskRunFields{
// 			LogsRemoved: ptr(true),
// 		})
// 	if err != nil {
// 		if errors.Is(err, storage.ErrEntityNotFound) {
// 			return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.FailedPrecondition, "task run not found")
// 		}
// 		log.Error().Err(err).Msg("could not get task run")
// 		return &proto.DeleteTaskRunLogsResponse{}, status.Error(codes.Internal, "failed to retrieve task run from database")
// 	}

// 	return &proto.DeleteTaskRunLogsResponse{}, nil
// }
