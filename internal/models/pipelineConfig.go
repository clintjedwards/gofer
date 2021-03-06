package models

import (
	"encoding/base64"
	"errors"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/dag"
	validation "github.com/go-ozzo/ozzo-validation/v4"
	"github.com/go-ozzo/ozzo-validation/v4/is"
	"github.com/hashicorp/go-multierror"
	"github.com/hashicorp/hcl/v2"
	"github.com/hashicorp/hcl/v2/hclsimple"
)

type HCLPipelineRegistryAuthConfig struct {
	User string `json:"user" hcl:"user,optional"`
	Pass string `json:"pass" hcl:"pass,optional"`
}

type HCLPipelineExecConfig struct {
	Shell  string `json:"shell" hcl:"shell,label"`
	Script string `json:"script" hcl:"script"`
}

type HCLPipelineTaskConfig struct {
	ID          string                 `json:"id" hcl:"id,label"`
	ImageName   string                 `json:"image_name" hcl:"image_name,label"`
	Description string                 `json:"description" hcl:"description,optional"`
	DependsOn   map[string]string      `json:"depends_on" hcl:"depends_on,optional"`
	EnvVars     map[string]string      `json:"env_vars" hcl:"env_vars,optional"`
	Exec        *HCLPipelineExecConfig `json:"exec" hcl:"exec,block"`

	// HCLv2 has many idiosyncrasies, but this one is noteworthy for future reference. The only way to make a block
	// optional is to make the type a reference to the real struct and then make sure to check for the possible nil.
	RegistryAuth *HCLPipelineRegistryAuthConfig `json:"registry_auth" hcl:"registry_auth,block"`
}

func (config *HCLPipelineTaskConfig) Validate() error {
	for key := range config.EnvVars {
		key := strings.TrimSpace(key)
		if key == "" {
			return fmt.Errorf("envvar key fields cannot be empty")
		}
	}

	configDeref := *config
	return validation.ValidateStruct(&configDeref,
		// Name cannot be empty, greater than 70 chars, and must not contain spaces/special chars etc.
		validation.Field(&configDeref.ID, validation.Required, validation.Length(1, 80), validation.By(isRestrictedCharSet)),
		// Description cannot be greater than 3000 chars
		validation.Field(&configDeref.Description, validation.Length(0, 3000)),
		validation.Field(&configDeref.ImageName, validation.Required),
	)
}

// HCLPipelineTriggerConfig is a representation of a trigger within the pipeline configuration.
// There could be more than one trigger.
type HCLPipelineTriggerConfig struct {
	Kind   string         `hcl:"kind,label"`    // The trigger name/id.
	Label  string         `hcl:"label,label"`   // the user defined name for the trigger.
	Config hcl.Attributes `hcl:"config,remain"` // Any configuration the trigger might need per pipeline.
}

func (config *HCLPipelineTriggerConfig) Validate() error {
	configDeref := *config
	return validation.ValidateStruct(&configDeref,
		// Name cannot be empty, greater than 70 chars, and must not contain spaces/special chars etc.
		validation.Field(&configDeref.Label, validation.Required, validation.Length(1, 80), validation.By(isRestrictedCharSet)),
		// Kind cannot be empty, greater than 70 chars, and must not contain spaces/special chars etc.
		validation.Field(&configDeref.Kind, validation.Required, validation.Length(1, 80), validation.By(isRestrictedCharSet)),
	)
}

// HCLPipelineNotifierConfig is a representation of a notifier within the pipeline configuration.
// There could be more than one notifier.
type HCLPipelineNotifierConfig struct {
	Kind   string         `hcl:"kind,label"`    // The notifier name/id.
	Label  string         `hcl:"label,label"`   // Custom identifier for notifier.
	Config hcl.Attributes `hcl:"config,remain"` // Any configuration the notifier might need per pipeline.
}

func (config *HCLPipelineNotifierConfig) Validate() error {
	configDeref := *config
	return validation.ValidateStruct(&configDeref,
		// Name cannot be empty, greater than 70 chars, and must not contain spaces/special chars etc.
		validation.Field(&configDeref.Label, validation.Required, validation.Length(1, 80), validation.By(isRestrictedCharSet)),
		// Kind cannot be empty, greater than 70 chars, and must not contain spaces/special chars etc.
		validation.Field(&configDeref.Kind, validation.Required, validation.Length(1, 80), validation.By(isRestrictedCharSet)),
	)
}

