package models

import (
	"time"

	"github.com/clintjedwards/gofer/proto"
)

type PipelineState string

const (
	PipelineStateUnknown  PipelineState = "UNKNOWN"
	PipelineStateActive   PipelineState = "ACTIVE"
	PipelineStateDisabled PipelineState = "DISABLED"

	// When a pipeline is abandoned it unsubscribes from all triggers and can no longer be used.
	PipelineStateAbandoned PipelineState = "ABANDONED"
)

// Pipeline is a representation of a collection of logically grouped tasks. A task is a unit of work wrapped in a
// docker container. Pipeline is a secondary level unit being contained within pipelines and containing tasks.
type Pipeline struct {
	// Location points to where the pipeline might have been received from. Gofer has the ability to get pipeline files
	// via URL or by bytes.
	//
	// If by URL, the value accepted here conforms to the hashicorp go-getter URL formatter:
	// https://github.com/hashicorp/go-getter#protocol-specific-options
	Location string `json:"location"`

	Created     int64  `json:"created" storm:"index"` // Time pipeline was created in epoch milliseconds.
	Description string `json:"description"`           // Description of pipeline's purpose and other details.
	ID          string `json:"id" storm:"id"`         // Unique identifier; user defined.
	Namespace   string `json:"namespace"`             // The namespace this pipeline belongs to.
	LastRunTime int64  `json:"last_run_time"`         // Last time a run was triggered for this pipeline.
	LastRunID   int64  `json:"last_run_id"`           // The ID of the most recent run.
	Updated     int64  `json:"updated"`               // Modified time in epoch millisecs. User initiated changes only.
	Name        string `json:"name"`                  // Name refers to a human readable pipeline name.
	Sequential  bool   `json:"sequential"`            // Only run one job at a time.
	// The current running state of the pipeline. This is used to determine if the pipeline should continue to process
	// runs or not and properly convey that to the user.
	State PipelineState   `json:"state"`
	Tasks map[string]Task `json:"tasks"` // Map for quickly finding pipeline tasks and assists with DAG generation.

	// Triggers is a listing of trigger labels to the their trigger subscription objects
	Triggers map[string]PipelineTriggerConfig `json:"triggers"`
	Objects  []string                         `json:"objects"` // Object keys that are stored at the pipeline level.
}

func NewPipeline(location string, pipelineConfig *PipelineConfig) *Pipeline {
	newPipeline := &Pipeline{
		Location: location,
		Created:  time.Now().UnixMilli(),
		Updated:  time.Now().UnixMilli(),
		Tasks:    map[string]Task{},
		Triggers: map[string]PipelineTriggerConfig{},
		Objects:  []string{},
	}

	newPipeline.FromConfig(pipelineConfig)

	return newPipeline
}

func (p *Pipeline) FromConfig(config *PipelineConfig) {
	p.ID = config.ID
	p.Name = config.Name
	p.Description = config.Description
	p.Namespace = config.Namespace
	p.Updated = time.Now().UnixMilli()
	p.Sequential = config.Sequential

	p.Triggers = map[string]PipelineTriggerConfig{}
	for _, trigger := range config.Triggers {
		p.Triggers[trigger.Label] = trigger
	}

	p.Tasks = map[string]Task{}
	for _, task := range config.Tasks {
		p.Tasks[task.ID] = task
	}
}

func (p *Pipeline) ToProto() *proto.Pipeline {
	tasks := map[string]*proto.Task{}
	for id, task := range p.Tasks {
		dependson := map[string]proto.TaskRequiredParentState{}
		for name, state := range task.DependsOn {
			dependson[name] = proto.TaskRequiredParentState(proto.TaskRequiredParentState_value[string(state)])
		}

		tasks[id] = &proto.Task{
			Id:          task.ID,
			Description: task.Description,
			Image:       task.Image,
			DependsOn:   dependson,
			EnvVars:     task.EnvVars,
		}
	}

	triggers := map[string]*proto.PipelineTriggerConfig{}
	for label, trigger := range p.Triggers {
		triggers[label] = &proto.PipelineTriggerConfig{
			Config: trigger.Config,
			Kind:   trigger.Kind,
			Label:  trigger.Label,
			State:  proto.PipelineTriggerConfig_State(proto.PipelineTriggerConfig_State_value[string(trigger.State)]),
		}
	}

	return &proto.Pipeline{
		Location:    p.Location,
		Created:     p.Created,
		Description: p.Description,
		Id:          p.ID,
		LastRunTime: p.LastRunTime,
		LastRunId:   p.LastRunID,
		Updated:     p.Updated,
		Name:        p.Name,
		Sequential:  p.Sequential,
		State:       proto.Pipeline_State(proto.Pipeline_State_value[string(p.State)]),
		Tasks:       tasks,
		Triggers:    triggers,
		Namespace:   p.Namespace,
		Objects:     p.Objects,
	}
}

func (p *Pipeline) FromProto(proto *proto.Pipeline) {
	p.Location = proto.Location
	p.Created = proto.Created
	p.Description = proto.Description
	p.ID = proto.Id
	p.LastRunTime = proto.LastRunTime
	p.LastRunID = proto.LastRunId
	p.Updated = proto.Updated
	p.Name = proto.Name
	p.Sequential = proto.Sequential
	p.State = PipelineState(proto.State.String())
	p.Tasks = map[string]Task{}
	p.Namespace = proto.Namespace
	p.Objects = proto.Objects
	for id, task := range proto.Tasks {
		dependson := map[string]RequiredParentState{}
		for name, state := range task.DependsOn {
			dependson[name] = RequiredParentState(state.String())
		}

		p.Tasks[id] = Task{
			ID:          task.Id,
			Description: task.Description,
			Image:       task.Image,
			DependsOn:   dependson,
			EnvVars:     task.EnvVars,
		}
	}
	for label, trigger := range proto.Triggers {
		p.Triggers[label] = PipelineTriggerConfig{
			Label:  trigger.Label,
			Config: trigger.Config,
			Kind:   trigger.Kind,
			State:  PipelineTriggerState(trigger.State.String()),
		}
	}
}

// IsOperational returns whether or not the pipeline is in a state capable of launching new runs.
func (p *Pipeline) IsOperational() bool {
	return p.State == PipelineStateActive
}
