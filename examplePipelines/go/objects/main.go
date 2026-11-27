package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	pipeline := sdk.NewPipeline("objects", "Objects Pipeline").
		Description(`This pipeline shows how a task can read values from Gofer's object store, which keeps them across runs.`)

	pipeline.Task(sdk.NewTask("simple-task", "ghcr.io/clintjedwards/gofer/debug/log:latest").
		Description("This task has no dependencies so it will run immediately").
		Variables(map[string]string{
			"SOME_VARIABLE":         "something here",
			"LOGS_HEADER":           sdk.PipelineObject("logs_header"),
			"ALTERNATE_LOGS_HEADER": "pipeline_object{{alternate_logs_header}}",
		}))

	if err := pipeline.Finish(); err != nil {
		log.Fatal(err)
	}
}
