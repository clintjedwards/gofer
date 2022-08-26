package sdk

import (
	"errors"
	"testing"

	"github.com/clintjedwards/gofer/sdk/go/dag"
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

	// Output:
	// {"id":"simple_test_pipeline","name":"Simple Test Pipeline","description":"Simple Test Pipeline","parallelism":0,"tasks":[{"kind":"CUSTOM","id":"simple_task","description":"This task simply prints our hello-world message and exits!","image":"ubuntu:latest","registry_auth":null,"depends_on":{},"variables":{},"entrypoint":null,"command":["echo","Hello from Gofer!"]}],"triggers":[]}
}

func ExampleNewPipeline_dag() {
	taskOne := NewCustomTask("task_one", "ghcr.io/clintjedwards/gofer-containers/debug/wait:latest").
		WithDescription("This task has no dependencies so it will run immediately").
		WithVariable("WAIT_DURATION", "20s")

	dependsOnOne := NewCustomTask("depends_on_one", "ghcr.io/clintjedwards/gofer-containers/debug/log:latest").
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

	// Output:
	// {"id":"dag_test_pipeline","name":"DAG Test Pipeline","description":"This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain\ntogether containers that depend on other container's end states. This is obviously very useful if you want to\nperform certain trees of actions depending on what happens in earlier containers.","parallelism":10,"tasks":[{"kind":"CUSTOM","id":"task_one","description":"This task has no dependencies so it will run immediately","image":"ghcr.io/clintjedwards/gofer-containers/debug/wait:latest","registry_auth":null,"depends_on":{},"variables":{"WAIT_DURATION":"20s"},"entrypoint":null,"command":null},{"kind":"CUSTOM","id":"depends_on_one","description":"This task depends on the first task to finish with a successfull result.This means that if the first task fails this task will not run.","image":"ghcr.io/clintjedwards/gofer-containers/debug/log:latest","registry_auth":null,"depends_on":{"task_one":"SUCCESS"},"variables":{"LOGS_HEADER":"This string can be anything you want it to be"},"entrypoint":null,"command":null},{"kind":"CUSTOM","id":"depends_on_two","description":"This task depends on the second task, but will run after its finished regardless of the result.","image":"docker.io/library/hello-world","registry_auth":null,"depends_on":{"depends_on_one":"ANY"},"variables":{},"entrypoint":null,"command":null}],"triggers":[]}
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
