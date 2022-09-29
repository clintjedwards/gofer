package sdk

import (
	"bytes"
	"encoding/binary"
	"errors"
	"testing"

	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/clintjedwards/gofer/sdk/go/internal/dag"
	"github.com/google/go-cmp/cmp"
	pb "google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/testing/protocmp"
)

func ExampleNewPipeline_simple() {
	err := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		WithDescription("Simple Test Pipeline").
		WithTasks(
			NewCustomTask("simple_task", "ubuntu:latest").
				WithDescription("This task simply prints our hello-world message and exits!").
				WithCommand("echo", `Hello from Gofer!`),
		).
		Finish()
	if err != nil {
		panic(err)
	}
}

func ExampleNewPipeline_dag() {
	taskOne := NewCustomTask("task_one", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
		WithDescription("This task has no dependencies so it will run immediately").
		WithVariable("WAIT_DURATION", "20s")

	dependsOnOne := NewCustomTask("depends_on_one", "ghcr.io/clintjedwards/gofer/debug/log:latest").
		WithDescription("This task depends on the first task to finish with a successfull result."+
			"This means that if the first task fails this task will not run.").
		WithVariable("LOGS_HEADER", "This string can be anything you want it to be").
		WithDependsOnOne(taskOne.ID, RequiredParentStatusSuccess)

	dependsOnTwo := NewCustomTask("depends_on_two", "docker.io/library/hello-world").
		WithDescription("This task depends on the second task, but will run after its finished regardless of the result.").
		WithDependsOnOne(dependsOnOne.ID, RequiredParentStatusAny)

	err := NewPipeline("dag_test_pipeline", "DAG Test Pipeline").
		WithDescription(`This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain
together containers that depend on other container's end states. This is obviously very useful if you want to
perform certain trees of actions depending on what happens in earlier containers.`).
		WithParallelism(10).
		WithTasks(taskOne, dependsOnOne, dependsOnTwo).
		Finish()
	if err != nil {
		panic(err)
	}
}

func TestInvalidPipelineCyclical(t *testing.T) {
	taskA := NewCustomTask("task_a", "").WithDependsOnOne("task_b", RequiredParentStatusAny)
	taskB := NewCustomTask("task_b", "").WithDependsOnOne("task_c", RequiredParentStatusAny)
	taskC := NewCustomTask("task_c", "").WithDependsOnOne("task_a", RequiredParentStatusAny)

	err := NewPipeline("invalid_pipeline", "").WithTasks(taskA, taskB, taskC).Finish()

	if !errors.Is(err, dag.ErrEdgeCreatesCycle) {
		t.Fatalf("expected cyclic graph error; found %v", err)
	}
}

// Tests that a pipeline can be fully serialized into proto and back and retain correct structure.
// Mostly a test on the many ToProto calls in the library.
func TestSimpleConfigSerialization(t *testing.T) {
	pipeline := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		WithDescription("Simple Test Pipeline").
		WithTasks(
			NewCustomTask("simple_task", "ubuntu:latest").
				WithDescription("This task simply prints our hello-world message and exits!").
				WithCommand("echo", `Hello from Gofer!`),
		)

	pipelineProto := pipeline.ToProto()

	output, err := pb.Marshal(pipelineProto)
	if err != nil {
		t.Fatal(err)
	}

	buf := bytes.NewBuffer([]byte{})
	err = binary.Write(buf, binary.LittleEndian, output)
	if err != nil {
		t.Fatal(err)
	}

	got := proto.PipelineConfig{}
	err = pb.Unmarshal(buf.Bytes(), &got)
	if err != nil {
		t.Fatal(err)
	}

	want := proto.PipelineConfig{
		Id:          "simple_test_pipeline",
		Name:        "Simple Test Pipeline",
		Description: "Simple Test Pipeline",
		Tasks: []*proto.PipelineTaskConfig{
			{
				Task: &proto.PipelineTaskConfig_CustomTask{
					CustomTask: &proto.CustomTaskConfig{
						Id:          "simple_task",
						Image:       "ubuntu:latest",
						Description: "This task simply prints our hello-world message and exits!",
						Command:     []string{"echo", `Hello from Gofer!`},
					},
				},
			},
		},
	}

	if diff := cmp.Diff(&want, &got, protocmp.Transform()); diff != "" {
		t.Errorf("proto did not match (-want +got):\n%s", diff)
	}
}

// Tests that compliation fails if user attempts to request a global var.
func TestInvalidConfigGlobalSecrets(t *testing.T) {
	err := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		WithDescription("Simple Test Pipeline").
		WithTasks(
			NewCustomTask("simple_task", "ubuntu:latest").
				WithDescription("This task simply prints our hello-world message and exits!").
				WithCommand("echo", `Hello from Gofer!`).
				WithVariable("test_var", GlobalSecret("some_secret_here")),
		).Finish()
	if err == nil {
		t.Fatal("pipeline should return an error due to user attempting to use global secrets, but it does not")
	}
}
