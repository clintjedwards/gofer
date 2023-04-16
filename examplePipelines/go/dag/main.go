package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("dag", "Dag Test Pipeline").
		Description(
			"This pipeline shows off how you might use Gofer's DAG(Directed Acyclic Graph) system to chain "+
				"together containers that depend on other container's end states. This is obviously very useful if you want to perform "+
				"certain trees of actions depending on what happens in earlier containers.").
		Tasks(
			sdk.NewTask("first_task", "ghcr.io/clintjedwards/gofer/debug/wait:latest").
				Description("This task has no dependencies so it will run immediately").
				Variable("WAIT_DURATION", "20s"),

			sdk.NewTask("depends_on_first", "ghcr.io/clintjedwards/gofer/debug/log:latest").
				Description("This task depends on the first task to finish with a successful result. This means "+
					"that if the first task fails this task will not run").
				DependsOn("first_task", sdk.RequiredParentStatusSuccess).
				Variable("LOGS_HEADER", "This string is a stand in for something you might pass to your task"),

			sdk.NewTask("depends_on_second", "docker.io/library/hello-world").
				Description(`This task depends on the second task, but will run after it's finished regardless of the result`).
				DependsOn("depends_on_first", sdk.RequiredParentStatusAny),
		).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
