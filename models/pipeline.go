package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"

	sdk "github.com/clintjedwards/gofer/sdk/go"
)

type PipelineState string

const (
	PipelineStateUnknown  PipelineState = "UNKNOWN"
	PipelineStateActive   PipelineState = "ACTIVE"
	PipelineStateDisabled PipelineState = "DISABLED"
)

type PipelineErrorKind string

const (
	PipelineErrorKindUnknown                    PipelineErrorKind = "UNKNOWN"
	PipelineErrorKindTriggerSubscriptionFailure PipelineErrorKind = "TRIGGER_SUBSCRIPTION_FAILURE"
)

type PipelineError struct {
	Kind        PipelineErrorKind
	Description string
}

func (p *PipelineError) ToProto() *proto.Pipeline_Error {
	return &proto.Pipeline_Error{
		Kind:        proto.Pipeline_ErrorKind(proto.Pipeline_ErrorKind_value[string(p.Kind)]),
		Description: p.Description,
	}
}

func (p *PipelineError) FromProto(proto *proto.Pipeline_Error) {
	p.Kind = PipelineErrorKind(proto.Kind)
	p.Description = proto.Description
}

// / A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
// / Pipeline is a secondary level unit being contained within namespaces and containing runs.
type Pipeline struct {
	Namespace   string `json:"namespace"`   // The namespace this pipeline belongs to.
	ID          string `json:"id"`          // Unique identifier; user defined.
	Name        string `json:"name"`        // Name refers to a human readable pipeline name.
	Description string `json:"description"` // Description of pipeline's purpose and other details.
	/// Controls how many runs can be active at any single time. 0 indicates unbounded with respect to bounds
	/// enforced by Gofer.
	Parallelism int64 `json:"parallelism"`
	Created     int64 `json:"created"` // Time pipeline was created in epoch milliseconds.
	/// The current state of the pipeline. Pipelines can be disabled to stop execution of runs/tasks.
	Modified int64 `json:"modified"`
	// The current running state of the pipeline. This is used to determine if the pipeline should continue to process
	// runs or not and properly convey that to the user.
	State PipelineState   `json:"state"`
	Tasks map[string]Task `json:"tasks"` // Map for quickly finding pipeline tasks and assists with DAG generation.
	// Triggers is a listing of trigger labels to the their trigger subscription objects
	Triggers    map[string]PipelineTriggerSettings    `json:"triggers"`
	CommonTasks map[string]PipelineCommonTaskSettings `json:"common_tasks"`
	// There are certain things that might occur within a pipeline that the user will have to fix to restore full
	// functionality of the pipeline[^1]. Errors is a way to describe to the user which problems their pipeline might
	// have.
	//
	// [^1]: For example, if you turn off Gofer and it restores trigger connections but one trigger is not available anymore
	// Then we would use errors to message the user that this thing that was previously part of your pipeline will not
	// work anymore. These cases should be rare, but are important to get right.
	Errors []PipelineError `json:"errors"`
}

func NewPipeline(namespace string, config *sdk.Pipeline) *Pipeline {
	tasks := map[string]Task{}
	for _, task := range config.Tasks {
		tasks[task.ID] = FromTaskConfig(&task)
	}

	triggers := map[string]PipelineTriggerSettings{}
	for _, trigger := range config.Triggers {
		triggers[trigger.Label] = FromTriggerConfig(&trigger)
	}

	commonTasks := map[string]PipelineCommonTaskSettings{}
	for _, task := range config.CommonTasks {
		commonTasks[task.Label] = FromCommonTaskConfig(&task)
	}

	newPipeline := &Pipeline{
		Namespace:   namespace,
		ID:          config.ID,
		Name:        config.Name,
		Description: config.Description,
		Parallelism: config.Parallelism,
		Created:     time.Now().UnixMilli(),
		Modified:    time.Now().UnixMilli(),
		State:       PipelineStateActive,
		Tasks:       tasks,
		Triggers:    triggers,
		CommonTasks: commonTasks,
		Errors:      []PipelineError{},
	}

	return newPipeline
}

