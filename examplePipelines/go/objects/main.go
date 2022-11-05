package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("objects", "Objects Pipeline").
		Description(`This pipeline displays how one might use Gofer's object/kv store to pass container results to other containers.`).
		Tasks(
			sdk.NewCustomTask("simple_task", "ghcr.io/clintjedwards/experimental:log").
				Description("This task has no dependencies so it will run immediately").
				Variables(map[string]string{
					"SOME_VARIABLE":         "something here",
					"LOGS_HEADER":           sdk.PipelineObject("logs_header"),
					"ALTERNATE_LOGS_HEADER": "pipeline_object{{alternate_logs_header}}",
				})).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
