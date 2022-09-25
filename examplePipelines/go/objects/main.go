package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go"
)

func main() {
	err := sdk.NewPipeline("objects", "Objects Pipeline").
		WithDescription(`This pipeline displays how one might use Gofer's object/kv store to pass container results to other containers.`).
		WithTasks(
			sdk.NewCustomTask("simple_task", "ghcr.io/clintjedwards/experimental:log").
				WithDescription("This task has no dependencies so it will run immediately").
				WithVariables(map[string]string{
					"SOME_VARIABLE": "something here",
					"LOGS_HEADER":   "pipeline_object{{ logs_header }}",
				})).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