// HCLPipelineConfig represents the structure of a pipeline configuration file in HCL form.
type HCLPipelineConfig struct {
	ID          string                      `hcl:"id"`
	Description string                      `hcl:"description,optional"`
	Name        string                      `hcl:"name"`
	Namespace   string                      `hcl:"namespace,optional"`  // Namespace pipeline will belong to, if empty is set to "default".
	Sequential  bool                        `hcl:"sequential,optional"` // Restrict pipeline to only one run at a time.
	Tasks       []HCLPipelineTaskConfig     `hcl:"task,block"`          // Each task represents a unit of work wrapped in a docker container.
	Triggers    []HCLPipelineTriggerConfig  `hcl:"trigger,block"`       // Each trigger represents an automated way to start a pipeline.
	Notifiers   []HCLPipelineNotifierConfig `hcl:"notify,block"`        // Each notifier represents a pipeline configured communication method.
}

// Validate examines the HCL pipeline configuration to make sure it adheres to best practices and formatting mistakes.
func (config *HCLPipelineConfig) Validate() error {
	var result error

	// 1) Check for basic input validation.
	configDeref := *config
	err := validation.ValidateStruct(&configDeref,
		// ID cannot be empty, greater than 70 chars.
		validation.Field(&configDeref.ID, validation.Required, validation.Length(1, 70), validation.By(isRestrictedCharSet)),
		// Name cannot be empty, greater than 70 chars.
		validation.Field(&configDeref.Name, validation.Required, validation.Length(1, 70), is.PrintableASCII),
		// Description cannot be greater than 3000 chars.
		validation.Field(&configDeref.Description, validation.Length(0, 3000)),
		// Can not have no tasks.
		validation.Field(&configDeref.Tasks, validation.Length(1, 0)),
	)
	if err != nil {
		result = multierror.Append(result, err)
	}

	// 2) Tasks for basic input validation, unique naming, and DAG cycles.
	for _, task := range config.Tasks {
		err = task.Validate()
		if err != nil {
			result = multierror.Append(result, err)
		}
	}

	err = isDAG(config.Tasks)
	if err != nil {
		result = multierror.Append(result, err)
	}

	// 3) Check triggers for basic input validation and unique naming
	triggerSet := map[string]struct{}{}
	for _, trigger := range config.Triggers {
		_, exists := triggerSet[trigger.Label]
		if exists {
			result = multierror.Append(result, fmt.Errorf("trigger ids must be unique"))
		}

		triggerSet[trigger.Label] = struct{}{}
		err = trigger.Validate()
		if err != nil {
			result = multierror.Append(result, err)
		}
	}

	// 3) Check notifiers for basic input validation and unique naming
	notifierSet := map[string]struct{}{}
	for _, notifier := range config.Notifiers {
		_, exists := notifierSet[notifier.Label]
		if exists {
			result = multierror.Append(result, fmt.Errorf("notifier ids must be unique"))
		}

		notifierSet[notifier.Label] = struct{}{}
		err = notifier.Validate()
		if err != nil {
			result = multierror.Append(result, err)
		}
	}

	return result
}

func isRestrictedCharSet(value interface{}) error {
	s, _ := value.(string)

	for _, char := range s {
		if !(char >= 'a' && char <= 'z') &&
			!(char >= 'A' && char <= 'Z') &&
			!(char >= '0' && char <= '9') &&
			char != '_' {
			if char == ' ' {
				return fmt.Errorf("spaces are not allowed")
			}
			return fmt.Errorf("char %q not allowed", char)
		}
	}

	return nil
}

// PipelineConfig is the representation of pipeline configuration without HCL elements.
type PipelineConfig struct {
	ID          string
	Description string
	Name        string
	Namespace   string                   // Unique ID for namespace pipeline will belong to.
	Sequential  bool                     // Restrict pipeline to only one run at a time.
	Tasks       []Task                   // Each task represents a unit of work wrapped in a docker container.
	Triggers    []PipelineTriggerConfig  // Each trigger represents an automated way to start a pipeline.
	Notifiers   []PipelineNotifierConfig // Each notifier represents a communication method defined by a pipeline.
}

type PipelineTriggerState string

const (
	PipelineTriggerStateUnknown     PipelineTriggerState = "UNKNOWN"
	PipelineTriggerStateActive      PipelineTriggerState = "ACTIVE"
	PipelineTriggerStateDisabled    PipelineTriggerState = "DISABLED"
	PipelineTriggerStateUnsupported PipelineTriggerState = "UNSUPPORTED"
)

// PipelineTriggerConfig is the representation of the pipeline trigger configuration without HCL elements.
type PipelineTriggerConfig struct {
	Kind   string            // The trigger name/id.
	Label  string            // Custom identifier for the trigger.
	Config map[string]string // Any configuration the trigger might need per pipeline.
	// Trigger subscriptions might be unable to be created, in this instance we need to set the
	// state of the trigger so that the user can understand why there might be a problem with their pipeline.
	State PipelineTriggerState
}

