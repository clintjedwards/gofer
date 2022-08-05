package models

import (
	"encoding/json"
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type EventKind string

const (
	// The Any kind is a special event kind that denotes the caller wants to listen for any event.
	// It should not be used as a normal event type(for example do not publish anything with it).
	// It is internal only and not passed back on event streaming.
	EventKindAny EventKind = "ANY"

	EventKindCreatedNamespace EventKind = "CREATED_NAMESPACE"
	EventKindDeletedNamespace EventKind = "DELETED_NAMESPACE"

	EventKindDisabledPipeline EventKind = "DISABLED_PIPELINE"
	EventKindEnabledPipeline  EventKind = "ENABLED_PIPELINE"
	EventKindCreatedPipeline  EventKind = "CREATED_PIPELINE"
	EventKindDeletedPipeline  EventKind = "DELETED_PIPELINE"

	EventKindStartedRun   EventKind = "STARTED_RUN"
	EventKindCompletedRun EventKind = "COMPLETED_RUN"

	EventKindCreatedTaskRun   EventKind = "CREATED_TASKRUN"
	EventKindStartedTaskRun   EventKind = "STARTED_TASKRUN"
	EventKindCompletedTaskRun EventKind = "COMPLETED_TASKRUN"

	EventKindInstalledTrigger   EventKind = "INSTALLED_TRIGGER"
	EventKindUninstalledTrigger EventKind = "UNINSTALLED_TRIGGER"
	EventKindEnabledTrigger     EventKind = "ENABLED_TRIGGER"
	EventKindDisabledTrigger    EventKind = "DISABLED_TRIGGER"

	EventKindInstalledCommonTask   EventKind = "INSTALLED_COMMON_TASK"
	EventKindUninstalledCommonTask EventKind = "UNINSTALLED_COMMON_TASK"
	EventKindEnabledCommonTask     EventKind = "ENABLED_COMMON_TASK"
	EventKindDisabledCommonTask    EventKind = "DISABLED_COMMON_TASK"

	EventKindFiredTriggerEvent     EventKind = "FIRED_TRIGGER_EVENT"
	EventKindProcessedTriggerEvent EventKind = "PROCESSED_TRIGGER_EVENT"
	EventKindResolvedTriggerEvent  EventKind = "RESOLVED_TRIGGER_EVENT"

	EventKindExpiredRunObjects     EventKind = "EXPIRED_RUN_OBJECTS"
	EventKindEvictedPipelineObject EventKind = "EVICTED_PIPELINE_OBJECT"
)

type EventKindDetails interface {
	Kind() EventKind
}

type EventCreatedNamespace struct {
	NamespaceID string `json:"namespace_id"`
}

func (e EventCreatedNamespace) Kind() EventKind {
	return EventKindCreatedNamespace
}

type EventDeletedNamespace struct {
	NamespaceID string `json:"namespace_id"`
}

func (e EventDeletedNamespace) Kind() EventKind {
	return EventKindDeletedNamespace
}

type EventDisabledPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventDisabledPipeline) Kind() EventKind {
	return EventKindDisabledPipeline
}

type EventEnabledPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventEnabledPipeline) Kind() EventKind {
	return EventKindEnabledPipeline
}

type EventCreatedPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventCreatedPipeline) Kind() EventKind {
	return EventKindCreatedPipeline
}

type EventDeletedPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventDeletedPipeline) Kind() EventKind {
	return EventKindDeletedPipeline
}

type EventStartedRun struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
}

func (e EventStartedRun) Kind() EventKind {
	return EventKindStartedRun
}

type EventCompletedRun struct {
	NamespaceID string    `json:"namespace_id"`
	PipelineID  string    `json:"pipeline_id"`
	RunID       int64     `json:"run_id"`
	Status      RunStatus `json:"status"`
}

func (e EventCompletedRun) Kind() EventKind {
	return EventKindCompletedRun
}

