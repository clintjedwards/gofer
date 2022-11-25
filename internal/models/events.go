package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

// Make sure to keep changes to these enums in lockstep with EventKindMap
type EventKind string

const (
	// The Any kind is a special event kind that denotes the caller wants to listen for any event.
	// It should not be used as a normal event type(for example do not publish anything with it).
	// It is internal only and not passed back on event streaming.
	EventKindAny EventKind = "ANY"

	EventKindCreatedNamespace EventKind = "CREATED_NAMESPACE"
	EventKindDeletedNamespace EventKind = "DELETED_NAMESPACE"

	EventKindDisabledPipeline                              EventKind = "DISABLED_PIPELINE"
	EventKindEnabledPipeline                               EventKind = "ENABLED_PIPELINE"
	EventKindRegisteredPipeline                            EventKind = "REGISTERED_PIPELINE"
	EventKindStartedDeployPipeline                         EventKind = "STARTED_DEPLOY_PIPELINE"
	EventKindCompletedDeployPipeline                       EventKind = "COMPLETED_DEPLOY_PIPELINE"
	EventKindCompletedExtensionSubscriptionPipeline        EventKind = "COMPLETED_EXTENSION_SUBSCRIPTION_PIPELINE"
	EventKindFailedExtensionSubscriptionPipeline           EventKind = "FAILED_EXTENSION_SUBSCRIPTION_PIPELINE"
	EventKindCompletedExtensionSubscriptionRemovalPipeline EventKind = "COMPLETED_EXTENSION_SUBSCRIPTION_REMOVAL_PIPELINE"
	EventKindFailedExtensionSubscriptionRemovalPipeline    EventKind = "FAILED_EXTENSION_SUBSCRIPTION_REMOVAL_PIPELINE"
	EventKindDeletedPipeline                               EventKind = "DELETED_PIPELINE"

	EventKindRegisteredPipelineConfig EventKind = "REGISTERED_PIPELINE_CONFIG"
	EventKindDeletedPipelineConfig    EventKind = "DELETED_PIPELINE_CONFIG"

	EventKindStartedRun   EventKind = "STARTED_RUN"
	EventKindCompletedRun EventKind = "COMPLETED_RUN"

	EventKindCreatedTaskRun   EventKind = "CREATED_TASKRUN"
	EventKindStartedTaskRun   EventKind = "STARTED_TASKRUN"
	EventKindCompletedTaskRun EventKind = "COMPLETED_TASKRUN"

	EventKindInstalledExtension   EventKind = "INSTALLED_EXTENSION"
	EventKindUninstalledExtension EventKind = "UNINSTALLED_EXTENSION"
	EventKindEnabledExtension     EventKind = "ENABLED_EXTENSION"
	EventKindDisabledExtension    EventKind = "DISABLED_EXTENSION"

	EventKindInstalledCommonTask   EventKind = "INSTALLED_COMMON_TASK"
	EventKindUninstalledCommonTask EventKind = "UNINSTALLED_COMMON_TASK"
	EventKindEnabledCommonTask     EventKind = "ENABLED_COMMON_TASK"
	EventKindDisabledCommonTask    EventKind = "DISABLED_COMMON_TASK"

	EventKindFiredExtensionEvent     EventKind = "FIRED_EXTENSION_EVENT"
	EventKindProcessedExtensionEvent EventKind = "PROCESSED_EXTENSION_EVENT"
	EventKindResolvedExtensionEvent  EventKind = "RESOLVED_EXTENSION_EVENT"

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

type EventRegisteredPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventRegisteredPipeline) Kind() EventKind {
	return EventKindRegisteredPipeline
}

type EventRegisteredPipelineConfig struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
}

func (e EventRegisteredPipelineConfig) Kind() EventKind {
	return EventKindRegisteredPipelineConfig
}

type EventStartedDeployPipeline struct {
	NamespaceID  string `json:"namespace_id"`
	PipelineID   string `json:"pipeline_id"`
	StartVersion int64  `json:"start_version"`
	EndVersion   int64  `json:"end_version"`
}

func (e EventStartedDeployPipeline) Kind() EventKind {
	return EventKindStartedDeployPipeline
}

