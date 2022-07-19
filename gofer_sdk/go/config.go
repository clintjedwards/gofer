package sdk

import (
	"encoding/json"
	"fmt"
	"regexp"
)

type RequiredParentStatus string

const (
	RequiredParentStatusAny     RequiredParentStatus = "ANY"
	RequiredParentStatusSuccess RequiredParentStatus = "SUCCESS"
	RequiredParentStatusFailure RequiredParentStatus = "FAILURE"
)

type RegistryAuth struct {
	User string `json:"user"`
	Pass string `json:"pass"`
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

func (p *PipelineTriggerConfig) validate() error {
	return validateIdentifier("label", p.Label)
}

type PipelineNotifierConfig struct {
	Name     string            `json:"name"`
	Label    string            `json:"label"`
	Settings map[string]string `json:"settings"`
}

func (p *PipelineNotifierConfig) validate() error {
	return validateIdentifier("label", p.Label)
}

type Pipeline struct {
	ID          string                   `json:"id"`
	Name        string                   `json:"name"`
	Description string                   `json:"description"`
	Parallelism uint64                   `json:"parallelism"`
	Tasks       []Task                   `json:"tasks"`
	Triggers    []PipelineTriggerConfig  `json:"triggers"`
	Notifiers   []PipelineNotifierConfig `json:"notifiers"`
}

func NewPipeline(id, name string) *Pipeline {
	return &Pipeline{
		ID:          id,
		Name:        name,
		Description: "",
		Parallelism: 0,
		Tasks:       []Task{},
		Triggers:    []PipelineTriggerConfig{},
		Notifiers:   []PipelineNotifierConfig{},
	}
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

	for _, notifier := range p.Notifiers {
		err = notifier.validate()
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

func (p *Pipeline) WithParallelism(parallelism uint64) *Pipeline {
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

func (p *Pipeline) WithNotifiers(notifiers []PipelineNotifierConfig) *Pipeline {
	p.Notifiers = notifiers
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

/// Identifiers are used as the primary key in most of gofer's resources.
/// They're defined by the user and therefore should have some sane bounds.
/// For all ids we'll want the following:
/// * 32 > characters < 3
/// * Only alphanumeric characters or underscores
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
