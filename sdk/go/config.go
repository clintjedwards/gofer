package sdk

import (
	"encoding/json"
	"fmt"
	"regexp"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type RequiredParentStatus string

const (
	RequiredParentStatusAny     RequiredParentStatus = "ANY"
	RequiredParentStatusSuccess RequiredParentStatus = "SUCCESS"
	RequiredParentStatusFailure RequiredParentStatus = "FAILURE"
)

func (status RequiredParentStatus) ToString() string {
	switch status {
	case RequiredParentStatusAny:
		return "any"
	case RequiredParentStatusFailure:
		return "failure"
	case RequiredParentStatusSuccess:
		return "success"
	default:
		return "unknown"
	}
}

type RegistryAuth struct {
	User string `json:"user"`
	Pass string `json:"pass"`
}

func (ra *RegistryAuth) FromProto(proto *proto.RegistryAuth) {
	ra.User = proto.User
	ra.Pass = proto.Pass
}

type Task struct {
	ID           string                          `json:"id"`
	Description  string                          `json:"description"`
	Image        string                          `json:"image"`
	RegistryAuth *RegistryAuth                   `json:"registry_auth"`
	DependsOn    map[string]RequiredParentStatus `json:"depends_on"`
	Variables    map[string]string               `json:"variables"`
	Entrypoint   []string                        `json:"entrypoint"`
	Command      []string                        `json:"command"`
}

func NewTask(id, image string) *Task {
	return &Task{
		ID:           id,
		Description:  "",
		Image:        image,
		RegistryAuth: nil,
		DependsOn:    make(map[string]RequiredParentStatus),
		Variables:    make(map[string]string),
		Entrypoint:   []string{},
	}
}

func (t *Task) FromProto(proto *proto.TaskConfig) {
	var registryAuth *RegistryAuth = nil
	if proto.RegistryAuth != nil {
		ra := RegistryAuth{}
		ra.FromProto(proto.RegistryAuth)
		registryAuth = &ra
	}
	dependsOn := map[string]RequiredParentStatus{}
	for id, status := range proto.DependsOn {
		dependsOn[id] = RequiredParentStatus(status)
	}

	t.ID = proto.Id
	t.Description = proto.Description
	t.Image = proto.Image
	t.RegistryAuth = registryAuth
	t.DependsOn = dependsOn
	t.Variables = proto.Variables
	t.Entrypoint = proto.Entrypoint
}

func (t *Task) validate() error {
	return validateIdentifier("id", t.ID)
}

func (t *Task) WithDescription(description string) *Task {
	t.Description = description
	return t
}

func (t *Task) WithRegistryAuth(user, pass string) *Task {
	t.RegistryAuth = &RegistryAuth{
		User: user,
		Pass: pass,
	}
	return t
}

func (t *Task) WithDependsOnOne(taskID string, state RequiredParentStatus) *Task {
	t.DependsOn[taskID] = state
	return t
}

func (t *Task) WithDependsOnMany(dependsOn map[string]RequiredParentStatus) *Task {
	for id, status := range dependsOn {
		t.DependsOn[id] = status
	}
	return t
}

func (t *Task) WithVariable(key, value string) *Task {
	t.Variables[key] = value
	return t
}

func (t *Task) WithVariables(variables map[string]string) *Task {
	for key, value := range variables {
		t.Variables[key] = value
	}
	return t
}

func (t *Task) WithEntrypoint(entrypoint []string) *Task {
	t.Entrypoint = entrypoint
	return t
}

func (t *Task) WithCommand(command []string) *Task {
	t.Command = command
	return t
}

type PipelineTriggerConfig struct {
	Name     string            `json:"name"`
	Label    string            `json:"label"`
	Settings map[string]string `json:"settings"`
}

func (p *PipelineTriggerConfig) FromProto(proto *proto.PipelineTriggerConfig) {
	p.Name = proto.Name
	p.Label = proto.Label
	p.Settings = proto.Settings
}

func (p *PipelineTriggerConfig) validate() error {
	return validateIdentifier("label", p.Label)
}

type PipelineCommonTaskConfig struct {
	Name     string            `json:"name"`
	Label    string            `json:"label"`
	Settings map[string]string `json:"settings"`
}

func (p *PipelineCommonTaskConfig) FromProto(proto *proto.PipelineCommonTaskConfig) {
	p.Name = proto.Name
	p.Label = proto.Label
	p.Settings = proto.Settings
}

func (p *PipelineCommonTaskConfig) validate() error {
	return validateIdentifier("label", p.Label)
}

type Pipeline struct {
	ID          string                     `json:"id"`
	Name        string                     `json:"name"`
	Description string                     `json:"description"`
	Parallelism int64                      `json:"parallelism"`
	Tasks       []Task                     `json:"tasks"`
	Triggers    []PipelineTriggerConfig    `json:"triggers"`
	CommonTasks []PipelineCommonTaskConfig `json:"common_tasks"`
}

func NewPipeline(id, name string) *Pipeline {
	return &Pipeline{
		ID:          id,
		Name:        name,
		Description: "",
		Parallelism: 0,
		Tasks:       []Task{},
		Triggers:    []PipelineTriggerConfig{},
		CommonTasks: []PipelineCommonTaskConfig{},
	}
}

func (p *Pipeline) FromProto(proto *proto.PipelineConfig) {
	tasks := []Task{}
	for _, taskConfig := range proto.Tasks {
		task := Task{}
		task.FromProto(taskConfig)
		tasks = append(tasks, task)
	}

	triggers := []PipelineTriggerConfig{}
	for _, triggerConfig := range proto.Triggers {
		trigger := PipelineTriggerConfig{}
		trigger.FromProto(triggerConfig)
		triggers = append(triggers, trigger)
	}

	commonTasks := []PipelineCommonTaskConfig{}
	for _, commonTaskConfig := range proto.CommonTasks {
		commonTask := PipelineCommonTaskConfig{}
		commonTask.FromProto(commonTaskConfig)
		commonTasks = append(commonTasks, commonTask)
	}

	p.ID = proto.Id
	p.Name = proto.Name
	p.Description = proto.Description
	p.Parallelism = proto.Parallelism
	p.Tasks = tasks
	p.Triggers = triggers
	p.CommonTasks = commonTasks
}

func (p *Pipeline) validate() error {
	err := validateIdentifier("id", p.ID)
	if err != nil {
		return err
	}

	for _, task := range p.Tasks {
		err = task.validate()
		if err != nil {
			return err
		}
	}

	for _, trigger := range p.Triggers {
		err = trigger.validate()
		if err != nil {
			return err
		}
	}

	for _, commontask := range p.CommonTasks {
		err = commontask.validate()
		if err != nil {
			return err
		}
	}
	return nil
}

func (p *Pipeline) WithDescription(description string) *Pipeline {
	p.Description = description
	return p
}

func (p *Pipeline) WithParallelism(parallelism int64) *Pipeline {
	p.Parallelism = parallelism
	return p
}

func (p *Pipeline) WithTasks(tasks []Task) *Pipeline {
	p.Tasks = tasks
	return p
}

func (p *Pipeline) WithTriggers(triggers []PipelineTriggerConfig) *Pipeline {
	p.Triggers = triggers
	return p
}

func (p *Pipeline) WithCommonTasks(commontasks []PipelineCommonTaskConfig) *Pipeline {
	p.CommonTasks = commontasks
	return p
}

// Call finish as the last method to the pipeline config
func (p *Pipeline) Finish() error {
	err := p.validate()
	if err != nil {
		return err
	}

	output, err := json.Marshal(p)
	if err != nil {
		return err
	}

	fmt.Printf("%s", output)

	return nil
}

var alphanumericWithUnderscores = regexp.MustCompile("^[a-zA-Z0-9_]*$")

// / Identifiers are used as the primary key in most of gofer's resources.
// / They're defined by the user and therefore should have some sane bounds.
// / For all ids we'll want the following:
// / * 32 > characters < 3
// / * Only alphanumeric characters or underscores
func validateIdentifier(arg, value string) error {
	if len(value) > 32 {
		return fmt.Errorf("length of arg %q cannot be greater than 32", arg)
	}

	if len(value) < 3 {
		return fmt.Errorf("length of arg %q cannot be less than 3", arg)
	}

	if !alphanumericWithUnderscores.MatchString(value) {
		return fmt.Errorf("can only be made up of alphanumeric and underscore characters")
	}
	return nil
}
