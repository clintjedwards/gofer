package main

import (
	"fmt"
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("objects", "Objects Pipeline").
		Description(`This pipeline displays how one might use Gofer's object/kv store to pass container results to other containers.`).
		Tasks(
			sdk.NewTask("simple-task", "ghcr.io/clintjedwards/gofer/debug/log:latest").
				Description("This task has no dependencies so it will run immediately").
				Variables(map[string]string{
					"SOME_VARIABLE":         "something here",
					"LOGS_HEADER":           sdk.PipelineObject("logs_header"),
					"ALTERNATE_LOGS_HEADER": fmt.Sprintf("pipeline_object%s", sdk.PipelineObject("alternate_logs_header")),
				})).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
