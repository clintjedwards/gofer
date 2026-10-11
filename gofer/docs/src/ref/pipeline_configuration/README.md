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

Creating a pipeline involves using the SDK currently written in Go or Rust. A pipeline config is just a small
program: it describes the pipeline with the SDK and prints it as JSON, and `gofer up` builds it, runs it, and sends
that JSON to Gofer.

- **Go:** `github.com/clintjedwards/gofer/sdk/go/config`. Every function is documented on
  [pkg.go.dev](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk/go/config).
- **Rust:** the `gofer_sdk` crate. It isn't published on crates.io, so depend on it from Git, using the tag that
  matches your Gofer version:

  ```toml
  [dependencies]
  gofer_sdk = { git = "https://github.com/clintjedwards/gofer", tag = "v<gofer version>" }
  ```

  `cargo doc --open` shows its documentation.

Both SDKs have the same functions; Go uses `CamelCase` names and Rust uses `snake_case`.

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

It's important to note here that while your human readable name ("My Simple Pipeline" in this case) can contain a large
array of characters the ID can only contain letters, numbers, and hyphens, and has to be 3 to 32 characters long. Any
other characters will result in an error when attempting to register the pipeline, due to each id needing to be URL
safe.

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
        Tasks(sdk.NewTask("simple-task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exits!").
			Command("echo", "Hello from Gofer!"),
    )
```

We used the `Tasks` function to add multiple tasks and then we use the SDK's `NewTask` function to create a task.
You can see we:

- Give the task an ID, much like our pipeline earlier.
- Specify which image we want to use.
- Tack on a description.
- And then finally specify the command.

To tie a bow on it, we add the `.Finish()` function to specify that our pipeline is in its final form.

```go
err := sdk.NewPipeline("my-pipeline", "My Simple Pipeline").
    Description("This pipeline is purely for testing purposes.").
    Tasks(sdk.NewTask("simple-task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exits!").
			Command("echo", "Hello from Gofer!"),
    ).Finish()
```

That's it! This is a fully functioning pipeline.

You can run and test this pipeline much like you would any other code you write. Running it will produce
a JSON output which Gofer uses to pass to the server.

You can find examples like this and more in [example pipelines](https://github.com/clintjedwards/gofer/tree/main/examplePipelines)

### The same pipeline in Rust

With `gofer_sdk` added to your `Cargo.toml` (see [SDK](#sdk) above), the same pipeline in `src/main.rs` looks like
this:

```rust
use gofer_sdk::config::{Pipeline, Task};

fn main() {
    Pipeline::new("my-pipeline", "My Simple Pipeline")
        .description("This pipeline is purely for testing purposes.")
        .tasks(vec![
            Task::new("simple-task", "ubuntu:latest")
                .description("This task simply prints our hello-world message and exits!")
                .command(vec!["echo".to_string(), "Hello from Gofer!".to_string()]),
        ])
        .finish()
        .unwrap();
}
```

`gofer up` works out which language a pipeline is written in by looking for a `go.mod` or `Cargo.toml` in the folder
you point it at.

## Extra Examples

### Auto Inject API Tokens

Gofer has the ability to auto-create and inject a token into your tasks. This is helpful if you
want to use the [Gofer CLI](../../cli/index.html) or the Gofer API to communicate with Gofer at
some point in your task.

You can tell Gofer to do this by using the `InjectAPIToken` function for a particular task. The token shows up in the
task as the `GOFER_TOKEN` environment variable, which is the same variable the Gofer CLI reads. Your task still needs
to be told where Gofer is; see [passing data between tasks](../object_store/index.html#passing-data-between-tasks)
for a full example.

The token is deleted when the run's objects expire (see `run_object_expiry` in the
[configuration reference](../server_configuration/configuration_reference.md)).

```go
err := sdk.NewPipeline("my-pipeline", "My Simple Pipeline").
    Description("This pipeline is purely for testing purposes.").
    Tasks(
		sdk.NewTask("simple-task", "ubuntu:latest").
			Description("This task simply prints our hello-world message and exits!").
			Command("echo", "Hello from Gofer!").InjectAPIToken(true),
    ).Finish()
```

### Limiting Concurrent Runs

By default a pipeline can have up to 200 runs going at once (the server's `pipeline_run_concurrency_limit`). Use
`Parallelism` to set a lower limit for one pipeline. For example, a deploy pipeline that should never run twice at the
same time:

```go
err := sdk.NewPipeline("deploy", "Deploy").
    Parallelism(1).
    Tasks(...).Finish()
```

Runs started while the pipeline is at its limit wait until an earlier one finishes. A pipeline can't set a limit
higher than the server's.

## Next Steps

- [Tasks](./tasks.md) covers everything a task can do: dependencies, variables, secrets, and objects.
- [Updating Pipelines](./updating.md) explains what happens when you `gofer up` a pipeline again.
- [Running Pipelines Automatically](./subscriptions.md) shows how to make a pipeline run on a schedule or when
  something happens.
