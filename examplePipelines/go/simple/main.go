package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("simple", "Simple Pipeline").
		Description("This pipeline shows off a very simple Gofer pipeline that simply pulls in " +
			"a container and runs a command. Veterans of CI/CD tooling should be familiar with this pattern.\n\n" +

			"Shown below, tasks are the building blocks of a pipeline. They represent individual containers " +
			"and can be configured to depend on one or multiple other tasks.\n\n" +

			"In the task here, we simply call the very familiar Ubuntu container and run some commands of our own.\n\n" +

			"While this is the simplest example of Gofer, the vision is to move away from writing our logic code " +
			"in long bash scripts within these task definitions.\n\n" +

			"Ideally, these tasks are custom containers built with the purpose of being run within Gofer for a " +
			"particular workflow. Allowing you to keep the logic code closer to the actual object that uses it " +
			"and keeping the Gofer pipeline configurations from becoming a mess.\n").
		Tasks(
			sdk.NewTask("simple-task", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", "Hello from Gofer!").Variable("test", "sample"),
		).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
