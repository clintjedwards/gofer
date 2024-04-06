package models

import (
	"encoding/json"
	"fmt"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

type TaskExecutionState string

const (
	TaskExecutionStateUnknown    TaskExecutionState = "UNKNOWN"    // Unknown state, should never be in this state.
	TaskExecutionStateProcessing TaskExecutionState = "PROCESSING" // Pre-scheduling validation and prep.
	TaskExecutionStateWaiting    TaskExecutionState = "WAITING"    // Waiting to be scheduled.
	TaskExecutionStateRunning    TaskExecutionState = "RUNNING"    // Currently running as reported by scheduler.
	TaskExecutionStateComplete   TaskExecutionState = "COMPLETE"
)

type TaskExecutionStatus string

const (
	TaskExecutionStatusUnknown    TaskExecutionStatus = "UNKNOWN"
	TaskExecutionStatusFailed     TaskExecutionStatus = "FAILED"     // Has encountered an issue, either container issue or scheduling issue.
	TaskExecutionStatusSuccessful TaskExecutionStatus = "SUCCESSFUL" // Finished with a proper error code.
	TaskExecutionStatusCancelled  TaskExecutionStatus = "CANCELLED"  // Cancelled mid run due to user requested cancellation.
	TaskExecutionStatusSkipped    TaskExecutionStatus = "SKIPPED"    // Not run due to dependencies not being met.
)

type TaskExecutionStatusReasonKind string

const (
	TaskExecutionStatusReasonKindUnknown            TaskExecutionStatusReasonKind = "UNKNOWN"             // Unknown state, should never be in this state.
	TaskExecutionStatusReasonKindAbnormalExit       TaskExecutionStatusReasonKind = "ABNORMAL_EXIT"       // A non-zero exit code has been received.
	TaskExecutionStatusReasonKindSchedulerError     TaskExecutionStatusReasonKind = "SCHEDULER_ERROR"     // Encountered an error with the backend scheduler.
	TaskExecutionStatusReasonKindFailedPrecondition TaskExecutionStatusReasonKind = "FAILED_PRECONDITION" // User error in task run parameters.
	TaskExecutionStatusReasonKindCancelled          TaskExecutionStatusReasonKind = "CANCELLED"           // User invoked cancellation.
	TaskExecutionStatusReasonKindOrphaned           TaskExecutionStatusReasonKind = "ORPHANED"            // Task run was lost due to extreme internal error.
)

// A task execution is a specific execution of a task/container.
// It represents a 4th level unit in the hierarchy:
//
//	namespace -> pipeline -> run -> task execution
type TaskExecution struct {
	NamespaceID     string                     `json:"namespace_id" example:"default" doc:"Unique identifier of the target namespace"`
	PipelineID      string                     `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	Version         int64                      `json:"version" example:"42" doc:"Which version of the pipeline did this task execution occur in"`
	RunID           int64                      `json:"run_id" example:"1" doc:"Unique identifier for the target run"`
	TaskExecutionID string                     `json:"task_execution_id" example:"30" doc:"Unique identifier for the target task execution"`
	Created         uint64                     `json:"created" example:"1712433802634" doc:"Time of task execution creation in epoch milliseconds"`
	Started         uint64                     `json:"started" example:"1712433802640" doc:"Time of task execution start in epoch milliseconds"`
	Ended           uint64                     `json:"ended" example:"1712433802640" doc:"Time of task execution completion in epoch milliseconds"`
	ExitCode        *int64                     `json:"exit_code,omitempty" example:"0" doc:"The exit code of the task execution if it is finished"`
	LogsExpired     bool                       `json:"logs_expired" example:"true" doc:"Whether the logs have past their retention time"`
	LogsRemoved     bool                       `json:"logs_removed" example:"true" doc:"If the logs for this execution have been removed. This can be due to user request or automatic action based on expiry time"`
	State           TaskExecutionState         `json:"state" example:"PROCESSING" doc:"What current state of execution is the task execution within. This is a meta status on the progress of the task execution itself."`
	Status          TaskExecutionStatus        `json:"status" example:"SUCCESSFUL" doc:"What is the final end state of the task execution"`
	StatusReason    *TaskExecutionStatusReason `json:"status_reason,omitempty" doc:"More information about the current status"`
	Variables       []Variable                 `json:"variables" doc:"The environment variables injected during this particular task run"`
	Task            Task                       `json:"task" doc:"Information about the underlying task this task execution ran"`
}

type TaskExecutionStatusReason struct {
	Reason      TaskExecutionStatusReasonKind `json:"reason" example:"ABNORMAL_EXIT" doc:"Specific reason type; useful for documentation"`
	Description string                        `json:"description" example:"task exited without an error code of 0" doc:"A humanized description for what occurred"`
}

func (t *TaskExecutionStatusReason) ToJSON() string {
	reason, err := json.Marshal(t)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert extension subscription status reason to json")
	}

	return string(reason)
}

func NewTaskExecution(namespace, pipeline string, version, run int64, task Task) *TaskExecution {
	return &TaskExecution{
		NamespaceID:     namespace,
		PipelineID:      pipeline,
		Version:         version,
		RunID:           run,
		TaskExecutionID: task.ID,
		Created:         uint64(time.Now().UnixMilli()),
		Started:         0,
		Ended:           0,
		ExitCode:        nil,
		StatusReason:    nil,
		LogsExpired:     false,
		LogsRemoved:     false,
		State:           TaskExecutionStateProcessing,
		Status:          TaskExecutionStatusUnknown,
		Variables:       []Variable{},
		Task:            task,
	}
}

func (r *TaskExecution) ToStorage() *storage.PipelineTaskExecution {
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

	taskExecution := &storage.PipelineTaskExecution{
		Namespace:    r.NamespaceID,
		Pipeline:     r.PipelineID,
		Run:          r.RunID,
		ID:           r.TaskExecutionID,
		Task:         string(task),
		Created:      fmt.Sprint(r.Created),
		Started:      fmt.Sprint(r.Started),
		Ended:        fmt.Sprint(r.Ended),
		ExitCode:     exitcode,
		LogsExpired:  r.LogsExpired,
		LogsRemoved:  r.LogsRemoved,
		State:        string(r.State),
		Status:       string(r.Status),
		StatusReason: r.StatusReason.ToJSON(),
		Variables:    string(variables),
	}

	return taskExecution
}

func (r *TaskExecution) FromStorage(storage *storage.PipelineTaskExecution) {
	var statusReason TaskExecutionStatusReason
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

	created, err := strconv.ParseUint(storage.Created, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	started, err := strconv.ParseUint(storage.Started, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	ended, err := strconv.ParseUint(storage.Ended, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	r.NamespaceID = storage.Namespace
	r.PipelineID = storage.Pipeline
	r.RunID = storage.Run
	r.TaskExecutionID = storage.ID
	r.Task = task
	r.Created = created
	r.Started = started
	r.Ended = ended
	r.ExitCode = Ptr(storage.ExitCode)
	r.LogsExpired = storage.LogsExpired
	r.LogsRemoved = storage.LogsRemoved
	r.State = TaskExecutionState(storage.State)
	r.Status = TaskExecutionStatus(storage.Status)
	r.StatusReason = &statusReason
	r.Variables = variables
}
