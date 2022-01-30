package models

import (
	"time"

	"github.com/clintjedwards/gofer/proto"
)

type RunState string

const (
	RunUnknown    RunState = "UNKNOWN"    // The state of the run is unknown.
	RunProcessing RunState = "PROCESSING" // Going through validation checks in preparation for scheduling.
	RunWaiting    RunState = "WAITING"    // Waiting to be scheduled.
	RunRunning    RunState = "RUNNING"    // Currently running.
	RunFailed     RunState = "FAILED"     // Encountered an issue either in scheduling or with the container application.
	RunSuccess    RunState = "SUCCESS"    // Finished with a proper error code.
	RunCancelled  RunState = "CANCELLED"  // Cancelled by the user action.
)

type RunFailureKind string

const (
	RunFailureKindUnknown            RunFailureKind = "UNKNOWN"             // The failure is for unknown reasons.
	RunFailureKindAbnormalExit       RunFailureKind = "ABNORMAL_EXIT"       // Run could not complete successfully.
	RunFailureKindSchedulerError     RunFailureKind = "SCHEDULER_ERROR"     // Run could not be scheduled properly.
	RunFailureKindFailedPrecondition RunFailureKind = "FAILED_PRECONDITION" // Run parameters were not acceptable.
	RunFailureKindCancelled          RunFailureKind = "CANCELLED"           // Run or child task runs were cancelled due to user action.
)

// Run represents one or more tasks being executed on behalf of some trigger(manual or otherwise). Run is a third level
// unit being contained within pipelines.
type Run struct {
	Ended       int64      `json:"ended"`                   // Time of run finish in epoch milli.
	Failure     RunFailure `json:"failure"`                 // Details about a failed run.
	ID          int64      `json:"id" storm:"id,increment"` // UniqueID of a run. Autoincrementing and cannot be zero.
	NamespaceID string     `json:"namespace_id"`            // Unique ID of namespace.
	PipelineID  string     `json:"pipeline_id"`             // The unique ID of the related pipeline.
	Started     int64      `json:"started"`                 // Time of run start in epoch milli.
	State       RunState   `json:"status"`                  // The current state of the run.
	TaskRuns    []string   `json:"task_runs"`               // The unique ID of each task run.

	// Allows the ability to only launch a run with specific tasks mentioned in this list.
	Only        map[string]struct{} `json:"only"`
	TriggerKind string              `json:"trigger_kind"` // The id/name of the trigger used to initiate this run.
	TriggerName string              `json:"trigger_name"` // The user defined name of the trigger used to initiate this run.

	// Env vars to be injected into each child task run. These are usually taken from a trigger.
	Variables      map[string]string `json:"variables"`
	Objects        []string          `json:"objects"`         // Object keys that are stored at the run level.
	ObjectsExpired bool              `json:"objects_expired"` // Tracks whether objects for this run have expired already.
}

type RunFailure struct {
	Kind        RunFailureKind `json:"kind"`        // The specific type of run failure. Good for documentation about what it might be.
	Description string         `json:"description"` // The description of why the run might have failed.
}

func NewRun(pipelineID, namespaceID, triggerKind, triggerName string, taskFilter map[string]struct{}, vars map[string]string) *Run {
	return &Run{
		Started:     time.Now().UnixMilli(),
		State:       RunProcessing,
		PipelineID:  pipelineID,
		NamespaceID: namespaceID,
		TriggerKind: triggerKind,
		TriggerName: triggerName,
		Only:        taskFilter,
		Variables:   vars,
		Objects:     []string{},
	}
}

// IsComplete returns whether the run has completed and no further state changes will be made.
func (r *Run) IsComplete() bool {
	if r.State == RunFailed ||
		r.State == RunSuccess ||
		r.State == RunCancelled ||
		r.State == RunUnknown {
		return true
	}

	return false
}

// SetFailed updates the run with the appropriate parameters should a run fail. Only updates it in memory, a call to
// storage to save it is still needed.
func (r *Run) SetFailed(kind RunFailureKind, description string) {
	r.State = RunFailed
	r.Ended = time.Now().UnixMilli()
	r.Failure = RunFailure{
		Description: description,
		Kind:        kind,
	}
}

func (r *Run) SetSucceeded() {
	r.State = RunSuccess
	r.Ended = time.Now().UnixMilli()
}

func (r *Run) SetCancelled(description string) {
	r.State = RunCancelled
	r.Ended = time.Now().UnixMilli()
	r.Failure = RunFailure{
		Description: description,
		Kind:        RunFailureKindCancelled,
	}
}

func (r *Run) ToProto() *proto.Run {
	protoFailure := proto.RunFailure{
		Kind:        proto.RunFailure_Kind(proto.RunFailure_Kind_value[string(r.Failure.Kind)]),
		Description: r.Failure.Description,
	}
	protoOnly := []string{}
	for id := range r.Only {
		protoOnly = append(protoOnly, id)
	}

	return &proto.Run{
		Ended:          r.Ended,
		Id:             r.ID,
		PipelineId:     r.PipelineID,
		NamespaceId:    r.NamespaceID,
		Started:        r.Started,
		State:          proto.Run_State(proto.Run_State_value[string(r.State)]),
		TaskRuns:       r.TaskRuns,
		Only:           protoOnly,
		TriggerKind:    r.TriggerKind,
		TriggerName:    r.TriggerName,
		Failure:        &protoFailure,
		Variables:      r.Variables,
		Objects:        r.Objects,
		ObjectsExpired: r.ObjectsExpired,
	}
}

func (r *Run) FromProto(proto *proto.Run) {
	failure := RunFailure{
		Kind:        RunFailureKind(proto.Failure.Kind.String()),
		Description: proto.Failure.Description,
	}

	r.Ended = proto.Ended
	r.ID = proto.Id
	r.PipelineID = proto.PipelineId
	r.NamespaceID = proto.NamespaceId
	r.Started = proto.Started
	r.State = RunState(proto.State.String())
	r.TaskRuns = proto.TaskRuns
	r.Only = sliceToSet(proto.Only)
	r.TriggerKind = proto.TriggerKind
	r.TriggerName = proto.TriggerName
	r.Failure = failure
	r.Objects = proto.Objects
	r.Variables = proto.Variables
	r.ObjectsExpired = proto.ObjectsExpired
}
