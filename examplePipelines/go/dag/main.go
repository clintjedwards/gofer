package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go"
)

func main() {
	err := sdk.NewPipeline("dag", "Dag Test Pipeline").
		WithDescription(
			"This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain "+
				"together containers that depend on other container's end states. This is obviously very useful if you want to perform "+
				"certain trees of actions depending on what happens in earlier containers.").
		WithTasks(
			sdk.NewCustomTask("first_task", "ghcr.io/clintjedwards/experimental:wait").
				WithDescription("This task has no dependencies so it will run immediately").
				WithVariable("WAIT_DURATION", "20s"),

			sdk.NewCustomTask("depends_on_first", "ghcr.io/clintjedwards/experimental:log").
				WithDescription("This task depends on the first task to finish with a successful result. This means "+
					"that if the first task fails this task will not run").
				WithDependsOnOne("first_task", sdk.RequiredParentStatusSuccess).
				WithVariable("LOGS_HEADER", "This string can be anything you want it to be"),

			sdk.NewCustomTask("depends_on_second", "docker.io/library/hello-world").
				WithDescription(`This task depends on the second task, but will run after it's finished regardless of the result`).
				WithDependsOnOne("depends_on_first", sdk.RequiredParentStatusAny),
		).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
