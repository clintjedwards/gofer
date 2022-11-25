package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
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

type RunStatusReason struct {
	Reason      RunStatusReasonKind `json:"kind"`        // The specific type of run failure. Good for documentation about what it might be.
	Description string              `json:"description"` // The description of why the run might have failed.
}

func (r *RunStatusReason) ToJSON() string {
	reason, err := json.Marshal(r)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert extension subscription status reason to json")
	}

	return string(reason)
}

func (r *RunStatusReason) ToProto() *proto.RunStatusReason {
	return &proto.RunStatusReason{
		Reason:      proto.RunStatusReason_RunStatusReasonKind(proto.RunStatusReason_RunStatusReasonKind_value[string(r.Reason)]),
		Description: r.Description,
	}
}

// Information about which extension was responsible for the run's execution.
type ExtensionInfo struct {
	Name string // The extension kind responsible for starting the run.
	// The extension label responsible for starting the run. The label is a user chosen name
	// for the extension to differentiate it from other pipeline extensions of the same kind.
	Label string
}

func (t *ExtensionInfo) ToProto() *proto.Run_RunExtensionInfo {
	return &proto.Run_RunExtensionInfo{
		Name:  t.Name,
		Label: t.Label,
	}
}

// A run is one or more tasks being executed on behalf of some extension.
// Run is a third level unit containing tasks and being contained in a pipeline.
type Run struct {
	Namespace           string           `json:"namespace"`             // Unique ID of namespace.
	Pipeline            string           `json:"pipeline"`              // The unique ID of the related pipeline.
	Version             int64            `json:"version"`               // Which version of the pipeline did this run occur.
	ID                  int64            `json:"id"`                    // UniqueID of a run. Auto-incrementing and cannot be zero.
	Started             int64            `json:"started"`               // Time of run start in epoch milli.
	Ended               int64            `json:"ended"`                 // Time of run finish in epoch milli.
	State               RunState         `json:"state"`                 // The current state of the run.
	Status              RunStatus        `json:"status"`                // The current status of the run.
	StatusReason        *RunStatusReason `json:"status_reason"`         // Contains more information about a run's current status.
	Extension           ExtensionInfo    `json:"extension"`             // Information about which extension was responsible for the run's execution.
	Variables           []Variable       `json:"variables"`             // Environment variables to be injected into each child task run. These are usually injected by the extension.
	StoreObjectsExpired bool             `json:"store_objects_expired"` // Tracks whether objects for this run have expired already.
}

func NewRun(namespace, pipeline string, version, id int64, extension ExtensionInfo, variables []Variable) *Run {
	return &Run{
		Namespace:           namespace,
		Pipeline:            pipeline,
		Version:             version,
		ID:                  id,
		Started:             time.Now().UnixMilli(),
		Ended:               0,
		State:               RunStatePending,
		Status:              RunStatusUnknown,
		StatusReason:        nil,
		Extension:           extension,
		Variables:           variables,
		StoreObjectsExpired: false,
	}
}

func (r *Run) ToProto() *proto.Run {
	variables := []*proto.Variable{}
	for _, variable := range r.Variables {
		variables = append(variables, variable.ToProto())
	}

	var statusReason *proto.RunStatusReason
	if r.StatusReason != nil {
		statusReason = r.StatusReason.ToProto()
	}

	return &proto.Run{
		Namespace:           r.Namespace,
		Pipeline:            r.Pipeline,
		Version:             r.Version,
		Id:                  r.ID,
		Started:             r.Started,
		Ended:               r.Ended,
		State:               proto.Run_RunState(proto.Run_RunState_value[string(r.State)]),
		Status:              proto.Run_RunStatus(proto.Run_RunStatus_value[string(r.Status)]),
		StatusReason:        statusReason,
		Extension:           r.Extension.ToProto(),
		Variables:           variables,
		StoreObjectsExpired: r.StoreObjectsExpired,
	}
}

func (r *Run) ToStorage() *storage.PipelineRun {
	extension, err := json.Marshal(r.Extension)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	variables, err := json.Marshal(r.Variables)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.PipelineRun{
		Namespace:             r.Namespace,
		Pipeline:              r.Pipeline,
		PipelineConfigVersion: r.Version,
		ID:                    r.ID,
		Started:               r.Started,
		Ended:                 r.Ended,
		State:                 string(r.State),
		Status:                string(r.Status),
		StatusReason:          r.StatusReason.ToJSON(),
		Extension:             string(extension),
		Variables:             string(variables),
		StoreObjectsExpired:   r.StoreObjectsExpired,
	}
}

func (r *Run) FromStorage(storage *storage.PipelineRun) {
	var statusReason RunStatusReason
	err := json.Unmarshal([]byte(storage.StatusReason), &statusReason)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var extension ExtensionInfo
	err = json.Unmarshal([]byte(storage.Extension), &extension)
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
	r.Version = storage.PipelineConfigVersion
	r.ID = storage.ID
	r.Started = storage.Started
	r.Ended = storage.Ended
	r.State = RunState(storage.State)
	r.Status = RunStatus(storage.Status)
	r.StatusReason = &statusReason
	r.Extension = extension
	r.Variables = variables
	r.StoreObjectsExpired = storage.StoreObjectsExpired
}
