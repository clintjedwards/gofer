package models

import (
	"encoding/json"
	"fmt"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

type RunState string

const (
	RunStateUnknown RunState = "UNKNOWN" // The state of the run is unknown.
	// Before the tasks in a run is sent to a scheduler it must complete various steps like
	// validation checking. This state represents that step where the run and task executions are
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

type RunStatusReasonType string

const (
	// Gofer has no idea how the run got into this state.
	RunStatusReasonKindUnknown RunStatusReasonType = "UNKNOWN"
	// While executing the run one or more tasks exited with an abnormal exit code.
	RunStatusReasonKindAbnormalExit RunStatusReasonType = "ABNORMAL_EXIT"
	// While executing the run one or more tasks could not be scheduled.
	RunStatusReasonKindSchedulerError RunStatusReasonType = "SCHEDULER_ERROR"
	// The run could not be executed as requested due to user defined attributes given.
	RunStatusReasonKindFailedPrecondition RunStatusReasonType = "FAILED_PRECONDITION"
	// One or more tasks could not be completed due to a user cancelling the run.
	RunStatusReasonKindUserCancelled RunStatusReasonType = "USER_CANCELLED"
	// One or more tasks could not be completed due to the system or admin cancelling the run.
	RunStatusReasonKindAdminCancelled RunStatusReasonType = "ADMIN_CANCELLED"
)

type RunStatusReason struct {
	Reason      RunStatusReasonType `json:"reason" example:"ABNORMAL_EXIT" doc:"The specific type of run failure"`
	Description string              `json:"description" example:"some description about the reason" doc:"The description of why the run might have failed"`
}

func (r *RunStatusReason) ToJSON() string {
	reason, err := json.Marshal(r)
	if err != nil {
		log.Fatal().Err(err).Msg("failed to convert extension subscription status reason to json")
	}

	return string(reason)
}

type InitiatorType string

const (
	// Gofer has no idea who was the initiator.
	InitiatorTypeUnknown   InitiatorType = "UNKNOWN"
	InitiatorTypeBot       InitiatorType = "BOT"
	InitiatorTypeHuman     InitiatorType = "HUMAN"
	InitiatorTypeExtension InitiatorType = "EXTENSION"
)

// Information about which extension was responsible for the run's execution.
type Initiator struct {
	Type   InitiatorType `json:"type" example:"BOT" doc:"Which type of user initiated the run"`
	Name   string        `json:"name" example:"obama" doc:"The name of the user which initiated the run"`
	Reason string        `json:"reason" example:"Re-running due to previous failure" doc:"The reason the run was initiated."`
}

// A run is one or more tasks being executed on behalf of some extension.
// Run is a third level unit containing tasks and being contained in a pipeline.
type Run struct {
	NamespaceID         string           `json:"namespace_id,omitempty" example:"default" doc:"Unique identifier of the target namespace"`
	PipelineID          string           `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline"`
	Version             int64            `json:"version" example:"42" doc:"Which version of the pipeline did this task execution occur in"`
	RunID               int64            `json:"run_id" example:"1" doc:"Unique identifier for the target run"`
	Started             uint64           `json:"started" example:"1712433802634" doc:"Time of run creation in epoch milliseconds"`
	Ended               uint64           `json:"ended" example:"1712433802634" doc:"Time of run completion in epoch milliseconds"`
	State               RunState         `json:"state" example:"PENDING" doc:"The current state of the run within the Gofer execution model. Describes if the run is in progress or not."`
	Status              RunStatus        `json:"status" example:"SUCCESSFUL" doc:"The final result of the run."`
	StatusReason        *RunStatusReason `json:"status_reason,omitempty" example:"Could not finish run due to some reason" doc:"More information on the circumstances around a particular run's status"`
	Initiator           Initiator        `json:"initiator" doc:"Information about what started the run"`
	Variables           []Variable       `json:"variables" doc:"Run level environment variables to be passed to each task execution"`
	StoreObjectsExpired bool             `json:"store_objects_expired" doc:"Whether run level objects were expired"`
}

func NewRun(namespace, pipeline string, version, id int64, initiator Initiator, variables []Variable) *Run {
	return &Run{
		NamespaceID:         namespace,
		PipelineID:          pipeline,
		Version:             version,
		RunID:               id,
		Started:             uint64(time.Now().UnixMilli()),
		Ended:               0,
		State:               RunStatePending,
		Status:              RunStatusUnknown,
		StatusReason:        nil,
		Initiator:           initiator,
		Variables:           variables,
		StoreObjectsExpired: false,
	}
}

func (r *Run) ToStorage() *storage.PipelineRun {
	initiator, err := json.Marshal(r.Initiator)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	variables, err := json.Marshal(r.Variables)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.PipelineRun{
		Namespace:             r.NamespaceID,
		Pipeline:              r.PipelineID,
		PipelineConfigVersion: r.Version,
		ID:                    r.RunID,
		Started:               fmt.Sprint(r.Started),
		Ended:                 fmt.Sprint(r.Ended),
		State:                 string(r.State),
		Status:                string(r.Status),
		StatusReason:          r.StatusReason.ToJSON(),
		Initiator:             string(initiator),
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

	var initiator Initiator
	err = json.Unmarshal([]byte(storage.Initiator), &initiator)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var variables []Variable
	err = json.Unmarshal([]byte(storage.Variables), &variables)
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
	r.Version = storage.PipelineConfigVersion
	r.RunID = storage.ID
	r.Started = started
	r.Ended = ended
	r.State = RunState(storage.State)
	r.Status = RunStatus(storage.Status)
	r.StatusReason = &statusReason
	r.Initiator = initiator
	r.Variables = variables
	r.StoreObjectsExpired = storage.StoreObjectsExpired
}
