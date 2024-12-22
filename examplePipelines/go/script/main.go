package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

const exampleScript string = `#!/bin/bash

echo "This is a simple example of a script"
echo "You quickly wrote and didn't want"
echo "to go through the trouble of wrapping it in a docker container"
uname -a
sleep 5m
`

func main() {
	err := sdk.NewPipeline("script", "Script Example Pipeline").
		Description(
			"This pipeline shows how to run a simple multiline script in Gofer. "+
				"Since Gofer is an extension of platforms that run containers the natural way to "+
				"run a script is to simply pass it in via the command field. \n\n"+
				"Below, we'll simply run an ubuntu container containing bash and then pass in our script "+
				"to the command field. Since Gofer uses a full programming language you can pass this "+
				"script in anyway that feels natural. You can keep it in a separate file or just keep it as "+
				"a string literal in this pipeline config.\n\n"+
				"Alternatively, if you only have a handful of commands you want to run, you can always just "+
				"enter them directly into the command field. Both examples are shown.").
		Tasks(
			sdk.NewTask("direct-input", "ubuntu:latest").
				Description("This task simply prints our hello-world message and exits!").
				Command("echo", "Hello from Gofer!"),
			sdk.NewTask("from-prepared-string", "ubuntu:latest").
				Description("This task executes a bash script passed in from somewhere else").
				Command("/bin/bash", "-c", exampleScript),
		).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