type EventCreatedTaskRun struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
	TaskRunID   string `json:"task_run_id"`
}

func (e EventCreatedTaskRun) Kind() EventKind {
	return EventKindCreatedTaskRun
}

type EventStartedTaskRun struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
	TaskRunID   string `json:"task_run_id"`
}

func (e EventStartedTaskRun) Kind() EventKind {
	return EventKindStartedTaskRun
}

type EventCompletedTaskRun struct {
	NamespaceID string        `json:"namespace_id"`
	PipelineID  string        `json:"pipeline_id"`
	RunID       int64         `json:"run_id"`
	TaskRunID   string        `json:"task_run_id"`
	Status      TaskRunStatus `json:"status"`
}

func (e EventCompletedTaskRun) Kind() EventKind {
	return EventKindCompletedTaskRun
}

type EventInstalledTrigger struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventInstalledTrigger) Kind() EventKind {
	return EventKindInstalledTrigger
}

type EventUninstalledTrigger struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventUninstalledTrigger) Kind() EventKind {
	return EventKindUninstalledTrigger
}

type EventEnabledTrigger struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventEnabledTrigger) Kind() EventKind {
	return EventKindEnabledTrigger
}

type EventDisabledTrigger struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventDisabledTrigger) Kind() EventKind {
	return EventKindDisabledTrigger
}

type EventInstalledCommonTask struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventInstalledCommonTask) Kind() EventKind {
	return EventKindInstalledCommonTask
}

type EventUninstalledCommonTask struct {
	Name string `json:"name"`
}

func (e EventUninstalledCommonTask) Kind() EventKind {
	return EventKindUninstalledCommonTask
}

type EventEnabledCommonTask struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventEnabledCommonTask) Kind() EventKind {
	return EventKindEnabledCommonTask
}

type EventDisabledCommonTask struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventDisabledCommonTask) Kind() EventKind {
	return EventKindDisabledCommonTask
}

type EventFiredTriggerEvent struct {
	NamespaceID string            `json:"namespace_id"`
	PipelineID  string            `json:"pipeline_id"`
	Name        string            `json:"name"`
	Label       string            `json:"label"`
	Result      TriggerResult     `json:"result"`
	Metadata    map[string]string `json:"metadata"`
}

func (e EventFiredTriggerEvent) Kind() EventKind {
	return EventKindFiredTriggerEvent
}

type EventProcessedTriggerEvent struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Name        string `json:"name"`
	Label       string `json:"label"`
}

func (e EventProcessedTriggerEvent) Kind() EventKind {
	return EventKindProcessedTriggerEvent
}

type EventResolvedTriggerEvent struct {
	NamespaceID string            `json:"namespace_id"`
	PipelineID  string            `json:"pipeline_id"`
	Name        string            `json:"name"`
	Label       string            `json:"label"`
	Result      TriggerResult     `json:"result"`
	Metadata    map[string]string `json:"metadata"`
}

func (e EventResolvedTriggerEvent) Kind() EventKind {
	return EventKindResolvedTriggerEvent
}

type EventExpiredRunObjects struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
}

func (e EventExpiredRunObjects) Kind() EventKind {
	return EventKindExpiredRunObjects
}

type EventEvictedPipelineObject struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Key         string `json:"key"`
}

func (e EventEvictedPipelineObject) Kind() EventKind {
	return EventKindEvictedPipelineObject
}

