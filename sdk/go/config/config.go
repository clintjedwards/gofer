package config

import (
	"encoding/json"
	"errors"
	"fmt"
	"regexp"

	"github.com/clintjedwards/gofer/sdk/go/internal/dag"
)

// RequiredParentStatus is used to describe the state you would like the parent dependency to be in
// before the child is run.
type RequiredParentStatus string

const (
	// Any means that no matter what the state of the parent is at end, the child will run.
	RequiredParentStatusAny RequiredParentStatus = "Any"

	// Success requires that the parent pass with a SUCCESSFUL status before the child will run.
	RequiredParentStatusSuccess RequiredParentStatus = "Success"

	// Failure requires the parent fail in anyway before the child is run.
	RequiredParentStatusFailure RequiredParentStatus = "Failure"
)

// RegistryAuth represents docker repository authentication.
type RegistryAuth struct {
	User string `json:"user" example:"some_user" doc:"Container registry username"`
	Pass string `json:"pass" example:"some_pass" doc:"Container registry password"`
}

// PipelineWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type PipelineWrapper struct {
	Pipeline
}

// A pipeline is a representation of a Gofer pipeline, the structure in which users represent what they
// want to run in Gofer.
type Pipeline struct {
	ID          string         `json:"id" example:"my_pipeline_name" doc:"Unique Identifier for the pipeline"`                       // Unique Identifier for the pipeline.
	Name        string         `json:"name" example:"My Pipeline Name" doc:"Humanized name for the pipeline"`                        // Humanized name for the pipeline.
	Description string         `json:"description" example:"This pipeline is used for x" doc:"A short description for the pipeline"` // A short description for the pipeline.
	Parallelism int64          `json:"parallelism" example:"2" doc:"The total amount of pipelines run allowed at any given time"`    // How many runs are allowed to run at the same time.
	Tasks       []*TaskWrapper `json:"tasks" doc:"The task set of the pipeline. AKA which containers should run"`                    // The task set of the pipeline. AKA which containers should be run.
}

type UserPipelineTaskConfig struct {
	ID             string                          `json:"id" example:"my_pipeline_task_name" doc:"Unique identifier for the task"`
	Description    string                          `json:"description" example:"My pipeline does x" doc:"A short description for the task"`
	Image          string                          `json:"image" example:"ubuntu:latest" doc:"Which container image to run for this specific task"`
	RegistryAuth   *RegistryAuth                   `json:"registry_auth" doc:"Auth credentials for the image's registry"`
	DependsOn      map[string]RequiredParentStatus `json:"depends_on" example:"{\"task_one\":\"SUCCESS\"}"`
	Variables      map[string]string               `json:"variables" example:"{\"APP_VAR_ONE\":\"some_var_value\"}" doc:"Variables which will be passed in as env vars to the task"`
	Entrypoint     []string                        `json:"entrypoint" example:"[\"printenv\"]" doc:"Command to run on init of container; can be overridden"`
	Command        []string                        `json:"command" example:"[\"printenv\"]" doc:"Command to run on init of container; cannot be overridden"`
	InjectAPIToken bool                            `json:"inject_api_token,omitempty" example:"true" doc:"Whether to inject a run specific Gofer API Key. Useful for using Gofer API within the container"`
}

type UserPipelineConfig struct {
	ID          string                    `json:"id" example:"my_pipeline_name" doc:"Unique Identifier for the pipeline"`                       // Unique Identifier for the pipeline.
	Name        string                    `json:"name" example:"My Pipeline Name" doc:"Humanized name for the pipeline"`                        // Humanized name for the pipeline.
	Description string                    `json:"description" example:"This pipeline is used for x" doc:"A short description for the pipeline"` // A short description for the pipeline.
	Parallelism int64                     `json:"parallelism" example:"2" doc:"The total amount of pipelines run allowed at any given time"`    // How many runs are allowed to run at the same time.
	Tasks       []*UserPipelineTaskConfig `json:"tasks" doc:"The task set of the pipeline. AKA which containers should run"`                    // The task set of the pipeline. AKA which containers should be run.
}

// `Pipeline` represents a sequence of tasks, where each task is a discrete unit of work encapsulated within a container.
// This structure allows you to organize and define the workflow for the tasks you want to execute.
//   - The ID must be between 3 and 32 characters long and only alphanumeric, hyphens are the only allowed
//     alphanumeric character.
//     Ex. `simple-pipeline`
//   - The name is a human friendly name to represent the pipeline.
//     Ex. `Simple Pipeline`
//
// # Example
//
// The following example demonstrates how to create a simple pipeline in Gofer, which is familiar to those experienced with CI/CD tooling.
// It outlines how to define a simple task within a pipeline, use a standard Ubuntu container, and execute a basic command.
//
// This simple example serves as a foundation, illustrating the pattern of defining tasks as building blocks of a pipeline.
// In practice, you would create custom containers designed specifically for the tasks in your Gofer workflows,
// keeping your pipeline configuration clean and focused on orchestration rather than embedding complex logic.
//
// ```ignore
//
//	// Create a new pipeline with a name and a descriptive label.
//	Pipeline::new("simple", "Simple Pipeline")
//	    .description("This pipeline demonstrates a simple Gofer pipeline that pulls in a container and runs a command. \
//	                  This pattern will be familiar to those experienced with CI/CD tools. \
//	                  Tasks in this pipeline are individual containers that can depend on other tasks, illustrating the modular nature of Gofer.")
//	    // Adding a single task to the pipeline.
//	    .tasks(vec![
//	        Task::new("simple_task", "ubuntu:latest")
//	            .description("This task uses the Ubuntu container to print a 'Hello World' message.")
//	            .command(vec!["echo".to_string(), "Hello from Gofer!".to_string()])
//	    ])
//	    .finish() // Finalize and validate the pipeline setup.
//	    .unwrap(); // Handle potential errors during pipeline creation.
//
// ```
func NewPipeline(id, name string) *PipelineWrapper {
	return &PipelineWrapper{
		Pipeline{
			ID:          id,
			Name:        name,
			Description: "",
			Parallelism: 0,
			Tasks:       []*TaskWrapper{},
		},
	}
}

