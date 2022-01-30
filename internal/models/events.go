package models

import (
	"time"

	"github.com/clintjedwards/gofer/proto"
)

type EventType string

type Event interface {
	GetID() int64
	SetID(id int64)
	GetKind() EventType
	GetEmitted() int64
}

const (
	// AnyEvent is a special event type the denotes the caller wants to listen for any event. It should not be used
	// as a normal event type (for example do not publish anything with it).
	// The AnyEvent type is internal only.
	AnyEvent EventType = "*"

	// Namespace events
	CreatedNamespaceEvent EventType = "CREATED_NAMESPACE"

	// Pipeline events
	DisabledPipelineEvent  EventType = "DISABLED_PIPELINE"
	EnabledPipelineEvent   EventType = "ENABLED_PIPELINE"
	CreatedPipelineEvent   EventType = "CREATED_PIPELINE"
	AbandonedPipelineEvent EventType = "ABANDONED_PIPELINE"

	// Run events
	StartedRunEvent   EventType = "STARTED_RUN"
	CompletedRunEvent EventType = "COMPLETED_RUN"

	// TaskRun events
	StartedTaskRunEvent   EventType = "STARTED_TASK_RUN"   // Task run is getting ready to be scheduled.
	ScheduledTaskRunEvent EventType = "SCHEDULED_TASK_RUN" // Task run has been attempted to be scheduled.
	CompletedTaskRunEvent EventType = "COMPLETED_TASK_RUN" // Task run has completed.

	// Trigger events; these are all from the perspective of the Gofer main process.
	FiredTriggerEvent     EventType = "FIRED_TRIGGER"     // Received a new trigger event.
	ProcessedTriggerEvent EventType = "PROCESSED_TRIGGER" // Currently processing a trigger event that was fired.
	ResolvedTriggerEvent  EventType = "RESOLVED_TRIGGER"  // Successfully processed the trigger.
)

var EventMap = map[EventType]string{
	AnyEvent: string(AnyEvent),

	CreatedNamespaceEvent: string(CreatedNamespaceEvent),

	DisabledPipelineEvent:  string(DisabledPipelineEvent),
	EnabledPipelineEvent:   string(EnabledPipelineEvent),
	CreatedPipelineEvent:   string(CreatedPipelineEvent),
	AbandonedPipelineEvent: string(AbandonedPipelineEvent),

	StartedRunEvent:   string(StartedRunEvent),
	CompletedRunEvent: string(CompletedRunEvent),

	StartedTaskRunEvent:   string(StartedTaskRunEvent),
	ScheduledTaskRunEvent: string(ScheduledTaskRunEvent),
	CompletedTaskRunEvent: string(CompletedTaskRunEvent),

	FiredTriggerEvent:     string(FiredTriggerEvent),
	ProcessedTriggerEvent: string(ProcessedTriggerEvent),
	ResolvedTriggerEvent:  string(ResolvedTriggerEvent),
}

type Metadata struct {
	EventID int64     `json:"event_id" storm:"id,increment"` // Unique identifier for event
	Kind    EventType `json:"kind"`                          // the type of event it is disabled_pipeline
	Emitted int64     `json:"emitted"`                       // Time event was performed in epoch milliseconds.
}

func (m *Metadata) GetID() int64 {
	return m.EventID
}

func (m *Metadata) SetID(id int64) {
	m.EventID = id
}

func (m *Metadata) GetKind() EventType {
	return m.Kind
}

func (m *Metadata) GetEmitted() int64 {
	return m.Emitted
}

func (m *Metadata) ToProto() *proto.Metadata {
	return &proto.Metadata{
		EventId: m.EventID,
		Kind:    proto.EventType(proto.EventType_value[string(m.Kind)]),
		Emitted: m.Emitted,
	}
}

func (m *Metadata) FromProto(p *proto.Metadata) {
	m.EventID = p.EventId
	m.Kind = EventType(p.Kind.String())
	m.Emitted = p.Emitted
}

func NewMetadata(kind EventType) Metadata {
	return Metadata{
		Kind:    kind,
		Emitted: time.Now().UnixMilli(),
	}
}
