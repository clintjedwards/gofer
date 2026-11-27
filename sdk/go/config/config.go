package config

import (
	"encoding/json"
	"errors"
	"fmt"
	"regexp"
	"slices"
	"strings"
	"time"

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

// PipelineWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type PipelineWrapper struct {
	Pipeline
}

// A pipeline is a representation of a Gofer pipeline, the structure in which users represent what they
// want to run in Gofer.
type Pipeline struct {
	ID            string         `json:"id" example:"my_pipeline_name" doc:"Unique Identifier for the pipeline"`                                                                                 // Unique Identifier for the pipeline.
	Name          string         `json:"name" example:"My Pipeline Name" doc:"Humanized name for the pipeline"`                                                                                  // Humanized name for the pipeline.
	Description   string         `json:"description" example:"This pipeline is used for x" doc:"A short description for the pipeline"`                                                           // A short description for the pipeline.
	Parallelism   int64          `json:"parallelism" example:"2" doc:"The total amount of pipelines run allowed at any given time"`                                                              // How many runs are allowed to run at the same time.
	AllowRunsFrom []string       `json:"allow_runs_from,omitempty" example:"[\"team-a-.*/.*\"]" doc:"Other pipelines whose runs may start runs of this one, as namespace/pipeline patterns"`     // Other pipelines whose runs may start runs of this one.
	SkipDates     []string       `json:"skip_dates,omitempty" example:"[\"12/25\", \"11/22-11/28:thu\"]" doc:"Days the pipeline does not run, as MM/DD or MM/DD-MM/DD with an optional weekday"` // Days the pipeline does not run.
	Tasks         []*TaskWrapper `json:"tasks" doc:"The task set of the pipeline. AKA which containers should run"`                                                                              // The task set of the pipeline. AKA which containers should be run.
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
//	        Task::new("simple-task", "ubuntu:latest")
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
			ID:            id,
			Name:          name,
			Description:   "",
			Parallelism:   0,
			AllowRunsFrom: []string{},
			SkipDates:     []string{},
			Tasks:         []*TaskWrapper{},
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

	for _, pattern := range p.Pipeline.AllowRunsFrom {
		err = validateRunSource(pattern)
		if err != nil {
			return err
		}
	}

	for _, skipDate := range p.Pipeline.SkipDates {
		err = validateSkipDate(skipDate)
		if err != nil {
			return err
		}
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

// AllowRunsFrom lets runs of other pipelines start runs of this one. Each entry is a "namespace/pipeline" pattern
// where both halves are regexes matched against the whole id. For example "team-a-.*/.*" allows any pipeline in a
// namespace starting with "team-a-", and "deploys/release" allows just that one pipeline.
//
// Anyone who can start runs of an allowed pipeline can use its injected token to start this one, so only list
// pipelines whose users you'd trust to run this pipeline.
func (p *PipelineWrapper) AllowRunsFrom(patterns ...string) *PipelineWrapper {
	p.Pipeline.AllowRunsFrom = patterns
	return p
}

// validateRunSource mirrors the Rust SDK's check; the Gofer API runs that one on registration either way.
func validateRunSource(pattern string) error {
	namespace, pipeline, found := strings.Cut(pattern, "/")
	if !found || namespace == "" || pipeline == "" {
		return fmt.Errorf("invalid allow_runs_from '%s'; must be 'namespace/pipeline' where both halves are "+
			"non-empty regexes; use '.*' to match everything", pattern)
	}

	for _, half := range []string{namespace, pipeline} {
		if _, err := regexp.Compile("^(?:" + half + ")$"); err != nil {
			return fmt.Errorf("invalid allow_runs_from '%s'; '%s' is not a valid regex: %w", pattern, half, err)
		}
	}

	return nil
}

// SkipDates lists days the pipeline shouldn't run, as "MM/DD" or "MM/DD-MM/DD". Ranges include both ends and can go over
// the new year. Add a weekday ("sun" to "sat") to only skip that day of the range, so "11/22-11/28:thu" is Thanksgiving.
//
// Dates repeat every year and are in UTC. A run started on one of them is recorded as skipped, unless it's forced.
func (p *PipelineWrapper) SkipDates(dates ...string) *PipelineWrapper {
	p.Pipeline.SkipDates = dates
	return p
}

// validateSkipDate mirrors the Rust SDK's check; the Gofer API runs that one on registration either way.
func validateSkipDate(skipDate string) error {
	dates, weekday, hasWeekday := strings.Cut(skipDate, ":")
	first, last, isRange := strings.Cut(dates, "-")
	if !isRange {
		last = first
	}

	// Without a year Go parses dates as if they're in year 0, which is a leap year, so 02/29 is allowed.
	_, firstErr := time.Parse("01/02", first)
	_, lastErr := time.Parse("01/02", last)

	weekdays := []string{"sun", "mon", "tue", "wed", "thu", "fri", "sat"}

	if firstErr != nil || lastErr != nil || (hasWeekday && !slices.Contains(weekdays, weekday)) {
		return fmt.Errorf("invalid skip_dates '%s'; must be 'MM/DD' or 'MM/DD-MM/DD', optionally followed by a weekday "+
			"like ':thu'", skipDate)
	}

	return nil
}

// Tasks are containers that the pipeline runs.
func (p *PipelineWrapper) Tasks(tasks ...*TaskWrapper) *PipelineWrapper {
	p.Pipeline.Tasks = tasks
	return p
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
