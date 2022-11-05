package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("trigger", "Trigger Pipeline").
		Description("This pipeline shows off the various features of a simple Gofer pipeline. Triggers, Tasks, and " +
			"dependency graphs are all tools that can be wielded to create as complicated pipelines as need be.").
		Triggers(
			*sdk.NewTrigger("interval", "every_one_minute").Setting("every", "1m"),
		).Tasks(
		sdk.NewCustomTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!"),
	).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
