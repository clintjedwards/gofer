package config

import (
	"encoding/binary"
	"errors"
	"fmt"
	"os"
	"regexp"
	"strings"

	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/clintjedwards/gofer/sdk/go/internal/dag"
	pb "google.golang.org/protobuf/proto"
)

// RequiredParentStatus is used to describe the state you would like the parent dependency to be in
// before the child is run.
type RequiredParentStatus string

const (
	// Any means that no matter what the state of the parent is at end, the child will run.
	RequiredParentStatusAny RequiredParentStatus = "ANY"

	// Success requires that the parent pass with a SUCCESSFUL status before the child will run.
	RequiredParentStatusSuccess RequiredParentStatus = "SUCCESS"

	// Failure requires the parent fail in anyway before the child is run.
	RequiredParentStatusFailure RequiredParentStatus = "FAILURE"
)

// RegistryAuth represents docker repository authentication.
type RegistryAuth struct {
	User string `json:"user"`
	Pass string `json:"pass"`
}

func (ra *RegistryAuth) FromProto(proto *proto.RegistryAuth) {
	ra.User = proto.User
	ra.Pass = proto.Pass
}

// Returns the protobuf representation.
func (ra *RegistryAuth) Proto() *proto.RegistryAuth {
	if ra == nil {
		return nil
	}

	return &proto.RegistryAuth{
		User: ra.User,
		Pass: ra.Pass,
	}
}

// TaskConfig represents the interface for different types of tasks Gofer accepts.
type TaskConfig interface {
	isTaskConfig()
	getKind() TaskKind
	getID() string
	getDependsOn() map[string]RequiredParentStatus
	validate() error
}

// TaskKind represents an enum of types of tasks Gofer accepts.
type TaskKind string

const (
	TaskKindUnknown TaskKind = "UNKNOWN"

	// TaskKindCommon represents a common task. Common Tasks are set up by Gofer administrators and then can be included
	// in Gofer pipelines.
	TaskKindCommon TaskKind = "COMMON"

	// TaskKindCustom represents a custom task. Custom Tasks are set up on a per pipeline basis by the pipeline user.
	TaskKindCustom TaskKind = "CUSTOM"
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
	ID          string       `json:"id"`          // Unique Identifier for the pipeline.
	Name        string       `json:"name"`        // Humanized name for the pipeline.
	Description string       `json:"description"` // A short description for the pipeline.
	Parallelism int64        `json:"parallelism"` // How many runs are allowed to run at the same time.
	Tasks       []TaskConfig `json:"tasks"`       // The task set of the pipeline. AKA which containers should be run.
}

