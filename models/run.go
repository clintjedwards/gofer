package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type RunState string

const (
	RunStateUnknown RunState = "UNKNOWN" // The state of the run is unknown.
	// Before the tasks in a run is sent to a scheduler it must complete various steps like
	// validation checking. This state represents that step where the run and task_runs are
	// pre-checked.
	RunStatePending  RunState = "PENDING"
	RunStateRunning  RunState = "RUNNING"  // Currently running.
	RunStateComplete RunState = "COMPLETE" // All tasks have been resolved and the run is no longer being executed.

)

type RunStatus string

const (
	// Could not determine current state of the status. Should only be in this state if
	// the run has not yet completed.
	RunStatusUnknown    RunStatus = "UNKNOWN"
	RunStatusFailed     RunStatus = "FAILED"     // One or more tasks in run have failed.
	RunStatusSuccessful RunStatus = "SUCCESSFUL" // All tasks in run have completed with a non-failure state.
	RunStatusCancelled  RunStatus = "CANCELLED"  // One or more tasks in a run have been cancelled.
)

type RunStatusReasonKind string

const (
	// Gofer has no idea who the run got into this state.
	RunStatusReasonKindUnknown RunStatusReasonKind = "UNKNOWN"
	// While executing the run one or more tasks exited with an abnormal exit code.
	RunStatusReasonKindAbnormalExit RunStatusReasonKind = "ABNORMAL_EXIT"
	// While executing the run one or more tasks could not be scheduled.
	RunStatusReasonKindSchedulerError RunStatusReasonKind = "SCHEDULER_ERROR"
	// The run could not be executed as requested due to user defined attributes given.
	RunStatusReasonKindFailedPrecondition RunStatusReasonKind = "FAILED_PRECONDITION"
	// One or more tasks could not be completed due to a user cancelling the run.
	RunStatusReasonKindUserCancelled RunStatusReasonKind = "USER_CANCELLED"
	// One or more tasks could not be completed due to the system or admin cancelling the run.
	RunStatusReasonKindAdminCancelled RunStatusReasonKind = "ADMIN_CANCELLED"
)

// Information about which trigger was responsible for the run's execution.
type TriggerInfo struct {
	Name string // The trigger kind responsible for starting the run.
	// The trigger label responsible for starting the run. The label is a user chosen name
	// for the trigger to differentiate it from other pipeline triggers of the same kind.
	Label string
}

func (t *TriggerInfo) ToProto() *proto.Run_RunTriggerInfo {
	return &proto.Run_RunTriggerInfo{
		Name:  t.Name,
		Label: t.Label,
	}
}

func (t *TriggerInfo) FromProto(proto *proto.Run_RunTriggerInfo) {
	t.Name = proto.Name
	t.Label = proto.Label
}

// A run is one or more tasks being executed on behalf of some trigger.
// Run is a third level unit containing tasks and being contained in a pipeline.
type Run struct {
	Namespace           string           `json:"namespace"`             // Unique ID of namespace.
	Pipeline            string           `json:"pipeline"`              // The unique ID of the related pipeline.
	ID                  int64            `json:"id"`                    // UniqueID of a run. Auto-incrementing and cannot be zero.
	Started             int64            `json:"started"`               // Time of run start in epoch milli.
	Ended               int64            `json:"ended"`                 // Time of run finish in epoch milli.
	State               RunState         `json:"state"`                 // The current state of the run.
	Status              RunStatus        `json:"status"`                // The current status of the run.
	StatusReason        *RunStatusReason `json:"status_reason"`         // Contains more information about a run's current status.
	TaskRuns            []string         `json:"task_runs"`             // The unique ID of each task run.
	Trigger             TriggerInfo      `json:"trigger"`               // Information about which trigger was responsible for the run's execution.
	Variables           []Variable       `json:"variables"`             // Environment variables to be injected into each child task run. These are usually injected by the trigger.
	StoreObjectsExpired bool             `json:"store_objects_expired"` // Tracks whether objects for this run have expired already.
}

type RunStatusReason struct {
	Reason      RunStatusReasonKind `json:"kind"`        // The specific type of run failure. Good for documentation about what it might be.
	Description string              `json:"description"` // The description of why the run might have failed.
}

func (r *RunStatusReason) ToProto() *proto.RunStatusReason {
	return &proto.RunStatusReason{
		Reason:      proto.RunStatusReason_RunStatusReasonKind(proto.RunStatusReason_RunStatusReasonKind_value[string(r.Reason)]),
		Description: r.Description,
	}
}

func (r *RunStatusReason) FromProto(proto *proto.RunStatusReason) {
	r.Reason = RunStatusReasonKind(proto.Reason.String())
	r.Description = proto.Description
}

func NewRun(namespace, pipeline string, trigger TriggerInfo, variables []Variable) *Run {
	return &Run{
		Namespace:           namespace,
		Pipeline:            pipeline,
		ID:                  0,
		Started:             time.Now().UnixMilli(),
		Ended:               0,
		State:               RunStatePending,
		Status:              RunStatusUnknown,
		StatusReason:        nil,
		TaskRuns:            []string{},
		Trigger:             trigger,
		Variables:           variables,
		StoreObjectsExpired: false,
	}
}

func (r *Run) ToProto() *proto.Run {
	variables := []*proto.Variable{}
	for _, variable := range r.Variables {
		variables = append(variables, variable.ToProto())
	}

	var statusReason *proto.RunStatusReason = nil
	if r.StatusReason != nil {
		statusReason = r.StatusReason.ToProto()
	}

	return &proto.Run{
		Namespace:           r.Namespace,
		Pipeline:            r.Pipeline,
		Id:                  r.ID,
		Started:             r.Started,
		Ended:               r.Ended,
		State:               proto.Run_RunState(proto.Run_RunState_value[string(r.State)]),
		Status:              proto.Run_RunStatus(proto.Run_RunStatus_value[string(r.Status)]),
		StatusReason:        statusReason,
		TaskRuns:            r.TaskRuns,
		Trigger:             r.Trigger.ToProto(),
		Variables:           variables,
		StoreObjectsExpired: r.StoreObjectsExpired,
	}
}

func (r *Run) FromProto(proto *proto.Run) {
	var statusReason *RunStatusReason = nil
	if proto.StatusReason != nil {
		sr := &RunStatusReason{}
		sr.FromProto(proto.StatusReason)
		statusReason = sr
	}

	trigger := TriggerInfo{}
	trigger.FromProto(proto.Trigger)

	variables := []Variable{}
	for _, variable := range proto.Variables {
		vari := Variable{}
		vari.FromProto(variable)
		variables = append(variables, vari)
	}

	r.Namespace = proto.Namespace
	r.Pipeline = proto.Pipeline
	r.ID = proto.Id
	r.Started = proto.Started
	r.Ended = proto.Ended
	r.State = RunState(proto.State.String())
	r.Status = RunStatus(proto.Status.String())
	r.StatusReason = statusReason
	r.TaskRuns = proto.TaskRuns
	r.Trigger = trigger
	r.Variables = variables
	r.StoreObjectsExpired = proto.StoreObjectsExpired
}