func (p *Pipeline) ToProto() *proto.Pipeline {
	tasks := map[string]*proto.Task{}
	for id, task := range p.Tasks {
		tasks[id] = task.ToProto()
	}

	triggers := map[string]*proto.PipelineTriggerSettings{}
	for label, trigger := range p.Triggers {
		triggers[label] = trigger.ToProto()
	}

	commonTasks := map[string]*proto.PipelineCommonTaskSettings{}
	for label, commonTask := range p.CommonTasks {
		commonTasks[label] = commonTask.ToProto()
	}

	pipelineErrors := []*proto.Pipeline_Error{}
	for _, pipelineError := range p.Errors {
		pipelineErrors = append(pipelineErrors, pipelineError.ToProto())
	}

	return &proto.Pipeline{
		Namespace:   p.Namespace,
		Id:          p.ID,
		Name:        p.Name,
		Description: p.Description,
		Parallelism: p.Parallelism,
		Created:     p.Created,
		Modified:    p.Modified,
		State:       proto.Pipeline_PipelineState(proto.Pipeline_PipelineState_value[string(p.State)]),
		Tasks:       tasks,
		Triggers:    triggers,
		CommonTasks: commonTasks,
		Errors:      pipelineErrors,
	}
}

func (p *Pipeline) FromProto(proto *proto.Pipeline) {
	tasks := map[string]Task{}
	for id, protoTask := range proto.Tasks {
		task := Task{}
		task.FromProto(protoTask)
		tasks[id] = task
	}

	triggers := map[string]PipelineTriggerSettings{}
	for label, trigger := range proto.Triggers {
		settings := PipelineTriggerSettings{}
		settings.FromProto(trigger)
		triggers[label] = settings
	}

	commonTasks := map[string]PipelineCommonTaskSettings{}
	for label, commonTask := range proto.CommonTasks {
		settings := PipelineCommonTaskSettings{}
		settings.FromProto(commonTask)
		commonTasks[label] = settings
	}

	pipelineErrors := []PipelineError{}
	for _, protoPipelineError := range proto.Errors {
		pipelineError := PipelineError{}
		pipelineError.FromProto(protoPipelineError)
		pipelineErrors = append(pipelineErrors, pipelineError)
	}

	p.Namespace = proto.Namespace
	p.ID = proto.Id
	p.Name = proto.Name
	p.Description = proto.Description
	p.Parallelism = proto.Parallelism
	p.Created = proto.Created
	p.Modified = proto.Modified
	p.State = PipelineState(proto.State.String())
	p.Tasks = tasks
	p.Triggers = triggers
	p.CommonTasks = commonTasks
	p.Errors = pipelineErrors
}

// Every time a pipeline attempts to subscribe to a trigger, it passes certain
// values back to that trigger for certain functionality. Since triggers keep no
// permanent state, these settings are kept here so that when triggers are restarted
// they can be restored with proper settings.
type PipelineTriggerSettings struct {
	Name string // A global unique identifier.
	// A user defined identifier for the trigger so that a pipeline with multiple triggers can be differentiated.
	Label    string
	Settings map[string]string
}

func (t *PipelineTriggerSettings) ToProto() *proto.PipelineTriggerSettings {
	return &proto.PipelineTriggerSettings{
		Name:     t.Name,
		Label:    t.Label,
		Settings: t.Settings,
	}
}

func (t *PipelineTriggerSettings) FromProto(p *proto.PipelineTriggerSettings) {
	t.Name = p.Name
	t.Label = p.Label
	t.Settings = p.Settings
}

func FromTriggerConfig(t *sdk.PipelineTriggerConfig) PipelineTriggerSettings {
	return PipelineTriggerSettings{
		Name:     t.Name,
		Label:    t.Label,
		Settings: t.Settings,
	}
}

type PipelineCommonTaskSettings struct {
	Name string // A global unique identifier.
	// A user defined identifier for the common_task so that a pipeline with multiple common_tasks can be differentiated.
	Label    string
	Settings map[string]string
}

func (t *PipelineCommonTaskSettings) ToProto() *proto.PipelineCommonTaskSettings {
	return &proto.PipelineCommonTaskSettings{
		Name:     t.Name,
		Label:    t.Label,
		Settings: t.Settings,
	}
}

func (t *PipelineCommonTaskSettings) FromProto(p *proto.PipelineCommonTaskSettings) {
	t.Name = p.Name
	t.Label = p.Label
	t.Settings = p.Settings
}

func FromCommonTaskConfig(t *sdk.PipelineCommonTaskConfig) PipelineCommonTaskSettings {
	return PipelineCommonTaskSettings{
		Name:     t.Name,
		Label:    t.Label,
		Settings: t.Settings,
	}
}
