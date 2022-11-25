package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
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
	CustomTasks map[string]CustomTask `json:"custom_tasks"`
	// Map for quickly finding gofer provided pipeline tasks; assists with DAG generation.
	CommonTasks map[string]PipelineCommonTaskSettings `json:"common_tasks"`
	// The current running state of the pipeline. This is used to determine if the pipeline should continue to process
	// runs or not and properly convey that to the user.
	State      PipelineConfigState `json:"state"`
	Registered int64               `json:"registered"`
	// If the pipeline's state is "deprecated" we note the time it was so we know which is the oldest defunct version.
	Deprecated int64 `json:"deprecated"`
}

func NewPipelineConfig(namespace, pipeline string, version int64, pb *proto.UserPipelineConfig) *PipelineConfig {
	customTasks := map[string]CustomTask{}
	commonTasks := map[string]PipelineCommonTaskSettings{}

	for _, task := range pb.Tasks {
		switch t := task.Task.(type) {
		case *proto.UserPipelineTaskConfig_CustomTask:
			ct := CustomTask{}
			ct.FromProtoCustomTaskConfig(t.CustomTask)
			customTasks[t.CustomTask.Id] = ct
		case *proto.UserPipelineTaskConfig_CommonTask:
			ct := PipelineCommonTaskSettings{}
			ct.FromProtoCommonTaskConfig(t.CommonTask)
			commonTasks[ct.Label] = ct
		}
	}

	return &PipelineConfig{
		Namespace:   namespace,
		Pipeline:    pipeline,
		Version:     version,
		Parallelism: pb.Parallelism,
		Name:        pb.Name,
		Description: pb.Description,
		CustomTasks: customTasks,
		CommonTasks: commonTasks,
		State:       PipelineConfigStateUnreleased,
		Registered:  time.Now().UnixMilli(),
		Deprecated:  0,
	}
}

func (pc *PipelineConfig) ToStorage() (*storage.PipelineConfig, []*storage.PipelineCommonTaskSettings, []*storage.PipelineCustomTask) {
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

	commonTaskSettings := []*storage.PipelineCommonTaskSettings{}

	for _, commonTaskSetting := range pc.CommonTasks {
		commonTaskSettings = append(commonTaskSettings, commonTaskSetting.ToStorage(pc.Namespace, pc.Pipeline, pc.Version))
	}

	customTasks := []*storage.PipelineCustomTask{}

	for _, customTask := range pc.CustomTasks {
		customTasks = append(customTasks, customTask.ToStorage(pc.Namespace, pc.Pipeline, pc.Version))
	}

	return pipelineConfig, commonTaskSettings, customTasks
}

func (pc *PipelineConfig) FromStorage(spc *storage.PipelineConfig,
	spcts *[]storage.PipelineCommonTaskSettings, spct *[]storage.PipelineCustomTask,
) {
	customTasks := map[string]CustomTask{}

	for _, customTask := range *spct {
		var ct CustomTask
		ct.FromStorage(&customTask)
		customTasks[customTask.ID] = ct
	}

	commonTasks := map[string]PipelineCommonTaskSettings{}

	for _, commonTask := range *spcts {
		var ct PipelineCommonTaskSettings
		ct.FromStorage(&commonTask)
		commonTasks[commonTask.Label] = ct
	}

	pc.Namespace = spc.Namespace
	pc.Pipeline = spc.Pipeline
	pc.Version = spc.Version
	pc.Parallelism = spc.Parallelism
	pc.Name = spc.Name
	pc.Description = spc.Description
	pc.CustomTasks = customTasks
	pc.CommonTasks = commonTasks
	pc.State = PipelineConfigState(spc.State)
	pc.Registered = spc.Registered
	pc.Deprecated = spc.Deprecated
}

