package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

type TaskRunState string

const (
	TaskRunStateUnknown    TaskRunState = "UNKNOWN"    // Unknown state, should never be in this state.
	TaskRunStateProcessing TaskRunState = "PROCESSING" // Pre-scheduling validation and prep.
	TaskRunStateWaiting    TaskRunState = "WAITING"    // Waiting to be scheduled.
	TaskRunStateRunning    TaskRunState = "RUNNING"    // Currently running as reported by scheduler.
	TaskRunStateComplete   TaskRunState = "COMPLETE"
)

type TaskRunStatus string

const (
	TaskRunStatusUnknown    TaskRunStatus = "UNKNOWN"
	TaskRunStatusFailed     TaskRunStatus = "FAILED"     // Has encountered an issue, either container issue or scheduling issue.
	TaskRunStatusSuccessful TaskRunStatus = "SUCCESSFUL" // Finished with a proper error code.
	TaskRunStatusCancelled  TaskRunStatus = "CANCELLED"  // Cancelled mid run due to user requested cancellation.
	TaskRunStatusSkipped    TaskRunStatus = "SKIPPED"    // Not run due to dependencies not being met.
)

type TaskRunStatusReasonKind string

const (
	TaskRunStatusReasonKindUnknown            TaskRunStatusReasonKind = "UNKNOWN"             // Unknown state, should never be in this state.
	TaskRunStatusReasonKindAbnormalExit       TaskRunStatusReasonKind = "ABNORMAL_EXIT"       // A non-zero exit code has been received.
	TaskRunStatusReasonKindSchedulerError     TaskRunStatusReasonKind = "SCHEDULER_ERROR"     // Encountered an error with the backend scheduler.
	TaskRunStatusReasonKindFailedPrecondition TaskRunStatusReasonKind = "FAILED_PRECONDITION" // User error in task run parameters.
	TaskRunStatusReasonKindCancelled          TaskRunStatusReasonKind = "CANCELLED"           // User invoked cancellation.
	TaskRunStatusReasonKindOrphaned           TaskRunStatusReasonKind = "ORPHANED"            // Task run was lost due to extreme internal error.
)

// A task run is a specific execution of a task/container.
// It represents a 4th level unit in the hierarchy:
//
//	namespace -> pipeline -> run -> taskrun
type TaskRun struct {
	Namespace   string `json:"namespace"`    // Unique identifier for namespace.
	Pipeline    string `json:"pipeline"`     // Unique pipeline ID of task run.
	Version     int64  `json:"version"`      // Which version of the pipeline did this task run occur in.
	Run         int64  `json:"run"`          // Unique run ID of task run; sequential.
	ID          string `json:"id"`           // Unique ID for task run; taken from the taskID.
	Created     int64  `json:"created"`      // Time of task run creation in epoch milliseconds.
	Started     int64  `json:"started"`      // Time of task run actual start in epoch milliseconds.
	Ended       int64  `json:"ended"`        // Time of task run completion in epoch milliseconds.
	ExitCode    *int64 `json:"exit_code"`    // The exit code of the task run.
	LogsExpired bool   `json:"logs_expired"` // If the logs have past their retention time.
	// If the logs have been removed. This can be due to user request or automatic action based on expiry time.
	LogsRemoved  bool                 `json:"logs_removed"`
	State        TaskRunState         `json:"state"`
	Status       TaskRunStatus        `json:"status"`
	StatusReason *TaskRunStatusReason `json:"status_reason"` // Extra information about the current status.
	Variables    []Variable           `json:"variables"`     // The environment variables injected during this particular task run.
	Task         Task                 `json:"task"`          // Task information.
}

type TaskRunStatusReason struct {
	Reason      TaskRunStatusReasonKind `json:"reason"`      // Specific type; useful for documentation.
	Description string                  `json:"description"` // Details about type.
}

func (t *TaskRunStatusReason) ToJSON() string {
	reason, err := json.Marshal(t)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert extension subscription status reason to json")
	}

	return string(reason)
}

