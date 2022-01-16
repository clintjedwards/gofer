package models

import (
	"time"

	"github.com/clintjedwards/gofer/proto"
)

type ContainerState string

const (
	ContainerStateUnknown    ContainerState = "UNKNOWN"    // Unknown state, should never be in this state.
	ContainerStateProcessing ContainerState = "PROCESSING" // Pre-scheduling validation and prep.
	ContainerStateWaiting    ContainerState = "WAITING"    // Waiting to be scheduled.
	ContainerStateRunning    ContainerState = "RUNNING"    // Currently running as reported by scheduler.
	ContainerStateFailed     ContainerState = "FAILED"     // Has encountered an issue, either container issue or scheduling issue.
	ContainerStateSuccess    ContainerState = "SUCCESS"    // Finished with a proper error code.
	ContainerStateCancelled  ContainerState = "CANCELLED"  // Cancelled mid run due to user requested cancellation.
	ContainerStateSkipped    ContainerState = "SKIPPED"    // Not run due to dependencies not being met.
)

type TaskRunFailureKind string

const (
	TaskRunFailureKindUnknown            TaskRunFailureKind = "UNKNOWN"             // Unknown state, should never be in this state.
	TaskRunFailureKindAbnormalExit       TaskRunFailureKind = "ABNORMAL_EXIT"       // A non-zero exit code has been received.
	TaskRunFailureKindSchedulerError     TaskRunFailureKind = "SCHEDULER_ERROR"     // Encountered an error with the backend scheduler.
	TaskRunFailureKindFailedPrecondition TaskRunFailureKind = "FAILED_PRECONDITION" // User error in task run parameters.
	TaskRunFailureKindCancelled          TaskRunFailureKind = "CANCELLED"           // User invoked cancellation.
	TaskRunFailureKindOrphaned           TaskRunFailureKind = "ORPHANED"            // Task run was lost due to extreme internal error.
)

// TaskRun is a specific execution of a task/container.
// It represents a 4th level unit in the hierarchy: namespace -> pipeline -> run -> taskrun.
type TaskRun struct {
	Created     int64          `json:"created" storm:"index"` // Time of task run creation in epoch milliseconds.
	Ended       int64          `json:"ended"`                 // Time of task run completion in epoch milliseconds.
	ExitCode    int            `json:"exit_code"`             // The exit code of the task run.
	Failure     TaskRunFailure `json:"failure"`               // Detailed reasoning on task run failure.
	ID          string         `json:"id" storm:"id"`         // Unique ID for task run; taken from the taskID.
	LogsExpired bool           `json:"logs_expired"`          // If the logs have past their retention time.

	// If the logs have been removed. This can be due to user request or automatic action based on expiry time.
	LogsRemoved bool   `json:"logs_removed"`
	NamespaceID string `json:"namespace_id"` // Unique identifier for namespace.
	PipelineID  string `json:"pipeline_id"`  // Unique pipeline ID of task run.
	RunID       int64  `json:"run_id"`       // Unique run ID of task run; sequential.

	// The identifier used by the scheduler to identify this specific task run container. This is provided by the
	// scheduler.
	SchedulerID string         `json:"scheduler_id"`
	Started     int64          `json:"started" storm:"index"` // Time of task run actual start in epoch milliseconds.
	State       ContainerState `json:"status"`
	Task        `json:"task"`  // Task information.
}

type TaskRunFailure struct {
	Kind        TaskRunFailureKind `json:"kind"`        // Specific failure type; useful for documentation.
	Description string             `json:"description"` // Details on why the task run failed.
}

func NewTaskRun(run Run, task Task) *TaskRun {
	return &TaskRun{
		ID:          task.ID,
		Created:     time.Now().UnixMilli(),
		State:       ContainerStateProcessing,
		RunID:       run.ID,
		PipelineID:  run.PipelineID,
		NamespaceID: run.NamespaceID,
		Task:        task,
	}
}

// IsComplete returns whether the task run has completed and no further state changes will be made.
func (r *TaskRun) IsComplete() bool {
	if r.State == ContainerStateFailed ||
		r.State == ContainerStateSuccess ||
		r.State == ContainerStateCancelled ||
		r.State == ContainerStateSkipped ||
		r.State == ContainerStateUnknown {
		return true
	}

	return false
}

func (r *TaskRun) SetFinishedAbnormal(state ContainerState, failure TaskRunFailure, code int) {
	r.ExitCode = code
	r.State = state
	r.Ended = time.Now().UnixMilli()
	r.Failure = failure
}

func (r *TaskRun) SetFinishedSuccess() {
	r.ExitCode = 0
	r.State = ContainerStateSuccess
	r.Ended = time.Now().UnixMilli()
}

func (r *TaskRun) ToProto() *proto.TaskRun {
	protoFailure := proto.TaskRunFailure{
		Kind:        proto.TaskRunFailure_Kind(proto.TaskRunFailure_Kind_value[string(r.Failure.Kind)]),
		Description: r.Failure.Description,
	}

	dependsOn := map[string]proto.TaskRequiredParentState{}
	for key, value := range r.DependsOn {
		dependsOn[key] = proto.TaskRequiredParentState(proto.TaskRequiredParentState_value[string(value)])
	}

	return &proto.TaskRun{
		Ended:       r.Ended,
		ExitCode:    int64(r.ExitCode),
		Created:     r.Created,
		Failure:     &protoFailure,
		Id:          r.ID,
		LogsExpired: r.LogsExpired,
		LogsRemoved: r.LogsRemoved,
		PipelineId:  r.PipelineID,
		NamespaceId: r.NamespaceID,
		RunId:       r.RunID,
		SchedulerId: r.SchedulerID,
		Started:     r.Started,
		State:       proto.TaskRun_State(proto.TaskRun_State_value[string(r.State)]),
		Task:        r.Task.ToProto(),
	}
}

func (r *TaskRun) FromProto(proto *proto.TaskRun) {
	failure := TaskRunFailure{
		Kind:        TaskRunFailureKind(proto.Failure.Kind.String()),
		Description: proto.Failure.Description,
	}

	dependsOn := map[string]RequiredParentState{}
	for key, value := range proto.Task.DependsOn {
		dependsOn[key] = RequiredParentState(value.String())
	}

	r.Ended = proto.Ended
	r.ExitCode = int(proto.ExitCode)
	r.Created = proto.Created
	r.Failure = failure
	r.ID = proto.Id
	r.LogsExpired = proto.LogsExpired
	r.LogsRemoved = proto.LogsRemoved
	r.PipelineID = proto.PipelineId
	r.NamespaceID = proto.NamespaceId
	r.RunID = proto.RunId
	r.SchedulerID = proto.SchedulerId
	r.Started = proto.Started
	r.State = ContainerState(proto.State.String())
	r.ID = proto.Task.Id
	r.Description = proto.Task.Description
	r.Image = proto.Task.Image
	r.DependsOn = dependsOn
	r.EnvVars = proto.Task.EnvVars
}
