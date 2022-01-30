package models

import "github.com/clintjedwards/gofer/proto"

// Event names should be in past tense since in event systems these events have already happened.
//
// IMPORTANT: Events added here are unfortunately coupled to a large amount of places. The best solution is to ctrl+f or
// get a list of all references to an event already created and then mimic all the places that that event exists for
// adding/updating/removing an event.
//
// The short list of known places events need to be changed when updated/removed/added is:
// * The models package (events.go, eventTypes.go)
// * The API events handler (eventsHandlers.go)
// * The Proto package (gofer_message_events.go, gofer_transport.proto)
// * The Storage package (events.go)

type EventDisabledPipeline struct {
	Metadata
	NamespaceID string
	PipelineID  string
}

func NewEventDisabledPipeline(pipeline Pipeline) *EventDisabledPipeline {
	return &EventDisabledPipeline{
		Metadata:    NewMetadata(DisabledPipelineEvent),
		NamespaceID: pipeline.Namespace,
		PipelineID:  pipeline.ID,
	}
}

func (e *EventDisabledPipeline) ToProto() *proto.EventDisabledPipeline {
	return &proto.EventDisabledPipeline{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
	}
}

type EventEnabledPipeline struct {
	Metadata
	NamespaceID string
	PipelineID  string
}

func NewEventEnabledPipeline(pipeline Pipeline) *EventEnabledPipeline {
	return &EventEnabledPipeline{
		Metadata:    NewMetadata(EnabledPipelineEvent),
		NamespaceID: pipeline.Namespace,
		PipelineID:  pipeline.ID,
	}
}

func (e *EventEnabledPipeline) ToProto() *proto.EventEnabledPipeline {
	return &proto.EventEnabledPipeline{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
	}
}

type EventCreatedPipeline struct {
	Metadata
	NamespaceID string
	PipelineID  string
}

func NewEventCreatedPipeline(pipeline Pipeline) *EventCreatedPipeline {
	return &EventCreatedPipeline{
		Metadata:    NewMetadata(CreatedPipelineEvent),
		NamespaceID: pipeline.Namespace,
		PipelineID:  pipeline.ID,
	}
}

func (e *EventCreatedPipeline) ToProto() *proto.EventCreatedPipeline {
	return &proto.EventCreatedPipeline{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
	}
}

type EventAbandonedPipeline struct {
	Metadata
	NamespaceID string
	PipelineID  string
}

func NewEventAbandonedPipeline(pipeline Pipeline) *EventAbandonedPipeline {
	return &EventAbandonedPipeline{
		Metadata:    NewMetadata(AbandonedPipelineEvent),
		NamespaceID: pipeline.Namespace,
		PipelineID:  pipeline.ID,
	}
}

func (e *EventAbandonedPipeline) ToProto() *proto.EventAbandonedPipeline {
	return &proto.EventAbandonedPipeline{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
	}
}

type EventCreatedNamespace struct {
	Metadata
	NamespaceID string
}

func NewEventCreatedNamespace(namespace Namespace) *EventCreatedNamespace {
	return &EventCreatedNamespace{
		Metadata:    NewMetadata(CreatedNamespaceEvent),
		NamespaceID: namespace.ID,
	}
}

func (e *EventCreatedNamespace) ToProto() *proto.EventCreatedNamespace {
	return &proto.EventCreatedNamespace{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
	}
}

type EventStartedRun struct {
	Metadata
	NamespaceID string
	PipelineID  string
	RunID       int64
}

func NewEventStartedRun(run Run) *EventStartedRun {
	return &EventStartedRun{
		Metadata:    NewMetadata(StartedRunEvent),
		NamespaceID: run.NamespaceID,
		PipelineID:  run.PipelineID,
		RunID:       run.ID,
	}
}

func (e *EventStartedRun) ToProto() *proto.EventStartedRun {
	return &proto.EventStartedRun{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
		RunId:       e.RunID,
	}
}

type EventCompletedRun struct {
	Metadata
	NamespaceID string
	PipelineID  string
	RunID       int64
	State       RunState
}

func NewEventCompletedRun(run Run) *EventCompletedRun {
	return &EventCompletedRun{
		Metadata:    NewMetadata(CompletedRunEvent),
		NamespaceID: run.NamespaceID,
		PipelineID:  run.PipelineID,
		RunID:       run.ID,
		State:       run.State,
	}
}

func (e *EventCompletedRun) ToProto() *proto.EventCompletedRun {
	return &proto.EventCompletedRun{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
		RunId:       e.RunID,
		State:       proto.EventCompletedRun_State(proto.EventCompletedRun_State_value[string(e.State)]),
	}
}

type EventStartedTaskRun struct {
	Metadata
	NamespaceID string
	PipelineID  string
	RunID       int64
	TaskRunID   string
}

func NewEventStartedTaskRun(taskrun TaskRun) *EventStartedTaskRun {
	return &EventStartedTaskRun{
		Metadata:    NewMetadata(StartedTaskRunEvent),
		NamespaceID: taskrun.NamespaceID,
		PipelineID:  taskrun.PipelineID,
		RunID:       taskrun.RunID,
		TaskRunID:   taskrun.ID,
	}
}

func (e *EventStartedTaskRun) ToProto() *proto.EventStartedTaskRun {
	return &proto.EventStartedTaskRun{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
		RunId:       e.RunID,
		TaskRunId:   e.TaskRunID,
	}
}