// Create a new pipeline.
//   - The ID must be between 3 and 32 characters long and only alphanumeric, underscores are the only allowed
//     alphanumeric character.
//     Ex. `simple_pipeline`
//   - The name is a human friendly name to represent the pipeline.
//     Ex. `Simple Pipeline`
func NewPipeline(id, name string) *PipelineWrapper {
	return &PipelineWrapper{
		Pipeline{
			ID:          id,
			Name:        name,
			Description: "",
			Parallelism: 0,
			Tasks:       []TaskConfig{},
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

// Tasks are containers that the pipeline runs. There are two types of tasks.
//   - Common Tasks: Are set up by the Gofer administrator and allow you add pre-configured tasks to your  pipeline.
//   - Custom Tasks: Are containers that you define for Gofer to execute.
func (p *PipelineWrapper) Tasks(tasks ...TaskConfig) *PipelineWrapper {
	p.Pipeline.Tasks = tasks
	return p
}

func (p *PipelineWrapper) Proto() *proto.UserPipelineConfig {
	tasks := []*proto.UserPipelineTaskConfig{}
	for _, task := range p.Pipeline.Tasks {
		switch t := task.(type) {
		case *CommonTaskWrapper:
			tasks = append(tasks, &proto.UserPipelineTaskConfig{
				Task: &proto.UserPipelineTaskConfig_CommonTask{
					CommonTask: t.Proto(),
				},
			})
		case *CustomTaskWrapper:
			tasks = append(tasks, &proto.UserPipelineTaskConfig{
				Task: &proto.UserPipelineTaskConfig_CustomTask{
					CustomTask: t.Proto(),
				},
			})
		}
	}

	return &proto.UserPipelineConfig{
		Id:          p.Pipeline.ID,
		Name:        p.Pipeline.Name,
		Description: p.Pipeline.Description,
		Parallelism: p.Pipeline.Parallelism,
		Tasks:       tasks,
	}
}

// Call finish as the last method to the pipeline config. Finish validates and converts the pipeline
// to Protobuf so that it can be read in by other programs.
func (p *PipelineWrapper) Finish() error {
	err := p.Validate()
	if err != nil {
		return err
	}

	pipelineProto := p.Proto()

	output, err := pb.Marshal(pipelineProto)
	if err != nil {
		return err
	}

	err = binary.Write(os.Stdout, binary.LittleEndian, output)
	if err != nil {
		return err
	}

	return nil
}

// Convenience function to insert pipeline secret values.
func PipelineSecret(key string) string {
	return fmt.Sprintf("pipeline_secret{{%s}}", key)
}

// Convenience function to insert pipeline object values.
func PipelineObject(key string) string {
	return fmt.Sprintf("pipeline_object{{%s}}", key)
}

// Convenience function to insert run object values.
func RunObject(key string) string {
	return fmt.Sprintf("run_object{{%s}}", key)
}

var alphanumericWithUnderscores = regexp.MustCompile("^[a-zA-Z0-9_]*$")

// Identifiers are used as the primary key in most of gofer's resources.
// They're defined by the user and therefore should have some sane bounds.
// For all ids we'll want the following:
//   - 32 > characters < 3
//   - Only alphanumeric characters or underscores
func validateIdentifier(arg, value string) error {
	if len(value) > 32 {
		return fmt.Errorf("length of arg %q cannot be greater than 32", arg)
	}

	if len(value) < 3 {
		return fmt.Errorf("length of arg %q cannot be less than 3", arg)
	}

	if !alphanumericWithUnderscores.MatchString(value) {
		return fmt.Errorf("config %q can only be made up of alphanumeric and underscore characters; found %q", arg, value)
	}
	return nil
}

// validateVariables checks to make sure all variables are in a parsable form and don't contain any requests for global secrets
// as this will fail the pipeline.
func validateVariables(variables map[string]string) error {
	// TODO(clintjedwards): We should check to make sure that "interpolatevars" function in the main program will work here.

	for _, variable := range variables {
		if strings.HasPrefix(variable, "global_secret") {
			return fmt.Errorf("invalid variable %q; cannot use global secrets in pipeline configs; global secrets are only allowed for system level configs set up by Gofer administrators", variable)
		}
	}

	return nil
}

// isDAG validates whether given task list inside a pipeline config represents an acyclic graph.
func (p *PipelineWrapper) isDAG() error {
	taskDAG := dag.New()

	// Add all nodes to the DAG first
	for _, task := range p.Pipeline.Tasks {
		err := taskDAG.AddNode(task.getID())
		if err != nil {
			if errors.Is(err, dag.ErrEntityExists) {
				return fmt.Errorf("duplicate task names found; %q is already a task", task.getID())
			}
			return err
		}
	}

	// Add all edges
	for _, task := range p.Pipeline.Tasks {
		for id := range task.getDependsOn() {
			err := taskDAG.AddEdge(id, task.getID())
			if err != nil {
				if errors.Is(err, dag.ErrEdgeCreatesCycle) {
					return fmt.Errorf("a cycle was detected creating a dependency from task %q to task %q; %w", task.getID(), id, dag.ErrEdgeCreatesCycle)
				}
				if errors.Is(err, dag.ErrEntityNotFound) {
					return fmt.Errorf("task %q is listed as a dependency within task %q but does not exist", id, task.getID())
				}
				return err
			}
		}
	}

	return nil
}
