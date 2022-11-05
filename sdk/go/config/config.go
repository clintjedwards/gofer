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

func (ra *RegistryAuth) Proto() *proto.RegistryAuth {
	if ra == nil {
		return nil
	}

	return &proto.RegistryAuth{
		User: ra.User,
		Pass: ra.Pass,
	}
}

type TaskConfig interface {
	isTaskConfig()
	getKind() TaskKind
	getID() string
	getDependsOn() map[string]RequiredParentStatus
	validate() error
}

type TaskKind string

const (
	TaskKindUnknown TaskKind = "UNKNOWN"
	TaskKindCommon  TaskKind = "COMMON"
	TaskKindCustom  TaskKind = "CUSTOM"
)

// PipelineWrapper type simply exists so that we can make structs with fields like "id"
// and we can still add functions called "id()". This makes it not only easier to
// reason about when working with the struct, but when just writing pipelines as an end user.
type PipelineWrapper struct {
	Pipeline
}

type Pipeline struct {
	ID          string           `json:"id"`
	Name        string           `json:"name"`
	Description string           `json:"description"`
	Parallelism int64            `json:"parallelism"`
	Tasks       []TaskConfig     `json:"tasks"`
	Triggers    []TriggerWrapper `json:"triggers"`
}

func NewPipeline(id, name string) *PipelineWrapper {
	return &PipelineWrapper{
		Pipeline{
			ID:          id,
			Name:        name,
			Description: "",
			Parallelism: 0,
			Tasks:       []TaskConfig{},
			Triggers:    []TriggerWrapper{},
		},
	}
}

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

	for _, trigger := range p.Pipeline.Triggers {
		err = trigger.validate()
		if err != nil {
			return err
		}
	}

	return nil
}

func (p *PipelineWrapper) Description(description string) *PipelineWrapper {
	p.Pipeline.Description = description
	return p
}

func (p *PipelineWrapper) Parallelism(parallelism int64) *PipelineWrapper {
	p.Pipeline.Parallelism = parallelism
	return p
}

func (p *PipelineWrapper) Tasks(tasks ...TaskConfig) *PipelineWrapper {
	p.Pipeline.Tasks = tasks
	return p
}

func (p *PipelineWrapper) Triggers(triggers ...TriggerWrapper) *PipelineWrapper {
	p.Pipeline.Triggers = triggers
	return p
}

func (p *PipelineWrapper) Proto() *proto.PipelineConfig {
	tasks := []*proto.PipelineTaskConfig{}
	for _, task := range p.Pipeline.Tasks {
		switch t := task.(type) {
		case *CommonTaskWrapper:
			tasks = append(tasks, &proto.PipelineTaskConfig{
				Task: &proto.PipelineTaskConfig_CommonTask{
					CommonTask: t.Proto(),
				},
			})
		case *CustomTaskWrapper:
			tasks = append(tasks, &proto.PipelineTaskConfig{
				Task: &proto.PipelineTaskConfig_CustomTask{
					CustomTask: t.Proto(),
				},
			})
		}
	}

	triggers := []*proto.PipelineTriggerConfig{}
	for _, trigger := range p.Pipeline.Triggers {
		triggers = append(triggers, trigger.Proto())
	}

	return &proto.PipelineConfig{
		Id:          p.Pipeline.ID,
		Name:        p.Pipeline.Name,
		Description: p.Pipeline.Description,
		Parallelism: p.Pipeline.Parallelism,
		Tasks:       tasks,
		Triggers:    triggers,
	}
}

// Call finish as the last method to the pipeline config
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

func PipelineSecret(key string) string {
	return fmt.Sprintf("pipeline_secret{{%s}}", key)
}

func GlobalSecret(key string) string {
	return fmt.Sprintf("global_secret{{%s}}", key)
}

func PipelineObject(key string) string {
	return fmt.Sprintf("pipeline_object{{%s}}", key)
}

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