func (t *TaskRunStatusReason) ToProto() *proto.TaskRunStatusReason {
	return &proto.TaskRunStatusReason{
		Reason:      proto.TaskRunStatusReason_Reason(proto.TaskRunStatusReason_Reason_value[string(t.Reason)]),
		Description: t.Description,
	}
}

func NewTaskRun(namespace, pipeline string, version, run int64, task Task) *TaskRun {
	return &TaskRun{
		Namespace:    namespace,
		Pipeline:     pipeline,
		Version:      version,
		Run:          run,
		ID:           task.ID,
		Created:      time.Now().UnixMilli(),
		Started:      0,
		Ended:        0,
		ExitCode:     nil,
		StatusReason: nil,
		LogsExpired:  false,
		LogsRemoved:  false,
		State:        TaskRunStateProcessing,
		Status:       TaskRunStatusUnknown,
		Variables:    []Variable{},
		Task:         task,
	}
}

func (r *TaskRun) ToProto() *proto.TaskRun {
	variables := []*proto.Variable{}
	for _, variable := range r.Variables {
		variables = append(variables, variable.ToProto())
	}

	var statusReason *proto.TaskRunStatusReason
	if r.StatusReason != nil {
		statusReason = r.StatusReason.ToProto()
	}

	var exitCode int64 = 155
	if r.ExitCode != nil {
		exitCode = *r.ExitCode
	}

	protoTaskRun := &proto.TaskRun{
		Namespace:    r.Namespace,
		Pipeline:     r.Pipeline,
		Version:      r.Version,
		Run:          r.Run,
		Id:           r.ID,
		Created:      r.Created,
		Started:      r.Started,
		Ended:        r.Ended,
		Task:         r.Task.ToProto(),
		ExitCode:     exitCode,
		StatusReason: statusReason,
		LogsExpired:  r.LogsExpired,
		LogsRemoved:  r.LogsRemoved,
		State:        proto.TaskRun_TaskRunState(proto.TaskRun_TaskRunState_value[string(r.State)]),
		Status:       proto.TaskRun_TaskRunStatus(proto.TaskRun_TaskRunStatus_value[string(r.Status)]),
		Variables:    variables,
	}

	return protoTaskRun
}

func (r *TaskRun) ToStorage() *storage.PipelineTaskRun {
	task, err := json.Marshal(r.Task)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	variables, err := json.Marshal(r.Variables)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var exitcode int64 = 999
	if r.ExitCode != nil {
		exitcode = *r.ExitCode
	}

	taskRun := &storage.PipelineTaskRun{
		Namespace:    r.Namespace,
		Pipeline:     r.Pipeline,
		Run:          r.Run,
		ID:           r.ID,
		Task:         string(task),
		Created:      r.Created,
		Started:      r.Started,
		Ended:        r.Ended,
		ExitCode:     exitcode,
		LogsExpired:  r.LogsExpired,
		LogsRemoved:  r.LogsRemoved,
		State:        string(r.State),
		Status:       string(r.Status),
		StatusReason: r.StatusReason.ToJSON(),
		Variables:    string(variables),
	}

	return taskRun
}

func (r *TaskRun) FromStorage(storage *storage.PipelineTaskRun) {
	var statusReason TaskRunStatusReason
	err := json.Unmarshal([]byte(storage.StatusReason), &statusReason)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var task Task
	err = json.Unmarshal([]byte(storage.Task), &task)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var variables []Variable
	err = json.Unmarshal([]byte(storage.Variables), &variables)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	r.Namespace = storage.Namespace
	r.Pipeline = storage.Pipeline
	r.Run = storage.Run
	r.ID = storage.ID
	r.Task = task
	r.Created = storage.Created
	r.Started = storage.Started
	r.Ended = storage.Ended
	r.ExitCode = Ptr(storage.ExitCode)
	r.LogsExpired = storage.LogsExpired
	r.LogsRemoved = storage.LogsRemoved
	r.State = TaskRunState(storage.State)
	r.Status = TaskRunStatus(storage.Status)
	r.StatusReason = &statusReason
	r.Variables = variables
}

func Ptr[T any](v T) *T {
	return &v
}
