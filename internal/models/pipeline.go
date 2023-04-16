package models

import (
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
)

// A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
// Pipeline is a secondary level unit being contained within namespaces and containing runs.
type Pipeline struct {
	Metadata PipelineMetadata
	Config   PipelineConfig
}

type PipelineState string

const (
	PipelineStateUnknown  PipelineState = "UNKNOWN"
	PipelineStateActive   PipelineState = "ACTIVE"
	PipelineStateDisabled PipelineState = "DISABLED"
)

// Details about the pipeline itself, not including the configuration that the user can change.
// All these values are changed by the system or never changed at all. This sits in contrast to
// the config which the user can change freely.
type PipelineMetadata struct {
	Namespace string `json:"namespace"` // The namespace this pipeline belongs to.
	ID        string `json:"id"`        // Unique identifier; user defined.
	// Controls how many runs can be active at any single time. 0 indicates unbounded with respect to bounds
	// enforced by Gofer.
	Created  int64 `json:"created"`  // Time pipeline was created in epoch milliseconds.
	Modified int64 `json:"modified"` // Time pipeline was updated to a new version in epoch milliseconds.
	// The current running state of the pipeline. This is used to determine if the pipeline should continue to process
	// runs or not and properly convey that to the user.
	State PipelineState `json:"state"`
}

func NewPipelineMetadata(namespace, id string) *PipelineMetadata {
	newPipelineMetadata := &PipelineMetadata{
		Namespace: namespace,
		ID:        id,
		Created:   time.Now().UnixMilli(),
		Modified:  time.Now().UnixMilli(),
		State:     PipelineStateActive,
	}

	return newPipelineMetadata
}

func (p *PipelineMetadata) ToStorage() *storage.PipelineMetadata {
	return &storage.PipelineMetadata{
		Namespace: p.Namespace,
		ID:        p.ID,
		Created:   p.Created,
		Modified:  p.Modified,
		State:     string(p.State),
	}
}

func (p *PipelineMetadata) FromStorage(sp *storage.PipelineMetadata) {
	p.Namespace = sp.Namespace
	p.ID = sp.ID
	p.Created = sp.Created
	p.Modified = sp.Modified
	p.State = PipelineState(sp.State)
}

func (p *PipelineMetadata) ToProto() *proto.PipelineMetadata {
	return &proto.PipelineMetadata{
		Namespace: p.Namespace,
		Id:        p.ID,
		Created:   p.Created,
		Modified:  p.Modified,
		State:     proto.PipelineMetadata_PipelineState(proto.PipelineMetadata_PipelineState_value[string(p.State)]),
	}
}

type PipelineConfigState string

const (
	PipelineConfigStateUnknown    PipelineConfigState = "UNKNOWN"
	PipelineConfigStateUnreleased PipelineConfigState = "UNRELEASED" // Has never been deployed.
	PipelineConfigStateLive       PipelineConfigState = "LIVE"       // Currently deployed.
	PipelineConfigStateDeprecated PipelineConfigState = "DEPRECATED" // Has previously been deployed and is now defunct.
)

// A representation of the user's configuration settings for a particular pipeline.
type PipelineConfig struct {
	Namespace   string `json:"namespace"`
	Pipeline    string `json:"pipeline"`
	Version     int64  `json:"version"`
	Parallelism int64  `json:"parallelism"`
	Name        string `json:"name"`        // Name refers to a human readable pipeline name.
	Description string `json:"description"` // Description of pipeline's purpose and other details.
	// Map for quickly finding user created pipeline tasks; assists with DAG generation.
	Tasks map[string]Task `json:"tasks"`
	// The current running state of the pipeline. This is used to determine if the pipeline should continue to process
	// runs or not and properly convey that to the user.
	State      PipelineConfigState `json:"state"`
	Registered int64               `json:"registered"`
	// If the pipeline's state is "deprecated" we note the time it was so we know which is the oldest defunct version.
	Deprecated int64 `json:"deprecated"`
}

func NewPipelineConfig(namespace, pipeline string, version int64, pb *proto.UserPipelineConfig) *PipelineConfig {
	tasks := map[string]Task{}

	for _, taskRaw := range pb.Tasks {
		ct := Task{}
		ct.FromProtoPipelineTaskConfig(taskRaw)
		tasks[ct.ID] = ct
	}

	return &PipelineConfig{
		Namespace:   namespace,
		Pipeline:    pipeline,
		Version:     version,
		Parallelism: pb.Parallelism,
		Name:        pb.Name,
		Description: pb.Description,
		Tasks:       tasks,
		State:       PipelineConfigStateUnreleased,
		Registered:  time.Now().UnixMilli(),
		Deprecated:  0,
	}
}

func (pc *PipelineConfig) ToStorage() (*storage.PipelineConfig, []*storage.PipelineTask) {
	pipelineConfig := &storage.PipelineConfig{
		Namespace:   pc.Namespace,
		Pipeline:    pc.Pipeline,
		Version:     pc.Version,
		Parallelism: pc.Parallelism,
		Name:        pc.Name,
		Description: pc.Description,
		Registered:  pc.Registered,
		Deprecated:  pc.Deprecated,
		State:       string(pc.State),
	}

	tasks := []*storage.PipelineTask{}

	for _, task := range pc.Tasks {
		tasks = append(tasks, task.ToStorage(pc.Namespace, pc.Pipeline, pc.Version))
	}

	return pipelineConfig, tasks
}

func (pc *PipelineConfig) FromStorage(spc *storage.PipelineConfig, spct *[]storage.PipelineTask,
) {
	tasks := map[string]Task{}

	for _, task := range *spct {
		var ct Task
		ct.FromStorage(&task)
		tasks[task.ID] = ct
	}

	pc.Namespace = spc.Namespace
	pc.Pipeline = spc.Pipeline
	pc.Version = spc.Version
	pc.Parallelism = spc.Parallelism
	pc.Name = spc.Name
	pc.Description = spc.Description
	pc.Tasks = tasks
	pc.State = PipelineConfigState(spc.State)
	pc.Registered = spc.Registered
	pc.Deprecated = spc.Deprecated
}

func (pc *PipelineConfig) ToProto() *proto.PipelineConfig {
	tasks := map[string]*proto.Task{}

	for _, task := range pc.Tasks {
		protoTask := task.ToProto()
		tasks[protoTask.Id] = protoTask
	}

	return &proto.PipelineConfig{
		Namespace:   pc.Namespace,
		Pipeline:    pc.Pipeline,
		Version:     pc.Version,
		Parallelism: pc.Parallelism,
		Name:        pc.Name,
		Description: pc.Description,
		Tasks:       tasks,
		State:       proto.PipelineConfig_PipelineConfigState(proto.PipelineConfig_PipelineConfigState_value[string(pc.State)]),
		Registered:  pc.Registered,
		Deprecated:  pc.Deprecated,
	}
}
