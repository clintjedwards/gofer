# Custom Tasks

Gofer's abstraction for running a container is called a Task. Specifically Custom Tasks are containers you point Gofer to and configure to perform some workload.

A Custom Task can be any Docker container you want to run. In the [Getting Started](../../guide/create_your_first_pipeline_configuration.md) example we take a regular standard `ubuntu:latest` container and customize it to run a passed in bash script.

```go
WithTasks(
    sdk.NewCustomTask("simple_task", "ubuntu:latest").
        WithDescription("This task simply prints our hello-world message and exists!").
        WithCommand("echo", "Hello from Gofer!"),
)
```

// We need to combine the environment variables we get from multiple sources in order to pass them
// finally to the task run. The order in which they are passed is very important as they can and should
// overwrite each other, even though the intention of prefixing the environment variables is to prevent
// the chance of overwriting. The order in which they are passed into the extend function
// determines the priority in reverse order. Last in the stack will overwrite any conflicts from the others.
//
// There are many places a task_run could potentially get env vars from. From the outer most layer to the inner most:
// 1) The user sets variables in their pipeline configuration for each task.
// 2) At the time of run inception, either the trigger or the user themselves have the ability to inject extra env vars.
// 3) Right before the task run starts, Gofer itself might inject variables into the task run.
//
// The order in which the env vars are stacked are in reverse order to the above, due to that order being the best
// for giving the user the most control over what the pipeline does:
// 1) We first pass in the Gofer system specific envvars as these are the most replaceable on the totem pole.
// 2) We pass in the task specific envvars defined by the user in the pipeline config.
// 3) Lastly we pass in the run specific defined envvars. These are usually provided by either a trigger
// or the user when they attempt to start a new run manually. Since these are the most likely to be
// edited adhoc they are treated as the most important.

## How do I pass in variables to my task/container?

Gofer handles container configuration [the cloud native way](https://12factor.net/config). That is to say everything is passed in as an environment variable. This allows the program inside the container to read in it's configuration in a standardized way.

Gofer passes in the variables configured for each task in your pipeline configuration and also passes in a few others that might be useful to develop against:

## What happens when a task is run?

The high level flow is:

1. Gofer checks to make sure your task configuration is valid.
2. Gofer parses the task configuration's variables list. It attempts replace any substitution variables with their actual values from the object or secret store.
3. Gofer then passes the details of your task to the configured scheduler, variables are passed in as environment variables.
4. Usually this means the scheduler will take the configuration and attempt to pull the `image` mentioned in the configuration.
5. Once the image is successfully pulled the container is then run with the settings passed.