func (pc *PipelineConfig) ToProto() *proto.PipelineConfig {
	customTasks := map[string]*proto.CustomTask{}
	commonTasks := map[string]*proto.PipelineCommonTaskSettings{}

	for _, customTask := range pc.CustomTasks {
		protoCustomTask := customTask.ToProto()
		customTasks[protoCustomTask.Id] = protoCustomTask
	}

	for _, commonTask := range pc.CommonTasks {
		protoCommonTask := commonTask.ToProto()
		commonTasks[protoCommonTask.Label] = protoCommonTask
	}

	return &proto.PipelineConfig{
		Namespace:   pc.Namespace,
		Pipeline:    pc.Pipeline,
		Version:     pc.Version,
		Parallelism: pc.Parallelism,
		Name:        pc.Name,
		Description: pc.Description,
		CustomTasks: customTasks,
		CommonTasks: commonTasks,
		State:       proto.PipelineConfig_PipelineConfigState(proto.PipelineConfig_PipelineConfigState_value[string(pc.State)]),
		Registered:  pc.Registered,
		Deprecated:  pc.Deprecated,
	}
}

type PipelineCommonTaskSettings struct {
	Name string `json:"name"` // A global unique identifier for a specific type of common task.
	// A user defined identifier for the common_task so that a pipeline with multiple common_tasks can be differentiated.
	Label       string                          `json:"label"`
	Description string                          `json:"description"`
	DependsOn   map[string]RequiredParentStatus `json:"depends_on"`
	Settings    map[string]string               `json:"settings"`

	// Allows users to tell gofer to auto-create and inject API Token into task. If this setting is found, Gofer creates
	// an API key for the run (stored in the user's secret store) and then injects it for this run under the
	// environment variables "GOFER_API_TOKEN". This key is automatically cleaned up when Gofer attempts to clean up
	// the Run's objects.
	InjectAPIToken bool `json:"inject_api_token"`
}

func (t *PipelineCommonTaskSettings) ToProto() *proto.PipelineCommonTaskSettings {
	dependsOn := map[string]proto.PipelineCommonTaskSettings_RequiredParentStatus{}
	for key, value := range t.DependsOn {
		dependsOn[key] = proto.PipelineCommonTaskSettings_RequiredParentStatus(proto.PipelineCommonTaskSettings_RequiredParentStatus_value[string(value)])
	}

	return &proto.PipelineCommonTaskSettings{
		Name:           t.Name,
		Label:          t.Label,
		Description:    t.Description,
		DependsOn:      dependsOn,
		Settings:       t.Settings,
		InjectApiToken: t.InjectAPIToken,
	}
}

func (t *PipelineCommonTaskSettings) FromProtoCommonTaskConfig(p *proto.UserCommonTaskConfig) {
	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range p.DependsOn {
		dependsOn[id] = RequiredParentStatus(status.String())
	}

	t.Name = p.Name
	t.Label = p.Label
	t.Description = p.Description
	t.DependsOn = dependsOn
	t.Settings = p.Settings
	t.InjectAPIToken = p.InjectApiToken
}

func (t *PipelineCommonTaskSettings) FromStorage(p *storage.PipelineCommonTaskSettings) {
	var dependsOn map[string]RequiredParentStatus

	err := json.Unmarshal([]byte(p.DependsOn), &dependsOn)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	var settings map[string]string

	err = json.Unmarshal([]byte(p.Settings), &settings)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	t.Name = p.Name
	t.Label = p.Label
	t.Description = p.Description
	t.DependsOn = dependsOn
	t.Settings = settings
	t.InjectAPIToken = p.InjectAPIToken
}

func (t *PipelineCommonTaskSettings) ToStorage(namespace, pipeline string, version int64) *storage.PipelineCommonTaskSettings {
	dependsOn, err := json.Marshal(t.DependsOn)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	settings, err := json.Marshal(t.Settings)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.PipelineCommonTaskSettings{
		Namespace:             namespace,
		Pipeline:              pipeline,
		PipelineConfigVersion: version,
		Name:                  t.Name,
		Label:                 t.Label,
		Description:           t.Description,
		DependsOn:             string(dependsOn),
		Settings:              string(settings),
		InjectAPIToken:        t.InjectAPIToken,
	}
}
