package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go"
)

func main() {
	err := sdk.NewPipeline("trigger", "Trigger Pipeline").
		WithDescription("This pipeline shows off the various features of a simple Gofer pipeline. Triggers, Tasks, and " +
			"dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.").
		WithTriggers(
			sdk.PipelineTriggerConfig{
				Name:  "interval",
				Label: "every_one_minute",
				Settings: map[string]string{
					"every": "1m",
				},
			},
		).WithTasks(
		sdk.NewCustomTask("simple_task", "ubuntu:latest").
			WithDescription("This task simply prints our hello-world message and exists!").
			WithCommand("echo", "Hello from Gofer!"),
	).Finish()
	if err != nil {
		log.Fatal(err)
	}
}