type EventCompletedDeployPipeline struct {
	NamespaceID  string `json:"namespace_id"`
	PipelineID   string `json:"pipeline_id"`
	StartVersion int64  `json:"start_version"`
	EndVersion   int64  `json:"end_version"`
}

func (e EventCompletedDeployPipeline) Kind() EventKind {
	return EventKindCompletedDeployPipeline
}

type EventDeletedPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventDeletedPipeline) Kind() EventKind {
	return EventKindDeletedPipeline
}

type EventDeletedPipelineConfig struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
}

func (e EventDeletedPipelineConfig) Kind() EventKind {
	return EventKindDeletedPipelineConfig
}

type EventCompletedExtensionSubscriptionPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
	Label       string `json:"label"`
	Name        string `json:"name"`
}

func (e EventCompletedExtensionSubscriptionPipeline) Kind() EventKind {
	return EventKindCompletedExtensionSubscriptionPipeline
}

type EventFailedExtensionSubscriptionPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Label       string `json:"label"`
	Name        string `json:"name"`
}

func (e EventFailedExtensionSubscriptionPipeline) Kind() EventKind {
	return EventKindFailedExtensionSubscriptionPipeline
}

type EventCompletedExtensionSubscriptionRemovalPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Label       string `json:"label"`
	Name        string `json:"name"`
}

func (e EventCompletedExtensionSubscriptionRemovalPipeline) Kind() EventKind {
	return EventKindCompletedExtensionSubscriptionRemovalPipeline
}

type EventFailedExtensionSubscriptionRemovalPipeline struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Label       string `json:"label"`
	Name        string `json:"name"`
}

func (e EventFailedExtensionSubscriptionRemovalPipeline) Kind() EventKind {
	return EventKindFailedExtensionSubscriptionRemovalPipeline
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

type EventInstalledExtension struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventInstalledExtension) Kind() EventKind {
	return EventKindInstalledExtension
}

type EventUninstalledExtension struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventUninstalledExtension) Kind() EventKind {
	return EventKindUninstalledExtension
}

type EventEnabledExtension struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventEnabledExtension) Kind() EventKind {
	return EventKindEnabledExtension
}

type EventDisabledExtension struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventDisabledExtension) Kind() EventKind {
	return EventKindDisabledExtension
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

type EventFiredExtensionEvent struct {
	NamespaceID string            `json:"namespace_id"`
	PipelineID  string            `json:"pipeline_id"`
	Name        string            `json:"name"`
	Label       string            `json:"label"`
	Result      ExtensionResult   `json:"result"`
	Metadata    map[string]string `json:"metadata"`
}

func (e EventFiredExtensionEvent) Kind() EventKind {
	return EventKindFiredExtensionEvent
}

type EventProcessedExtensionEvent struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Name        string `json:"name"`
	Label       string `json:"label"`
}

func (e EventProcessedExtensionEvent) Kind() EventKind {
	return EventKindProcessedExtensionEvent
}

type EventResolvedExtensionEvent struct {
	NamespaceID string            `json:"namespace_id"`
	PipelineID  string            `json:"pipeline_id"`
	Name        string            `json:"name"`
	Label       string            `json:"label"`
	Result      ExtensionResult   `json:"result"`
	Metadata    map[string]string `json:"metadata"`
}

