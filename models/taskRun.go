package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
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

func (t *TaskRunStatusReason) ToProto() *proto.TaskRunStatusReason {
	return &proto.TaskRunStatusReason{
		Reason:      proto.TaskRunStatusReason_Reason(proto.TaskRunStatusReason_Reason_value[string(t.Reason)]),
		Description: t.Description,
	}
}

func (t *TaskRunStatusReason) FromProto(proto *proto.TaskRunStatusReason) {
	t.Reason = TaskRunStatusReasonKind(proto.Reason.String())
	t.Description = proto.Description
}

func NewTaskRun(namespace, pipeline string, run int64, task Task) *TaskRun {
	return &TaskRun{
		Namespace:    namespace,
		Pipeline:     pipeline,
		Run:          run,
		ID:           task.GetID(),
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
		Run:          r.Run,
		Id:           r.ID,
		Created:      r.Created,
		Started:      r.Started,
		Ended:        r.Ended,
		ExitCode:     exitCode,
		StatusReason: statusReason,
		LogsExpired:  r.LogsExpired,
		LogsRemoved:  r.LogsRemoved,
		State:        proto.TaskRun_TaskRunState(proto.TaskRun_TaskRunState_value[string(r.State)]),
		Status:       proto.TaskRun_TaskRunStatus(proto.TaskRun_TaskRunStatus_value[string(r.Status)]),
		Variables:    variables,
	}

	switch task := r.Task.(type) {
	case *CommonTask:
		protoTaskRun.Task = &proto.TaskRun_CommonTask{
			CommonTask: task.ToProto(),
		}
	case *CustomTask:
		protoTaskRun.Task = &proto.TaskRun_CustomTask{
			CustomTask: task.ToProto(),
		}
	}

	return protoTaskRun
}

func (r *TaskRun) FromProto(pb *proto.TaskRun) {
	var statusReason *TaskRunStatusReason
	if pb.StatusReason != nil {
		sr := &TaskRunStatusReason{}
		sr.FromProto(pb.StatusReason)
		statusReason = sr
	}

	variables := []Variable{}
	for _, variable := range pb.Variables {
		vari := Variable{}
		vari.FromProto(variable)
		variables = append(variables, vari)
	}

	var task Task

	switch t := pb.Task.(type) {
	case *proto.TaskRun_CommonTask:
		commonTask := &CommonTask{}
		commonTask.FromProto(t.CommonTask)
		task = commonTask
	case *proto.TaskRun_CustomTask:
		customTask := &CustomTask{}
		customTask.FromProto(t.CustomTask)
		task = customTask
	}

	r.Namespace = pb.Namespace
	r.Pipeline = pb.Pipeline
	r.Run = pb.Run
	r.ID = pb.Id
	r.Created = pb.Created
	r.Started = pb.Started
	r.Ended = pb.Ended
	r.ExitCode = Ptr(pb.ExitCode)
	r.StatusReason = statusReason
	r.LogsExpired = pb.LogsExpired
	r.LogsRemoved = pb.LogsRemoved
	r.State = TaskRunState(pb.State.String())
	r.Status = TaskRunStatus(pb.Status.String())
	r.Variables = variables
	r.Task = task
}

func Ptr[T any](v T) *T {
	return &v
}