// Checks pipeline for common pipeline mistakes and returns an error if found.
func (p *PipelineWrapper) Validate() error {
	err := validateIdentifier("id", p.ID)
	if err != nil {
		return err
	}

	err = p.isDAG()
	if err != nil {
		return err
	}

	for _, task := range p.Pipeline.Tasks {
		err = task.validate()
		if err != nil {
			return err
		}
	}

	return nil
}

// A description allows you to succinctly describe what your pipeline is used for.
func (p *PipelineWrapper) Description(description string) *PipelineWrapper {
	p.Pipeline.Description = description
	return p
}

// How many runs are allowed to happen at the same time. 0 means no-limit.
func (p *PipelineWrapper) Parallelism(parallelism int64) *PipelineWrapper {
	p.Pipeline.Parallelism = parallelism
	return p
}

// Tasks are containers that the pipeline runs.
func (p *PipelineWrapper) Tasks(tasks ...*TaskWrapper) *PipelineWrapper {
	p.Pipeline.Tasks = tasks
	return p
}

func (p *PipelineWrapper) ToUserPipelineConfig() *UserPipelineConfig {
	tasks := []*UserPipelineTaskConfig{}
	for _, task := range p.Pipeline.Tasks {
		tasks = append(tasks, task.ToUserPipelineTaskConfig())
	}

	return &UserPipelineConfig{
		ID:          p.ID,
		Name:        p.Name,
		Description: p.Pipeline.Description,
		Parallelism: p.Pipeline.Parallelism,
		Tasks:       tasks,
	}
}

// Call finish as the last method to the pipeline config. Finish validates and converts the pipeline
// to json so that it can be read in by other programs.
func (p *PipelineWrapper) Finish() error {
	err := p.Validate()
	if err != nil {
		return err
	}

	jsonData, err := json.Marshal(p)
	if err != nil {
		return err
	}

	fmt.Println(string(jsonData))

	return nil
}

// Convenience function to insert pipeline secret values.
func PipelineSecret(key string) string {
	return fmt.Sprintf("pipeline_secret{{%s}}", key)
}

// Convenience function to insert global secret values.
func GlobalSecret(key string) string {
	return fmt.Sprintf("global_secret{{%s}}", key)
}

// Convenience function to insert pipeline object values.
// When pulling objects from the object store Gofer will attempt to stringify the object (utf-8).
// If you need the raw bytes for an object use the Gofer cli.
func PipelineObject(key string) string {
	return fmt.Sprintf("pipeline_object{{%s}}", key)
}

// Convenience function to insert run object values.
// When pulling objects from the object store Gofer will attempt to stringify the object (utf-8).
// If you need the raw bytes for an object use the Gofer cli.
func RunObject(key string) string {
	return fmt.Sprintf("run_object{{%s}}", key)
}

// Identifiers are used as the primary key in most of gofer's resources.
// They're defined by the user and therefore should have some sane bounds.
// For all ids we'll want the following:
// * 32 > characters < 3
// * Only alphanumeric characters or hyphens
//
// We don't allow underscores to conform with common practices for url safe strings.
func validateIdentifier(arg, value string) error {
	alphanumericWithHyphens := regexp.MustCompile("^[a-zA-Z0-9-]*$")

	if len(value) > 32 {
		return fmt.Errorf("length of arg %q cannot be greater than 32", arg)
	}

	if len(value) < 3 {
		return fmt.Errorf("length of arg %q cannot be less than 3", arg)
	}

	if !alphanumericWithHyphens.MatchString(value) {
		return fmt.Errorf("config %q can only be made up of alphanumeric and hyphen characters; found %q", arg, value)
	}
	return nil
}

// isDAG validates whether given task list inside a pipeline config represents an acyclic graph.
func (p *PipelineWrapper) isDAG() error {
	taskDAG := dag.New()

	// Add all nodes to the DAG first
	for _, task := range p.Pipeline.Tasks {
		err := taskDAG.AddNode(task.ID)
		if err != nil {
			if errors.Is(err, dag.ErrEntityExists) {
				return fmt.Errorf("duplicate task names found; %q is already a task", task.ID)
			}
			return err
		}
	}

	// Add all edges
	for _, task := range p.Pipeline.Tasks {
		for id := range task.Task.DependsOn {
			err := taskDAG.AddEdge(id, task.ID)
			if err != nil {
				if errors.Is(err, dag.ErrEdgeCreatesCycle) {
					return fmt.Errorf("a cycle was detected creating a dependency from task %q to task %q; %w", task.ID, id, dag.ErrEdgeCreatesCycle)
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
