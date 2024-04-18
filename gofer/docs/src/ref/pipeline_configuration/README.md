# Pipeline Configuration

A pipeline is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a run.
Gofer allows users to configure their pipeline via a configuration file written in [Golang](https://go.dev/) or
[Rust](https://www.rust-lang.org/).

The general hierarchy for a pipeline is:

```
namespace
   \_ pipeline
         \_ run
             \_ task
```

That is to say:

1. A namespace might contain multiple pipelines.
2. A pipeline is made up of tasks/containers to be run.
3. Every time a pipeline is executed it is done through the concept of a run which makes sure the containers are executed properly.

## SDK

Creating a pipeline involves using the SDK currently written in Go or Rust.

## Small Walkthrough

To introduce some of the concepts slowly, lets build a pipeline step by step. We'll be using Go as our pipeline
configuration language and this documentation assumes you've already set up a new Go project and are operating
in a `main.go` file. If you haven't you can set up one
[following the guide instructions.](../../guide/create_your_first_pipeline_configuration.md)

### A Simple Pipeline

Every pipeline is initialized with a simple pipeline declaration. It's here that we will name our pipeline,
giving it a machine referable ID and a human referable name.

```go
err := sdk.NewPipeline("simple", "My Simple Pipeline")
```

It's important to note here that while your human readable name ("My Simple Pipeline" in this case) can contain a large array of characters the ID can only container alphanumeric letters, numbers, and underscores. Any other characters will result in an error when attempting to register the pipeline.

### Add a Description

Next we'll add a simple description to remind us what this pipeline is used for.

```go
err := sdk.NewPipeline("simple", "My Simple Pipeline").
        Description("This pipeline is purely for testing purposes.")
```

The SDK uses a builder pattern, which allows us to simply add another function onto our Pipeline object
which we can type our description into.

### Add a task

Lastly let's add a task(container) to our pipeline. We'll add a simple ubuntu container and change the command that gets
run on container start to just say "Hello from Gofer!".

```go
err := sdk.NewPipeline("simple", "My Simple Pipeline").
        Description("This pipeline is purely for testing purposes.").
        Tasks(sdk.NewTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!"),
    )
```

We used the `Tasks` function to add multiple tasks and then we use the SDK's `NewTask` function to create a task.
You can see we:

- Give the task an ID, much like our pipeline earlier.
- Specify which image we want to use.
- Tack on a description.
- And then finally specify the command.

To tie a bow on it, we add the `.Finish()` function to specify that our pipeline is in it's final form.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
    Description("This pipeline is purely for testing purposes.").
    Tasks(sdk.NewTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!"),
    ).Finish()
```

That's it! This is a fully functioning pipeline.

You can run and test this pipeline much like you would any other code you write. Running it will produce
a JSON output which Gofer uses to pass to the server.

You can find examples like this and more in [example pipelines](https://github.com/clintjedwards/gofer/tree/main/examplePipelines)

## Extra Examples

### Auto Inject API Tokens

Gofer has the ability to auto-create and inject a token into your tasks. This is helpful if you
want to use the [Gofer CLI](../../cli/index.html) or the Gofer API to communicate with Gofer at
some point in your task.

You can tell Gofer to do this by using the `InjectAPIToken` function for a particular task.

The token will be cleaned up the same time the logs for a particular run is cleaned up.

```go
err := sdk.NewPipeline("my_pipeline", "My Simple Pipeline").
    Description("This pipeline is purely for testing purposes.").
    Tasks(
		sdk.NewTask("simple_task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exists!").
			Command("echo", "Hello from Gofer!").InjectAPIToken(true),
    ).Finish()
```
