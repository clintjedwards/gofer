# Pipeline Configuration

A pipeline is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a run. Gofer allows users to configure their pipeline via a configuration file written in [Golang](https://go.dev/) or [Rust](https://www.rust-lang.org/).

The general hierarchy for a pipeline is:

```
pipeline
    \_ run
         \_ task
```

Each execution of a pipeline is a run and every run consists of one or more tasks (containers). These tasks are where users specify their containers and settings.

## SDK

Creating a pipeline involves using [Gofer's SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk) currently written in Go or Rust.

Extensive documentation can be found on the [SDK's reference page](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk/config). There you will find most of the features and idiosyncrasies available to you when creating a pipeline.

## Small Walkthrough

To introduce some of the concepts slowly, lets build a pipeline step by step. We'll be using Go as our pipeline configuration language and this documentation assumes you've already set up your project and are operating in a `main.go` file.

### A Simple Pipeline

Every pipeline is initialized with a simple pipeline declaration. It's here that we will name our pipeline, giving it a machine referable ID and a human referable name.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline")
```

It's important to note here that while your human readable name ("My Simple Pipeline" in this case) can contain a large amount of characters the ID can only container alphanumeric letters, numbers, and underscores. Any other characters will result in an error when attempting to register the pipeline.

### Add a Description

Next we'll add a simple description to remind us what this pipeline is used for.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
        Description("This pipeline is purely for testing purposes.")
```

The SDK uses a builder pattern, which allows us to simply add another function onto our Pipeline object which we can type our description into.

### Add a trigger

Next we'll add a trigger. Triggers allow us to automate when our pipeline's run. Triggers usually execute a pipeline for us based on some event. In this example that even is the passage of time.

To do this we'll use a trigger included with Gofer called the [interval](../triggers/provided/interval.md) trigger. This trigger simply counts time and executes pipeline's based on that pipeline's specific time configuration.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
        Description("This pipeline is purely for testing purposes.").
        Triggers(
            *sdk.NewTrigger("interval", "every_one_minute").WithSetting("every", "1m"),
    )
```

Here you can see we create a new `WithTriggers` block and then add a single trigger `interval`. We also add a setting block. Different triggers have different settings that pipelines can pass to them. In this case, passing the setting `every` along with the value `1m` will tell interval that this pipeline should be executed every minute.

When this pipeline is registered, Gofer will check that a trigger named `interval` actually exists and it will then communicate with that trigger to tell it which pipeline wants to register and which configuration values it has passed along.

If this registration with the trigger cannot be formed the registration of the overall pipeline will fail.

### Add a task

Lastly let's add a task(container) to our pipeline. We'll add a simple ubuntu container and change the command that gets
run on container start to just say "Hello from Gofer!".

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
        Description("This pipeline is purely for testing purposes.").
        Triggers(
        *sdk.NewTrigger("interval", "every_one_minute").Setting("every", "1m"),
    ).Tasks(
		sdk.NewCustomTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!"),
    )
```

We used the `Tasks` function to add multiple tasks and then we use the SDK's `NewCustomTask` function to create a task. You can see we:

- Give the task an ID, much like our pipeline earlier.
- Specify which image we want to use.
- Tack on a description.
- And then finally specify the command.

To tie a bow on it, we add the `.Finish()` function to specify that our pipeline is in it's final form.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
    Description("This pipeline is purely for testing purposes.").
    Triggers(
        *sdk.NewTrigger("interval", "every_one_minute").Setting("every", "1m"),
    ).Tasks(
		sdk.NewCustomTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!"),
    ).Finish()
```

That's it! This is a fully functioning pipeline.

You can run and test this pipeline much like you would any other code you write. Running it will produce
a protobuf binary output which Gofer uses to pass to the server.

## Full Example

```go
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
```

## Extra Examples

### Auto Inject API Tokens

Gofer has the ability to auto-create and inject a token into your tasks. This is helpful if you
want to use the [Gofer CLI](../../cli/README.md) or the Gofer API to communicate with Gofer at
some point in your task.

You can tell Gofer to do this by using the `InjectAPIToken` function for a particular task.

The token will be cleaned up the same time the logs for a particular run is cleaned up.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
    Description("This pipeline is purely for testing purposes.").
    Tasks(
		sdk.NewCustomTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!").InjectAPIToken(true),
    ).Finish()
```
