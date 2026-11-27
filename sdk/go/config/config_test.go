package config

import (
	"encoding/json"
	"errors"
	"strings"
	"testing"

	"github.com/clintjedwards/gofer/sdk/go/internal/dag"
	"github.com/google/go-cmp/cmp"
)

func ExampleNewPipeline_simple() {
	err := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").
		Description("Simple Test Pipeline").
		Tasks(
			NewTask("simple-task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", `Hello from Gofer!`),
		).
		Finish()
	if err != nil {
		panic(err)
	}
}

func ExampleNewPipeline_dag() {
	taskOne := NewTask("task-one", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
		Description("This task has no dependencies so it will run immediately").
		Variables(map[string]string{"WAIT_DURATION": "20s"})

	dependsOnOne := NewTask("depends-on-one", "ghcr.io/clintjedwards/gofer/debug/log:latest").
		Description("This task depends on the first task to finish  a successfull result."+
			"This means that if the first task fails this task will not run.").
		Variables(map[string]string{"LOGS_HEADER": "This string can be anything you want it to be"}).
		DependsOn(taskOne.ID, RequiredParentStatusSuccess)

	dependsOnTwo := NewTask("depends-on-two", "docker.io/library/hello-world").
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
	taskA := NewTask("task-a", "").DependsOn("task-b", RequiredParentStatusAny)
	taskB := NewTask("task-b", "").DependsOn("task-c", RequiredParentStatusAny)
	taskC := NewTask("task-c", "").DependsOn("task-a", RequiredParentStatusAny)

	err := NewPipeline("invalid-pipeline", "").Tasks(taskA, taskB, taskC).Finish()

	if !errors.Is(err, dag.ErrEdgeCreatesCycle) {
		t.Fatalf("expected cyclic graph error; found %v", err)
	}
}

// The Rust SDK writes the same JSON, and the server reads both, so the shape can't change by accident. This is the
// same JSON the Rust SDK's test_builds_serialize_like_the_go_sdk expects.
func TestBuildsSerializeLikeTheRustSDK(t *testing.T) {
	cases := []struct {
		task *TaskWrapper
		want string
	}{
		{
			task: NewTask("report", "rust:1").
				Copy("./report", "/src").
				Run("cargo build --release").
				Keep([]string{"/src/target/release/report"}, "debian:13-slim"),
			want: `{"steps": {
				"base": "rust:1",
				"steps": [
					{"copy": {"from": "./report", "to": "/src"}},
					{"run": {"command": "cargo build --release"}}
				],
				"keep": {"paths": ["/src/target/release/report"], "base": "debian:13-slim"}
			}}`,
		},
		{
			task: NewDockerfileTask("scan", "scan/Dockerfile", "scan"),
			want: `{"dockerfile": {"path": "scan/Dockerfile", "context": "scan"}}`,
		},
	}

	for _, c := range cases {
		gotJSON, err := json.Marshal(c.task)
		if err != nil {
			t.Fatal(err)
		}

		var got, want map[string]any
		if err := json.Unmarshal(gotJSON, &got); err != nil {
			t.Fatal(err)
		}
		if err := json.Unmarshal([]byte(c.want), &want); err != nil {
			t.Fatal(err)
		}

		if diff := cmp.Diff(want, got["build"]); diff != "" {
			t.Errorf("build json did not match (-want +got):\n%s", diff)
		}

		// A config straight from the SDK has no image yet.
		if got["image"] != nil {
			t.Errorf("expected no image, got %v", got["image"])
		}
	}
}

func TestStepsCantBeUsedOnDockerfileTasks(t *testing.T) {
	task := NewDockerfileTask("scan", "Dockerfile", ".").Copy("./app", "/app")

	err := NewPipeline("pipeline", "").Tasks(task).Validate()
	if err == nil || !strings.Contains(err.Error(), "built from a Dockerfile") {
		t.Fatalf("expected an error about Dockerfile tasks; found %v", err)
	}
}

func TestKeptPathsHaveToBeFullPaths(t *testing.T) {
	task := NewTask("report", "rust:1").Keep([]string{"target/release/report"}, "debian:13-slim")

	err := NewPipeline("pipeline", "").Tasks(task).Validate()
	if err == nil || !strings.Contains(err.Error(), "full path") {
		t.Fatalf("expected an error about full paths; found %v", err)
	}
}

// List options with no arguments leave a nil slice, which would serialize as null and fail to parse in Gofer.
func TestEmptyListsAreOmitted(t *testing.T) {
	pipeline := NewPipeline("simple_test_pipeline", "Simple Test Pipeline").AllowRunsFrom().SkipDates()

	pipelineJSON, err := json.Marshal(pipeline)
	if err != nil {
		t.Fatal(err)
	}

	got := map[string]any{}
	if err := json.Unmarshal(pipelineJSON, &got); err != nil {
		t.Fatal(err)
	}

	for _, key := range []string{"allow_runs_from", "skip_dates"} {
		if _, ok := got[key]; ok {
			t.Errorf("expected %s to be left out, got %s", key, pipelineJSON)
		}
	}
}

func TestInvalidAllowRunsFrom(t *testing.T) {
	for _, pattern := range []string{"no-slash", "/build", "team-a/", "team-a/("} {
		err := NewPipeline("simple-test-pipeline", "Simple Test Pipeline").AllowRunsFrom(pattern).Validate()
		if err == nil {
			t.Errorf("expected '%s' to be rejected", pattern)
		}
	}

	err := NewPipeline("simple-test-pipeline", "Simple Test Pipeline").AllowRunsFrom("team-a-.*/.*").Validate()
	if err != nil {
		t.Errorf("expected valid pattern to pass; %v", err)
	}
}

func TestInvalidSkipDates(t *testing.T) {
	for _, skipDate := range []string{
		"", "12", "12/", "1/1", "13/01", "00/10", "12/00", "12/32", "02/30", "04/31", "+1/01", "2026/12/25", "12/25-",
		"12/25-13/01", "12/24-12/25-12/26", "12/25:", "12/25:thursday", "12/25:thu:fri", "12/25 ",
	} {
		err := NewPipeline("simple-test-pipeline", "Simple Test Pipeline").SkipDates(skipDate).Validate()
		if err == nil {
			t.Errorf("expected '%s' to be rejected", skipDate)
		}
	}

	err := NewPipeline("simple-test-pipeline", "Simple Test Pipeline").
		SkipDates("12/25", "02/29", "12/24-01/02", "11/22-11/28:thu").Validate()
	if err != nil {
		t.Errorf("expected valid dates to pass; %v", err)
	}
}

func TestInjectAPITokens(t *testing.T) {
	pipeline := NewPipeline("inject-test-pipeline", "").Tasks(NewTask("task-1", "").InjectAPIToken(true))
	if pipeline.Pipeline.Tasks[0].Task.InjectAPIToken == false {
		t.Fatal("pipeline is not in correct state")
	}
}
