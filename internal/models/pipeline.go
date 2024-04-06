package models

import (
	"fmt"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	sdk "github.com/clintjedwards/gofer/sdk/go/config"
	"github.com/rs/zerolog/log"
)

// A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
// Pipeline is a secondary level unit being contained within namespaces and containing runs.
type Pipeline struct {
	Metadata PipelineMetadata `json:"metadata" doc:"Macro-level details on the targeted pipeline"`
	Config   PipelineConfig   `json:"config" doc:"User controlled data on the targeted pipeline"`
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
	NamespaceID string        `json:"namespace_id" example:"default" doc:"Unique identifier of the target namespace"`
	PipelineID  string        `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline; user supplied"`
	Created     uint64        `json:"created" example:"1712433802634" doc:"Time of pipeline creation in epoch milliseconds"`
	Modified    uint64        `json:"modified" example:"1712433802634" doc:"Time pipeline was updated to a new version in epoch milliseconds."`
	State       PipelineState `json:"state" example:"ACTIVE" doc:"The current running state of the pipeline. This is used to determine if the pipeline should continue to process runs or not and properly convey that to the user."`
}

func NewPipelineMetadata(namespace, id string) *PipelineMetadata {
	newPipelineMetadata := &PipelineMetadata{
		NamespaceID: namespace,
		PipelineID:  id,
		Created:     uint64(time.Now().UnixMilli()),
		Modified:    uint64(time.Now().UnixMilli()),
		State:       PipelineStateActive,
	}

	return newPipelineMetadata
}

func (p *PipelineMetadata) ToStorage() *storage.PipelineMetadata {
	return &storage.PipelineMetadata{
		Namespace: p.NamespaceID,
		ID:        p.PipelineID,
		Created:   fmt.Sprint(p.Created),
		Modified:  fmt.Sprint(p.Modified),
		State:     string(p.State),
	}
}

func (p *PipelineMetadata) FromStorage(sp *storage.PipelineMetadata) {
	created, err := strconv.ParseUint(sp.Created, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	modified, err := strconv.ParseUint(sp.Modified, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	p.NamespaceID = sp.Namespace
	p.PipelineID = sp.ID
	p.Created = created
	p.Modified = modified
	p.State = PipelineState(sp.State)
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
	NamespaceID string              `json:"namespace_id" example:"default" doc:"Unique identifier of the target namespace"`
	PipelineID  string              `json:"pipeline_id" example:"simple_pipeline" doc:"Unique identifier for the target pipeline; user supplied"`
	Version     int64               `json:"version" example:"42" doc:"The current version of this pipeline configuration"`
	Parallelism int64               `json:"parallelism" example:"5" doc:"The amount of runs allowed to happen at any given time"`
	Name        string              `json:"name" example:"Simple Pipeline" doc:"Human readable name for pipeline"`
	Description string              `json:"description" example:"Some description here" doc:"Description of pipeline's purpose and other details"`
	Tasks       map[string]Task     `json:"tasks" doc:"Tasks associated with this pipeline config"`
	State       PipelineConfigState `json:"state" example:"LIVE" doc:"The deployment state of the config. This is used to determine if the pipeline should continue to process runs or not and properly convey that to the user"`
	Registered  uint64              `json:"registered" example:"1712433802634" doc:"Time in epoch milliseconds when this pipeline config was registered"`
	Deprecated  uint64              `json:"deprecated" example:"1712433802634" doc:"The time in epoch milliseconds when the pipeline config was marked deprecated. This helps us figure out which version is actually most recent and which are defunct"`
}

func NewPipelineConfig(namespace, pipeline string, version int64, config *sdk.UserPipelineConfig) *PipelineConfig {
	tasks := map[string]Task{}

	for _, taskRaw := range config.Tasks {
		ct := Task{}
		ct.FromSDKUserPipelineTaskConfig(taskRaw)
		tasks[ct.ID] = ct
	}

	return &PipelineConfig{
		NamespaceID: namespace,
		PipelineID:  pipeline,
		Version:     version,
		Parallelism: config.Parallelism,
		Name:        config.Name,
		Description: config.Description,
		Tasks:       tasks,
		State:       PipelineConfigStateUnreleased,
		Registered:  uint64(time.Now().UnixMilli()),
		Deprecated:  0,
	}
}

func (pc *PipelineConfig) ToStorage() (*storage.PipelineConfig, []*storage.PipelineTask) {
	pipelineConfig := &storage.PipelineConfig{
		Namespace:   pc.NamespaceID,
		Pipeline:    pc.PipelineID,
		Version:     pc.Version,
		Parallelism: pc.Parallelism,
		Name:        pc.Name,
		Description: pc.Description,
		Registered:  fmt.Sprint(pc.Registered),
		Deprecated:  fmt.Sprint(pc.Deprecated),
		State:       string(pc.State),
	}

	tasks := []*storage.PipelineTask{}

	for _, task := range pc.Tasks {
		tasks = append(tasks, task.ToStorage(pc.NamespaceID, pc.PipelineID, pc.Version))
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

	registered, err := strconv.ParseUint(spc.Registered, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	deprecated, err := strconv.ParseUint(spc.Deprecated, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	pc.NamespaceID = spc.Namespace
	pc.PipelineID = spc.Pipeline
	pc.Version = spc.Version
	pc.Parallelism = spc.Parallelism
	pc.Name = spc.Name
	pc.Description = spc.Description
	pc.Tasks = tasks
	pc.State = PipelineConfigState(spc.State)
	pc.Registered = registered
	pc.Deprecated = deprecated
}
