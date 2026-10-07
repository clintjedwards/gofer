package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

// Prints a few test results and exits with an error so the pipeline has a failure to debug.
const fakeTestRun = `sleep 3
echo "running unit tests..."
echo "ok    auth       0.12s"
echo "ok    billing    0.31s"
echo "ok    storage    0.08s"
echo "FAIL  api        0.44s"
echo "    handlers_test.go:42: expected status 200, got 500"
exit 1`

func main() {
	err := sdk.NewPipeline("dag", "Dag Test Pipeline").
		Description(
			"This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain "+
				"together containers that depend on other container's end states. This is obviously very useful if you want to perform "+
				"certain trees of actions depending on what happens in earlier containers.").
		Tasks(
			sdk.NewTask("first-task", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
				Description("This task has no dependencies so it will run immediately").
				Variables(map[string]string{"WAIT_DURATION": "20s"}),

			sdk.NewTask("depends-on-first", "ghcr.io/clintjedwards/gofer/debug/log:latest").
				Description("This task depends on the first task to finish with a successful result. This means "+
					"that if the first task fails this task will not run").
				DependsOn("first-task", sdk.RequiredParentStatusSuccess).
				Variables(map[string]string{"LOGS_HEADER": "This string is a stand in for something you might pass to your task"}),

			sdk.NewTask("depends-on-second", "hello-world").
				Description(`This task depends on the second task, but will run after it's finished regardless of the result`).
				DependsOn("depends-on-first", sdk.RequiredParentStatusAny),

			sdk.NewTask("run-tests", "alpine:latest").
				Description("This task fails on purpose so you can see what a failure looks like in 'gofer run debug'").
				Command("sh", "-c", fakeTestRun),

			sdk.NewTask("publish-release", "alpine:latest").
				Description("This task only runs if the tests pass, so it gets skipped when run-tests fails").
				DependsOn("run-tests", sdk.RequiredParentStatusSuccess).
				Command("echo", "publishing release"),
		).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
