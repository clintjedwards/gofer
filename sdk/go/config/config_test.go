package config

import (
	"encoding/json"
	"errors"
	"testing"

	"github.com/clintjedwards/gofer/sdk/go/internal/dag"
	"github.com/google/go-cmp/cmp"
)

func ExampleNewPipeline_simple() {
	err := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		Description("Simple Test Pipeline").
		Tasks(
			NewTask("simple_task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", `Hello from Gofer!`),
		).
		Finish()
	if err != nil {
		panic(err)
	}
}

func ExampleNewPipeline_dag() {
	taskOne := NewTask("task_one", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
		Description("This task has no dependencies so it will run immediately").
		Variables(map[string]string{"WAIT_DURATION": "20s"})

	dependsOnOne := NewTask("depends_on_one", "ghcr.io/clintjedwards/gofer/debug/log:latest").
		Description("This task depends on the first task to finish  a successfull result."+
			"This means that if the first task fails this task will not run.").
		Variables(map[string]string{"LOGS_HEADER": "This string can be anything you want it to be"}).
		DependsOn(taskOne.ID, RequiredParentStatusSuccess)

	dependsOnTwo := NewTask("depends_on_two", "docker.io/library/hello-world").
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
	taskA := NewTask("task_a", "").DependsOn("task_b", RequiredParentStatusAny)
	taskB := NewTask("task_b", "").DependsOn("task_c", RequiredParentStatusAny)
	taskC := NewTask("task_c", "").DependsOn("task_a", RequiredParentStatusAny)

	err := NewPipeline("invalid_pipeline", "").Tasks(taskA, taskB, taskC).Finish()

	if !errors.Is(err, dag.ErrEdgeCreatesCycle) {
		t.Fatalf("expected cyclic graph error; found %v", err)
	}
}

// Tests that a pipeline can be fully serialized into json and back and retain correct structure.
// Mostly a test on the many json conversion calls in the library.
func TestSimpleConfigSerialization(t *testing.T) {
	pipeline := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		Description("Simple Test Pipeline").
		Tasks(
			NewTask("simple_task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", `Hello from Gofer!`),
		)

	pipelineJSON, err := json.Marshal(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	got := UserPipelineConfig{}
	err = json.Unmarshal(pipelineJSON, &got)
	if err != nil {
		t.Fatal(err)
	}

	want := UserPipelineConfig{
		ID:          "simple_test_pipeline",
		Name:        "Simple Test Pipeline",
		Description: "Simple Test Pipeline",
		Tasks: []*UserPipelineTaskConfig{
			{
				ID:          "simple_task",
				Image:       "ubuntu:latest",
				Description: "This task simply prints our hello-world message and exits!",
				Command:     []string{"echo", `Hello from Gofer!`},
				DependsOn:   map[string]RequiredParentStatus{},
				Variables:   map[string]string{},
			},
		},
	}

	if diff := cmp.Diff(&want, &got); diff != "" {
		t.Errorf("json did not match (-want +got):\n%s", diff)
	}
}

func TestInjectAPITokens(t *testing.T) {
	pipeline := NewPipeline("inject_test_pipeline", "").Tasks(NewTask("task_1", "").InjectAPIToken(true))
	if pipeline.Pipeline.Tasks[0].Task.InjectAPIToken == false {
		t.Fatal("pipeline is not in correct state")
	}
}