// Maps the kind type into an empty instance of the detail type.
// This allows us to quickly get back the correct type for things
// like json marshalling and unmarshalling.
var EventKindMap = map[EventKind]EventKindDetails{
	EventKindCreatedNamespace: &EventCreatedNamespace{},
	EventKindDeletedNamespace: &EventDeletedNamespace{},

	EventKindDisabledPipeline: &EventDisabledPipeline{},
	EventKindEnabledPipeline:  &EventEnabledPipeline{},
	EventKindCreatedPipeline:  &EventCreatedPipeline{},
	EventKindDeletedPipeline:  &EventDeletedPipeline{},

	EventKindStartedRun:   &EventStartedRun{},
	EventKindCompletedRun: &EventCompletedRun{},

	EventKindCreatedTaskRun:   &EventCreatedTaskRun{},
	EventKindStartedTaskRun:   &EventStartedTaskRun{},
	EventKindCompletedTaskRun: &EventCompletedTaskRun{},

	EventKindInstalledTrigger:   &EventInstalledTrigger{},
	EventKindUninstalledTrigger: &EventUninstalledTrigger{},
	EventKindEnabledTrigger:     &EventEnabledTrigger{},
	EventKindDisabledTrigger:    &EventDisabledTrigger{},

	EventKindFiredTriggerEvent:     &EventFiredTriggerEvent{},
	EventKindProcessedTriggerEvent: &EventProcessedTriggerEvent{},
	EventKindResolvedTriggerEvent:  &EventResolvedTriggerEvent{},

	EventKindExpiredRunObjects:     &EventExpiredRunObjects{},
	EventKindEvictedPipelineObject: &EventEvictedPipelineObject{},
}

// A single event type
type Event struct {
	ID      int64            // Unique identifier for event.
	Kind    EventKind        // The type of event it is.
	Details EventKindDetails // A struct of details about the specific event.
	Emitted int64            // Time event was performed in epoch milliseconds.
}

func NewEvent(details EventKindDetails) *Event {
	return &Event{
		ID:      0,
		Kind:    details.Kind(),
		Details: details,
		Emitted: time.Now().UnixMilli(),
	}
}

func (e *Event) ToProto() (*proto.Event, error) {
	details, err := json.Marshal(e.Details)
	if err != nil {
		return nil, err
	}

	return &proto.Event{
		Id:      e.ID,
		Kind:    string(e.Kind),
		Details: string(details),
		Emitted: e.Emitted,
	}, nil
}

// TriggerResultState is a description of what has happened as a result of a 'trigger event' being resolved.
// Normally when a trigger fires, it passes down some information to the Gofer handler on how a pipeline might
// be executed. This execution detail might contain some extra information on why or, particularly, why not
// a trigger has fired and a pipeline should be run.
//
// For example:
// A trigger that evaluates whether a pipeline should run on a specific date might also skip certain
// holidays. In this case it would pass down an "skipped" event result to inform the user that their pipeline
// would have ran, but did not due to holiday.
//
// Footnote:
// In this example, it is somewhat arguable that an event should not have happened in the first place.
// but the counter-argument would be that we can conceive of a world where a user might want to understand
// WHY a trigger did not go off for a particular date and the difference between the trigger just not understanding
// that it *should have* executed vs the trigger **intentionally** skipping that date.
type TriggerResultStatus string

const (
	TriggerResultStateUnknown TriggerResultStatus = "UNKNOWN" // Event did not have a result; should never be in this state.
	TriggerResultStateSuccess TriggerResultStatus = "SUCCESS" // Trigger evaluation was successful.
	TriggerResultStateFailure TriggerResultStatus = "FAILURE" // Trigger evaluation was not successful.
	TriggerResultStateSkipped TriggerResultStatus = "SKIPPED" // Trigger evaluation was skipped
)

type TriggerResult struct {
	Details string              `json:"details"` // details about the trigger's current result.
	Status  TriggerResultStatus `json:"state"`
}

func (t *TriggerResult) ToProto() *proto.TriggerResult {
	return &proto.TriggerResult{
		Status:  proto.TriggerResult_Status(proto.TriggerResult_Status_value[string(t.Status)]),
		Details: t.Details,
	}
}

func (t *TriggerResult) FromProto(p *proto.TriggerResult) {
	t.Details = p.Details
	t.Status = TriggerResultStatus(p.Status.String())
}
