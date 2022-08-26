package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
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
	State PipelineState `json:"state"`
	// Map for quickly finding user created pipeline tasks; assists with DAG generation.
	CustomTasks map[string]CustomTask `json:"custom_tasks"`
	// Map for quickly finding gofer provided pipeline tasks; assists with DAG generation.
	CommonTasks map[string]PipelineCommonTaskSettings `json:"common_tasks"`
	// Triggers is a listing of trigger labels to the their trigger subscription objects
	Triggers map[string]PipelineTriggerSettings `json:"triggers"`
	// There are certain things that might occur within a pipeline that the user will have to fix to restore full
	// functionality of the pipeline[^1]. Errors is a way to describe to the user which problems their pipeline might
	// have.
	//
	// [^1]: For example, if you turn off Gofer and it restores trigger connections but one trigger is not available anymore
	// Then we would use errors to message the user that this thing that was previously part of your pipeline will not
	// work anymore. These cases should be rare, but are important to get right.
	Errors []PipelineError `json:"errors"`
}

func NewPipeline(namespace string, pb *proto.PipelineConfig) *Pipeline {
	customTasks := map[string]CustomTask{}
	commonTasks := map[string]PipelineCommonTaskSettings{}

	for _, task := range pb.Tasks {
		switch t := task.Task.(type) {
		case *proto.PipelineTaskConfig_CustomTask:
			ct := CustomTask{}
			ct.FromProtoCustomTaskConfig(t.CustomTask)
			customTasks[t.CustomTask.Id] = ct
		case *proto.PipelineTaskConfig_CommonTask:
			ct := PipelineCommonTaskSettings{}
			ct.FromProtoCommonTaskConfig(t.CommonTask)
			commonTasks[ct.Label] = ct
		}
	}

	triggers := map[string]PipelineTriggerSettings{}
	for _, trigger := range pb.Triggers {
		triggerConfig := PipelineTriggerSettings{}
		triggerConfig.FromProtoTriggerConfig(trigger)
		triggers[trigger.Label] = triggerConfig
	}

	newPipeline := &Pipeline{
		Namespace:   namespace,
		ID:          pb.Id,
		Name:        pb.Name,
		Description: pb.Description,
		Parallelism: pb.Parallelism,
		Created:     time.Now().UnixMilli(),
		Modified:    time.Now().UnixMilli(),
		State:       PipelineStateActive,
		CustomTasks: customTasks,
		Triggers:    triggers,
		CommonTasks: commonTasks,
		Errors:      []PipelineError{},
	}

	return newPipeline
}

func (p *Pipeline) ToProto() *proto.Pipeline {
	customTasks := map[string]*proto.CustomTask{}
	for id, task := range p.CustomTasks {
		customTasks[id] = task.ToProto()
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
		CustomTasks: customTasks,
		Triggers:    triggers,
		CommonTasks: commonTasks,
		Errors:      pipelineErrors,
	}
}

func (p *Pipeline) FromProto(proto *proto.Pipeline) {
	customTasks := map[string]CustomTask{}
	for id, protoCustomTask := range proto.CustomTasks {
		customTask := CustomTask{}
		customTask.FromProto(protoCustomTask)
		customTasks[id] = customTask
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
	p.CustomTasks = customTasks
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

func (t *PipelineTriggerSettings) FromProtoTriggerConfig(p *proto.PipelineTriggerConfig) {
	t.Name = p.Name
	t.Label = p.Label
	t.Settings = p.Settings
}

type PipelineCommonTaskSettings struct {
	Name string `json:"name"` // A global unique identifier for a specific type of common task.
	// A user defined identifier for the common_task so that a pipeline with multiple common_tasks can be differentiated.
	Label       string                          `json:"label"`
	Description string                          `json:"description"`
	DependsOn   map[string]RequiredParentStatus `json:"depends_on"`
	Settings    map[string]string               `json:"settings"`
}

func (t *PipelineCommonTaskSettings) ToProto() *proto.PipelineCommonTaskSettings {
	dependsOn := map[string]proto.PipelineCommonTaskSettings_RequiredParentStatus{}
	for key, value := range t.DependsOn {
		dependsOn[key] = proto.PipelineCommonTaskSettings_RequiredParentStatus(proto.PipelineCommonTaskSettings_RequiredParentStatus_value[string(value)])
	}

	return &proto.PipelineCommonTaskSettings{
		Name:        t.Name,
		Label:       t.Label,
		Description: t.Description,
		DependsOn:   dependsOn,
		Settings:    t.Settings,
	}
}

func (t *PipelineCommonTaskSettings) FromProtoCommonTaskConfig(p *proto.CommonTaskConfig) {
	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range p.DependsOn {
		dependsOn[id] = RequiredParentStatus(status.String())
	}

	t.Name = p.Name
	t.Label = p.Label
	t.Description = p.Description
	t.DependsOn = dependsOn
	t.Settings = p.Settings
}

func (t *PipelineCommonTaskSettings) FromProto(p *proto.PipelineCommonTaskSettings) {
	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range p.DependsOn {
		dependsOn[id] = RequiredParentStatus(status.String())
	}

	t.Name = p.Name
	t.Label = p.Label
	t.Description = p.Description
	t.DependsOn = dependsOn
	t.Settings = p.Settings
}