func (e EventResolvedExtensionEvent) Kind() EventKind {
	return EventKindResolvedExtensionEvent
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
// Make sure to keep this map in lockstep with the EventKind enum.
var EventKindMap = map[EventKind]EventKindDetails{
	EventKindCreatedNamespace: &EventCreatedNamespace{},
	EventKindDeletedNamespace: &EventDeletedNamespace{},

	EventKindDisabledPipeline:                              &EventDisabledPipeline{},
	EventKindEnabledPipeline:                               &EventEnabledPipeline{},
	EventKindRegisteredPipeline:                            &EventRegisteredPipeline{},
	EventKindStartedDeployPipeline:                         &EventStartedDeployPipeline{},
	EventKindCompletedDeployPipeline:                       &EventCompletedDeployPipeline{},
	EventKindCompletedExtensionSubscriptionPipeline:        &EventCompletedExtensionSubscriptionPipeline{},
	EventKindFailedExtensionSubscriptionPipeline:           &EventFailedExtensionSubscriptionPipeline{},
	EventKindCompletedExtensionSubscriptionRemovalPipeline: &EventCompletedExtensionSubscriptionRemovalPipeline{},
	EventKindFailedExtensionSubscriptionRemovalPipeline:    &EventFailedExtensionSubscriptionRemovalPipeline{},
	EventKindDeletedPipeline:                               &EventDeletedPipeline{},

	EventKindRegisteredPipelineConfig: &EventRegisteredPipelineConfig{},
	EventKindDeletedPipelineConfig:    &EventDeletedPipelineConfig{},

	EventKindStartedRun:   &EventStartedRun{},
	EventKindCompletedRun: &EventCompletedRun{},

	EventKindCreatedTaskRun:   &EventCreatedTaskRun{},
	EventKindStartedTaskRun:   &EventStartedTaskRun{},
	EventKindCompletedTaskRun: &EventCompletedTaskRun{},

	EventKindInstalledExtension:   &EventInstalledExtension{},
	EventKindUninstalledExtension: &EventUninstalledExtension{},
	EventKindEnabledExtension:     &EventEnabledExtension{},
	EventKindDisabledExtension:    &EventDisabledExtension{},

	EventKindInstalledCommonTask:   &EventInstalledCommonTask{},
	EventKindUninstalledCommonTask: &EventUninstalledCommonTask{},
	EventKindEnabledCommonTask:     &EventEnabledCommonTask{},
	EventKindDisabledCommonTask:    &EventDisabledCommonTask{},

	EventKindFiredExtensionEvent:     &EventFiredExtensionEvent{},
	EventKindProcessedExtensionEvent: &EventProcessedExtensionEvent{},
	EventKindResolvedExtensionEvent:  &EventResolvedExtensionEvent{},

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

func (e *Event) ToStorage() *storage.Event {
	details, err := json.Marshal(e.Details)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	return &storage.Event{
		ID:      e.ID,
		Kind:    string(e.Kind),
		Details: string(details),
		Emitted: e.Emitted,
	}
}

func (e *Event) FromStorage(evt *storage.Event) {
	detail := EventKindMap[EventKind(evt.Kind)]

	err := json.Unmarshal([]byte(evt.Details), &detail)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	e.ID = evt.ID
	e.Kind = EventKind(evt.Kind)
	e.Details = detail
	e.Emitted = evt.Emitted
}

// ExtensionResultState is a description of what has happened as a result of a 'extension event' being resolved.
// Normally when a extension fires, it passes down some information to the Gofer handler on how a pipeline might
// be executed. This execution detail might contain some extra information on why or, particularly, why not
// a extension has fired and a pipeline should be run.
//
// For example:
// A extension that evaluates whether a pipeline should run on a specific date might also skip certain
// holidays. In this case it would pass down an "skipped" event result to inform the user that their pipeline
// would have ran, but did not due to holiday.
//
// Footnote:
// In this example, it is somewhat arguable that an event should not have happened in the first place.
// but the counter-argument would be that we can conceive of a world where a user might want to understand
// WHY a extension did not go off for a particular date and the difference between the extension just not understanding
// that it *should have* executed vs the extension **intentionally** skipping that date.
type ExtensionResultStatus string

const (
	ExtensionResultStateUnknown ExtensionResultStatus = "UNKNOWN" // Event did not have a result; should never be in this state.
	ExtensionResultStateSuccess ExtensionResultStatus = "SUCCESS" // Extension evaluation was successful.
	ExtensionResultStateFailure ExtensionResultStatus = "FAILURE" // Extension evaluation was not successful.
	ExtensionResultStateSkipped ExtensionResultStatus = "SKIPPED" // Extension evaluation was skipped
)

type ExtensionResult struct {
	Details string                `json:"details"` // details about the extension's current result.
	Status  ExtensionResultStatus `json:"state"`
}

func (t *ExtensionResult) ToProto() *proto.ExtensionResult {
	return &proto.ExtensionResult{
		Status:  proto.ExtensionResult_Status(proto.ExtensionResult_Status_value[string(t.Status)]),
		Details: t.Details,
	}
}