type EventScheduledTaskRun struct {
	Metadata
	NamespaceID string
	PipelineID  string
	RunID       int64
	TaskRunID   string
}

func NewEventScheduledTaskRun(taskrun TaskRun) *EventScheduledTaskRun {
	return &EventScheduledTaskRun{
		Metadata:    NewMetadata(ScheduledTaskRunEvent),
		NamespaceID: taskrun.NamespaceID,
		PipelineID:  taskrun.PipelineID,
		RunID:       taskrun.RunID,
		TaskRunID:   taskrun.ID,
	}
}

func (e *EventScheduledTaskRun) ToProto() *proto.EventScheduledTaskRun {
	return &proto.EventScheduledTaskRun{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
		RunId:       e.RunID,
		TaskRunId:   e.TaskRunID,
	}
}

type EventCompletedTaskRun struct {
	Metadata
	NamespaceID string
	PipelineID  string
	RunID       int64
	TaskRunID   string
	State       ContainerState
}

func NewEventCompletedTaskRun(taskrun TaskRun) *EventCompletedTaskRun {
	return &EventCompletedTaskRun{
		Metadata:    NewMetadata(CompletedTaskRunEvent),
		NamespaceID: taskrun.NamespaceID,
		PipelineID:  taskrun.PipelineID,
		RunID:       taskrun.RunID,
		TaskRunID:   taskrun.ID,
		State:       taskrun.State,
	}
}

func (e *EventCompletedTaskRun) ToProto() *proto.EventCompletedTaskRun {
	return &proto.EventCompletedTaskRun{
		Metadata:    e.Metadata.ToProto(),
		NamespaceId: e.NamespaceID,
		PipelineId:  e.PipelineID,
		RunId:       e.RunID,
		TaskRunId:   e.TaskRunID,
		State:       proto.EventCompletedTaskRun_State(proto.EventCompletedTaskRun_State_value[string(e.State)]),
	}
}

type EventFiredTrigger struct {
	Metadata
	Label           string
	Pipeline        string
	Namespace       string
	Result          TriggerResult
	TriggerMetadata map[string]string // Environment variables to be passed on to the pending run.
}

func NewEventFiredTrigger(namespace, pipeline, label string, result TriggerResult, metadata map[string]string) *EventFiredTrigger {
	return &EventFiredTrigger{
		Metadata:        NewMetadata(FiredTriggerEvent),
		Namespace:       namespace,
		Pipeline:        pipeline,
		Label:           label,
		Result:          result,
		TriggerMetadata: metadata,
	}
}

func (e *EventFiredTrigger) ToProto() *proto.EventFiredTrigger {
	return &proto.EventFiredTrigger{
		Metadata:        e.Metadata.ToProto(),
		Label:           e.Label,
		Pipeline:        e.Pipeline,
		Namespace:       e.Namespace,
		Result:          e.Result.ToProto(),
		TriggerMetadata: e.TriggerMetadata,
	}
}

type EventProcessedTrigger struct {
	Metadata
	Label           string
	Pipeline        string
	Namespace       string
	Result          TriggerResult
	TriggerMetadata map[string]string // Environment variables to be passed on to the pending run.
}

func NewEventProcessedTrigger(namespace, pipeline, label string, result TriggerResult, metadata map[string]string) *EventProcessedTrigger {
	return &EventProcessedTrigger{
		Metadata:        NewMetadata(ProcessedTriggerEvent),
		Namespace:       namespace,
		Pipeline:        pipeline,
		Label:           label,
		Result:          result,
		TriggerMetadata: metadata,
	}
}

func (e *EventProcessedTrigger) ToProto() *proto.EventProcessedTrigger {
	return &proto.EventProcessedTrigger{
		Metadata:        e.Metadata.ToProto(),
		Label:           e.Label,
		Pipeline:        e.Pipeline,
		Namespace:       e.Namespace,
		Result:          e.Result.ToProto(),
		TriggerMetadata: e.TriggerMetadata,
	}
}

type EventResolvedTrigger struct {
	Metadata
	Label           string
	Pipeline        string
	Namespace       string
	Result          TriggerResult
	TriggerMetadata map[string]string // Environment variables to be passed on to the pending run.
}

func NewEventResolvedTrigger(namespace, pipeline, label string, result TriggerResult, metadata map[string]string) *EventResolvedTrigger {
	return &EventResolvedTrigger{
		Metadata:        NewMetadata(ResolvedTriggerEvent),
		Namespace:       namespace,
		Pipeline:        pipeline,
		Label:           label,
		Result:          result,
		TriggerMetadata: metadata,
	}
}

func (e *EventResolvedTrigger) ToProto() *proto.EventResolvedTrigger {
	return &proto.EventResolvedTrigger{
		Metadata:        e.Metadata.ToProto(),
		Label:           e.Label,
		Pipeline:        e.Pipeline,
		Namespace:       e.Namespace,
		Result:          e.Result.ToProto(),
		TriggerMetadata: e.TriggerMetadata,
	}
}

func (e *EventResolvedTrigger) FromProto(p *proto.EventResolvedTrigger) {
	metadata := Metadata{}
	metadata.FromProto(p.Metadata)

	result := TriggerResult{}
	result.FromProto(p.Result)

	e.Metadata = metadata
	e.Label = p.Label
	e.Pipeline = p.Pipeline
	e.Namespace = p.Namespace
	e.Result = result
	e.TriggerMetadata = p.TriggerMetadata
}
