# Tasks

Gofer's abstraction for running a container is called a Task. Specifically Tasks are containers you point Gofer to and configure to perform some workload.

A Task can be any Docker container you want to run. In the [Getting Started](../../guide/create_your_first_pipeline_configuration.md) example we take a regular standard `ubuntu:latest` container and customize it to run a passed in bash script.

```go
Tasks(
    sdk.NewTask("simple_task", "ubuntu:latest").
        Description("This task simply prints our hello-world message and exists!").
        Command("echo", "Hello from Gofer!"),
)
```

## Task Environment Variables and Configuration

Gofer handles container configuration [the cloud native way](https://12factor.net/config). That is to say every configuration is passed in as an environment variable. This allows for many advantages, the greatest of which is standardization.

As a user, [you pass your configuration in via the `Variable(s)`](https://pkg.go.dev/github.com/clintjedwards/gofer@v0.5.0/sdk/go/config#TaskConfig.Variables) flavor of functions in your pipeline-config.

When a container is run by Gofer, the Gofer scheduler has the potential to pass in configuration from multiple sources[^1]:

1. **Your pipeline configuration:** Configs you pass in by using the `Variable(s)` functions.
2. **Extension/Manual configurations:** Extensions are allowed to pass in custom configuration for a run. Usually this configuration gives extra information the run might need. (For example, the git commit that activated the extension.).

   Alternatively, if this run was not activated by a extension and instead kicked of manually, the user who launched the run might opt to pass in configuration at that runtime.

3. **Gofer's system configurations:** Gofer will pass in system configurations that might be helpful to the user. (For example, what current pipeline is running.)[^2]

The exact key names injected for each of these configurations can be seen on any taskrun by getting that taskrun's details: `gofer taskrun get <pipeline_name> <run_id>`

[^1]: These sources are ordered from most to least important. Since the configuration is passed in a "Key => Value" format any conflicts between sources will default to the source with the greater importance. For instance, a pipeline config with the key `GOFER_PIPELINE_ID` will replace the key of the same name later injected by the Gofer system itself.

<!-- prettier-ignore -->
[^2]: The current Gofer system injected variables can be found [here.](https://github.com/clintjedwards/gofer/blob/40512915a3ae4cd140f5c855bbff631793c380fb/internal/api/runs.go#L56-L57) Below is a possibly out of date short reference:

| Key                 | Description                                                                                                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GOFER_PIPELINE_ID` | The pipeline identification string.                                                                                                                                             |
| `GOFER_RUN_ID`      | The run identification number.                                                                                                                                                  |
| `GOFER_TASK_ID`     | The task run identification string.                                                                                                                                             |
| `GOFER_TASK_IMAGE`  | The image name the task is currently running with.                                                                                                                              |
| `GOFER_API_TOKEN`   | Optional. Runs can be assigned a unique Gofer API token automatically. This makes it easy and manageable for tasks to query Gofer's API and do lots of other convenience tasks. |

## What happens when a task is run?

The high level flow is:

1. Gofer checks to make sure your task configuration is valid.
2. Gofer parses the task configuration's variables list. It attempts replace any substitution variables with their actual values from the object or secret store.
3. Gofer then passes the details of your task to the configured scheduler, variables are passed in as environment variables.
4. Usually this means the scheduler will take the configuration and attempt to pull the `image` mentioned in the configuration.
5. Once the image is successfully pulled the container is then run with the settings passed.
