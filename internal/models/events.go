package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

// Make sure to keep changes to these enums in lockstep with EventTypeMap
type EventType string

const (
	// The Any kind is a special event kind that denotes the caller wants to listen for any event.
	// It should not be used as a normal event type(for example do not publish anything with it).
	// It is internal only and not passed back on event streaming.
	EventTypeAny EventType = "ANY"

	// Namespaces
	EventTypeNamespaceCreated EventType = "NAMESPACE_CREATED"
	EventTypeNamespaceDeleted EventType = "NAMESPACE_DELETED"

	// Pipelines
	EventTypePipelineDisabled                EventType = "PIPELINE_DISABLED"
	EventTypePipelineEnabled                 EventType = "PIPELINE_ENABLED"
	EventTypePipelineRegistered              EventType = "PIPELINE_REGISTERED"
	EventTypePipelineDeployStarted           EventType = "PIPELINE_DEPLOY_STARTED"
	EventTypePipelineDeployCompleted         EventType = "PIPELINE_DEPLOY_COMPLETED"
	EventTypePipelineDeleted                 EventType = "PIPELINE_DELETED"
	EventTypePipelineExtensionSubscription   EventType = "PIPELINE_EXTENSION_SUBSCRIPTION"
	EventTypePipelineExtensionUnsubscription EventType = "PIPELINE_EXTENSION_UNSUBSCRIPTION"
	EventTypePipelineObjectEvicted           EventType = "EVICTED_PIPELINE_OBJECT"

	// Pipeline configs
	EventTypePipelineConfigRegistered EventType = "PIPELINE_CONFIG_REGISTERED"
	EventTypePipelineConfigDeleted    EventType = "PIPELINE_CONFIG_DELETED"

	// Runs
	EventTypeRunStarted        EventType = "RUN_STARTED"
	EventTypeRunCompleted      EventType = "RUN_COMPLETED"
	EventTypeRunObjectsExpired EventType = "RUN_OBJECTS_EXPIRED"

	// Task Runs
	EventTypeTaskRunCreated   EventType = "TASKRUN_CREATED"
	EventTypeTaskRunStarted   EventType = "TASKRUN_STARTED"
	EventTypeTaskRunCompleted EventType = "TASKRUN_COMPLETED"

	// Extensions
	EventTypeExtensionInstalled   EventType = "EXTENSION_INSTALLED"
	EventTypeExtensionUninstalled EventType = "EXTENSION_UNINSTALLED"
	EventTypeExtensionEnabled     EventType = "EXTENSION_ENABLED"
	EventTypeExtensionDisabled    EventType = "EXTENSION_DISABLED"
)

type EventTypeDetails interface {
	Kind() EventType
}

type EventNamespaceCreated struct {
	NamespaceID string `json:"namespace_id"`
}

func (e EventNamespaceCreated) Kind() EventType {
	return EventTypeNamespaceCreated
}

type EventNamespaceDeleted struct {
	NamespaceID string `json:"namespace_id"`
}

func (e EventNamespaceDeleted) Kind() EventType {
	return EventTypeNamespaceDeleted
}

type EventPipelineDisabled struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventPipelineDisabled) Kind() EventType {
	return EventTypePipelineDisabled
}

type EventPipelineEnabled struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventPipelineEnabled) Kind() EventType {
	return EventTypePipelineEnabled
}

type EventPipelineRegistered struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventPipelineRegistered) Kind() EventType {
	return EventTypePipelineRegistered
}

type EventPipelineRegisteredConfig struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
}

func (e EventPipelineRegisteredConfig) Kind() EventType {
	return EventTypePipelineConfigRegistered
}

type EventPipelineDeployStarted struct {
	NamespaceID  string `json:"namespace_id"`
	PipelineID   string `json:"pipeline_id"`
	StartVersion int64  `json:"start_version"`
	EndVersion   int64  `json:"end_version"`
}

func (e EventPipelineDeployStarted) Kind() EventType {
	return EventTypePipelineDeployStarted
}

type EventPipelineDeployCompleted struct {
	NamespaceID  string `json:"namespace_id"`
	PipelineID   string `json:"pipeline_id"`
	StartVersion int64  `json:"start_version"`
	EndVersion   int64  `json:"end_version"`
}

func (e EventPipelineDeployCompleted) Kind() EventType {
	return EventTypePipelineDeployCompleted
}

type EventPipelineDeleted struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
}

func (e EventPipelineDeleted) Kind() EventType {
	return EventTypePipelineDeleted
}

type EventPipelineConfigRegistered struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
}

func (e EventPipelineConfigRegistered) Kind() EventType {
	return EventTypePipelineConfigRegistered
}

type EventPipelineConfigDeleted struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
}

func (e EventPipelineConfigDeleted) Kind() EventType {
	return EventTypePipelineConfigDeleted
}

type EventPipelineExtensionSubscription struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Version     int64  `json:"version"`
	Label       string `json:"label"`
	Name        string `json:"name"`
}

func (e EventPipelineExtensionSubscription) Kind() EventType {
	return EventTypePipelineExtensionSubscription
}

type EventPipelineExtensionUnsubscription struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Label       string `json:"label"`
	Name        string `json:"name"`
}

func (e EventPipelineExtensionUnsubscription) Kind() EventType {
	return EventTypePipelineExtensionUnsubscription
}