// PipelineNotifierConfig is the representation of the pipeline notifier configuration without HCL elements.
type PipelineNotifierConfig struct {
	Kind   string            // The notifier name/id.
	Label  string            // Custom identifier for the notifier.
	Config map[string]string // Any configuration the notifier might need per pipeline.
}

// FromHCL returns a normal config struct from a given HCLConfig struct
func FromHCL(hcl *HCLPipelineConfig) (*PipelineConfig, error) {
	triggers := []PipelineTriggerConfig{}

	for _, trigger := range hcl.Triggers {
		triggerConfig := map[string]string{}

		for key, attr := range trigger.Config {
			value, err := attr.Expr.Value(nil)
			if err.HasErrors() {
				return nil, fmt.Errorf("could not parse HCL; %w", err)
			}

			triggerConfig[key] = value.AsString()
		}

		triggers = append(triggers, PipelineTriggerConfig{
			Kind:   trigger.Kind,
			Label:  trigger.Label,
			Config: triggerConfig,
			State:  PipelineTriggerStateActive,
		})
	}

	notifiers := []PipelineNotifierConfig{}

	for _, notifier := range hcl.Notifiers {
		notifierConfig := map[string]string{}

		for key, attr := range notifier.Config {
			value, err := attr.Expr.Value(nil)
			if err.HasErrors() {
				return nil, fmt.Errorf("could not parse HCL; %w", err)
			}

			notifierConfig[key] = value.AsString()
		}

		notifiers = append(notifiers, PipelineNotifierConfig{
			Kind:   notifier.Kind,
			Label:  notifier.Label,
			Config: notifierConfig,
		})
	}

	tasks := []Task{}
	for _, task := range hcl.Tasks {
		dependson := map[string]RequiredParentState{}
		for key, value := range task.DependsOn {
			dependson[key] = RequiredParentState(strings.ToUpper(value))
		}

		registryAuth := RegistryAuth{}
		if task.RegistryAuth != nil {
			registryAuth = RegistryAuth(*task.RegistryAuth)
		}

		exec := Exec{}
		if task.Exec != nil {
			exec = Exec(*task.Exec)
		}

		// Encode shell script into base64 so it's easy to store and pass around.
		exec.Script = base64.StdEncoding.EncodeToString([]byte(exec.Script))

		tasks = append(tasks, Task{
			ID:           task.ID,
			Description:  strings.TrimSpace(task.Description),
			Image:        task.ImageName,
			RegistryAuth: registryAuth,
			DependsOn:    dependson,
			EnvVars:      task.EnvVars,
			Exec:         exec,
		})
	}

	return &PipelineConfig{
		ID:          hcl.ID,
		Description: strings.TrimSpace(hcl.Description),
		Name:        strings.TrimSpace(hcl.Name),
		Namespace:   hcl.Namespace,
		Sequential:  hcl.Sequential,
		Tasks:       tasks,
		Triggers:    triggers,
		Notifiers:   notifiers,
	}, nil
}

// FromBytes attempts to parse a given HCL configuration. The filename param is for passing back to the user
// on error.
func (config *HCLPipelineConfig) FromBytes(content []byte, filename string) error {
	err := hclsimple.Decode(filename, content, nil, config)
	if err != nil {
		return fmt.Errorf("could not parse file: %w", err)
	}

	return nil
}

// isDAG validates whether given task list represents an acyclic graph.
func isDAG(tasks []HCLPipelineTaskConfig) error {
	taskDAG := dag.New()

	// Add all nodes to the DAG first
	for _, task := range tasks {
		err := taskDAG.AddNode(task.ID)
		if err != nil {
			if errors.Is(err, dag.ErrEntityExists) {
				return fmt.Errorf("duplicate task names found; %q is already a task", task.ID)
			}
			return err
		}
	}

	// Add all edges
	for _, task := range tasks {
		for id := range task.DependsOn {
			err := taskDAG.AddEdge(id, task.ID)
			if err != nil {
				if errors.Is(err, dag.ErrEdgeCreatesCycle) {
					return fmt.Errorf("a cycle was detected creating a dependency from task %q to task %q", task.ID, id)
				}
				if errors.Is(err, dag.ErrEntityNotFound) {
					return fmt.Errorf("task %q is listed as a dependency within task %q but does not exist", id, task.ID)
				}
				return err
			}
		}
	}

	return nil
}
