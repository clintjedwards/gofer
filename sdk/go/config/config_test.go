package config

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
		Description("Simple Test Pipeline").
		Tasks(
			NewCustomTask("simple_task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", `Hello from Gofer!`),
		).
		Finish()
	if err != nil {
		panic(err)
	}
}

func ExampleNewPipeline_dag() {
	taskOne := NewCustomTask("task_one", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
		Description("This task has no dependencies so it will run immediately").
		Variable("WAIT_DURATION", "20s")

	dependsOnOne := NewCustomTask("depends_on_one", "ghcr.io/clintjedwards/gofer/debug/log:latest").
		Description("This task depends on the first task to finish  a successfull result."+
			"This means that if the first task fails this task will not run.").
		Variable("LOGS_HEADER", "This string can be anything you want it to be").
		DependsOn(taskOne.ID, RequiredParentStatusSuccess)

	dependsOnTwo := NewCustomTask("depends_on_two", "docker.io/library/hello-world").
		Description("This task depends on the second task, but will run after its finished regardless of the result.").
		DependsOn(dependsOnOne.ID, RequiredParentStatusAny)

	err := NewPipeline("dag_test_pipeline", "DAG Test Pipeline").
		Description(`This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain
together containers that depend on other container's end states. This is obviously very useful if you want to
perform certain trees of actions depending on what happens in earlier containers.`).
		Parallelism(10).
		Tasks(taskOne, dependsOnOne, dependsOnTwo).
		Finish()
	if err != nil {
		panic(err)
	}
}

func TestInvalidPipelineCyclical(t *testing.T) {
	taskA := NewCustomTask("task_a", "").DependsOn("task_b", RequiredParentStatusAny)
	taskB := NewCustomTask("task_b", "").DependsOn("task_c", RequiredParentStatusAny)
	taskC := NewCustomTask("task_c", "").DependsOn("task_a", RequiredParentStatusAny)

	err := NewPipeline("invalid_pipeline", "").Tasks(taskA, taskB, taskC).Finish()

	if !errors.Is(err, dag.ErrEdgeCreatesCycle) {
		t.Fatalf("expected cyclic graph error; found %v", err)
	}
}

// Tests that a pipeline can be fully serialized into proto and back and retain correct structure.
// Mostly a test on the many ToProto calls in the library.
func TestSimpleConfigSerialization(t *testing.T) {
	pipeline := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		Description("Simple Test Pipeline").
		Tasks(
			NewCustomTask("simple_task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", `Hello from Gofer!`),
		)

	pipelineProto := pipeline.Proto()

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
		Description("Simple Test Pipeline").
		Tasks(
			NewCustomTask("simple_task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", `Hello from Gofer!`).
				Variable("test_var", "global_secret{{some_secret_here}}"),
		).Finish()
	if err == nil {
		t.Fatal("pipeline should return an error due to user attempting to use global secrets, but it does not")
	}
}

func TestInjectAPITokens(t *testing.T) {
	pipeline := NewPipeline("inject_test_pipeline", "").Tasks(NewCustomTask("task_1", "").InjectAPIToken(true))
	if pipeline.Pipeline.Tasks[0].(*CustomTaskWrapper).CustomTask.InjectAPIToken == false {
		t.Fatal("pipeline is not in correct state")
	}
}