type EventRunStarted struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
}

func (e EventRunStarted) Kind() EventType {
	return EventTypeRunStarted
}

type EventRunCompleted struct {
	NamespaceID string    `json:"namespace_id"`
	PipelineID  string    `json:"pipeline_id"`
	RunID       int64     `json:"run_id"`
	Status      RunStatus `json:"status"`
}

func (e EventRunCompleted) Kind() EventType {
	return EventTypeRunCompleted
}

type EventTaskRunCreated struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
	TaskRunID   string `json:"task_run_id"`
}

func (e EventTaskRunCreated) Kind() EventType {
	return EventTypeTaskRunCreated
}

type EventTaskRunStarted struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
	TaskRunID   string `json:"task_run_id"`
}

func (e EventTaskRunStarted) Kind() EventType {
	return EventTypeTaskRunStarted
}

type EventTaskRunCompleted struct {
	NamespaceID string        `json:"namespace_id"`
	PipelineID  string        `json:"pipeline_id"`
	RunID       int64         `json:"run_id"`
	TaskRunID   string        `json:"task_run_id"`
	Status      TaskRunStatus `json:"status"`
}

func (e EventTaskRunCompleted) Kind() EventType {
	return EventTypeTaskRunCompleted
}

type EventExtensionInstalled struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventExtensionInstalled) Kind() EventType {
	return EventTypeExtensionInstalled
}

type EventExtensionUninstalled struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventExtensionUninstalled) Kind() EventType {
	return EventTypeExtensionUninstalled
}

type EventExtensionEnabled struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventExtensionEnabled) Kind() EventType {
	return EventTypeExtensionEnabled
}

type EventExtensionDisabled struct {
	Name  string `json:"name"`
	Image string `json:"image"`
}

func (e EventExtensionDisabled) Kind() EventType {
	return EventTypeExtensionDisabled
}

type EventRunObjectsExpired struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	RunID       int64  `json:"run_id"`
}

func (e EventRunObjectsExpired) Kind() EventType {
	return EventTypeRunObjectsExpired
}

type EventPipelineObjectEvicted struct {
	NamespaceID string `json:"namespace_id"`
	PipelineID  string `json:"pipeline_id"`
	Key         string `json:"key"`
}

func (e EventPipelineObjectEvicted) Kind() EventType {
	return EventTypePipelineObjectEvicted
}

// Maps the kind type into an empty instance of the detail type.
// This allows us to quickly get back the correct type for things
// like json marshalling and unmarshalling.
// Make sure to keep this map in lockstep with the EventType enum.
var EventTypeMap = map[EventType]EventTypeDetails{
	EventTypeNamespaceCreated: &EventNamespaceCreated{},
	EventTypeNamespaceDeleted: &EventNamespaceDeleted{},

	EventTypePipelineDisabled:                &EventPipelineDisabled{},
	EventTypePipelineEnabled:                 &EventPipelineEnabled{},
	EventTypePipelineRegistered:              &EventPipelineRegistered{},
	EventTypePipelineDeployStarted:           &EventPipelineDeployStarted{},
	EventTypePipelineDeployCompleted:         &EventPipelineDeployCompleted{},
	EventTypePipelineExtensionSubscription:   &EventPipelineExtensionSubscription{},
	EventTypePipelineExtensionUnsubscription: &EventPipelineExtensionUnsubscription{},
	EventTypePipelineDeleted:                 &EventPipelineDeleted{},
	EventTypePipelineObjectEvicted:           &EventPipelineObjectEvicted{},

	EventTypePipelineConfigRegistered: &EventPipelineConfigRegistered{},
	EventTypePipelineConfigDeleted:    &EventPipelineConfigDeleted{},

	EventTypeRunStarted:        &EventRunStarted{},
	EventTypeRunCompleted:      &EventRunCompleted{},
	EventTypeRunObjectsExpired: &EventRunObjectsExpired{},

	EventTypeTaskRunCreated:   &EventTaskRunCreated{},
	EventTypeTaskRunStarted:   &EventTaskRunStarted{},
	EventTypeTaskRunCompleted: &EventTaskRunCompleted{},

	EventTypeExtensionInstalled:   &EventExtensionInstalled{},
	EventTypeExtensionUninstalled: &EventExtensionUninstalled{},
	EventTypeExtensionEnabled:     &EventExtensionEnabled{},
	EventTypeExtensionDisabled:    &EventExtensionDisabled{},
}

// A single event type
type Event struct {
	ID      int64            // Unique identifier for event.
	Type    EventType        // The type of event it is.
	Details EventTypeDetails // A struct of details about the specific event.
	Emitted int64            // Time event was performed in epoch milliseconds.
}

func NewEvent(details EventTypeDetails) *Event {
	return &Event{
		ID:      0,
		Type:    details.Kind(),
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
		Type:    string(e.Type),
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
		Type:    string(e.Type),
		Details: string(details),
		Emitted: e.Emitted,
	}
}

func (e *Event) FromStorage(evt *storage.Event) {
	detail := EventTypeMap[EventType(evt.Type)]

	err := json.Unmarshal([]byte(evt.Details), &detail)
	if err != nil {
		log.Fatal().Err(err).Msg("could not (un)marshal from storage")
	}

	e.ID = evt.ID
	e.Type = EventType(evt.Type)
	e.Details = detail
	e.Emitted = evt.Emitted
}
