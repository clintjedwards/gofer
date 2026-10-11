# Tasks

Gofer's abstraction for running a container is called a Task. Specifically Tasks are containers you point Gofer to and
configure to perform some workload.

A Task can be any container you want to run. In the
[Getting Started](../../guide/create_your_first_pipeline_configuration.md) example we take a regular standard
`ubuntu:latest` container and customize it to run a passed in bash script.

```go
Tasks(
    sdk.NewTask("simple-task", "ubuntu:latest").
        Description("This task simply prints our hello-world message and exits!").
        Command("echo", "Hello from Gofer!"),
)
```

## Task Dependencies (DAGs)

By default every task in a pipeline starts as soon as the run does, so tasks run in parallel. To make a task wait on
another one, use `DependsOn` and say what state the parent has to end in:

| Status                        | The child runs when the parent...        |
| ----------------------------- | ---------------------------------------- |
| `RequiredParentStatusSuccess` | finishes successfully.                   |
| `RequiredParentStatusFailure` | fails.                                   |
| `RequiredParentStatusAny`     | finishes in any way, even being skipped. |

If the parent doesn't end in the required state, the child is skipped. Skipped tasks count as neither success nor
failure, so whole branches below a skipped task get skipped too, except for tasks that depend on it with `Any`.

```go
Tasks(
    sdk.NewTask("run-tests", "alpine:latest").
        Command("sh", "-c", "./run_tests.sh"),

    // Only publish if the tests pass.
    sdk.NewTask("publish-release", "alpine:latest").
        DependsOn("run-tests", sdk.RequiredParentStatusSuccess).
        Command("echo", "publishing release"),

    // Tell someone the tests broke.
    sdk.NewTask("alert", "alpine:latest").
        DependsOn("run-tests", sdk.RequiredParentStatusFailure).
        Command("echo", "the tests failed"),
)
```

A task can depend on several parents with `DependsOnMany`; it runs once all of them have finished in their required
states. The [dag example pipeline](https://github.com/clintjedwards/gofer/tree/main/examplePipelines/go/dag) shows a
few of these together. `gofer run debug <pipeline> <run>` is handy here since it shows why each task was skipped.

## Task Environment Variables and Configuration

Gofer handles container configuration [the cloud native way](https://12factor.net/config). That is to say every
configuration is passed in as an environment variable. This allows for many advantages, the greatest of
which is standardization.

When a container is run by Gofer, its environment variables come from three places:

1. **Run variables:** Values passed in when a run is started. You can pass them yourself with
   `gofer pipeline run my-pipeline -v KEY=VALUE` (repeat `-v` for more than one), and extensions pass them in when
   they start a run; the [github extension](../extensions/provided/github.md), for example, passes in details about
   the commit that triggered it.
2. **Your pipeline configuration:** Values you set with the `Variables` function on a task.
3. **Gofer's system variables:** Values Gofer sets on every task, listed below.

If the same name comes from more than one place, the one higher on this list wins. So a run variable can override a
value from your pipeline configuration for a single run, and either one can override a system variable.

Variable names are always uppercased, so `Variables(map[string]string{"log_level": "debug"})` shows up in the
container as `LOG_LEVEL`.

You can see exactly what a task was given with `gofer task get <pipeline_id> <run_id> <task_id>`.

These are the system variables Gofer injects into every task:

| Key                 | Description                                                                                                                                                                                  |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GOFER_PIPELINE_ID` | The pipeline identification string.                                                                                                                                                          |
| `GOFER_RUN_ID`      | The run identification number.                                                                                                                                                               |
| `GOFER_TASK_ID`     | The task execution identification string.                                                                                                                                                    |
| `GOFER_TASK_IMAGE`  | The image name the task is currently running with.                                                                                                                                           |
| `GOFER_TOKEN`       | Optional. Only set when the task uses `InjectAPIToken`. It's the same variable the Gofer CLI reads its token from. See [passing data between tasks](../object_store/index.html#passing-data-between-tasks) for an example. |

## Using Secrets and Objects in Variables

A variable's value can pull from Gofer's [secret store](../secret_store/index.html) or
[object store](../object_store/index.html) instead of being written into the pipeline config. That keeps secrets out
of your code and lets one task hand a value to the next.

You do this with a special string as the value. The SDK has a helper function for each kind, or you can write the
string yourself:

| Go SDK                    | Rust SDK               | String                   | Pulls from                                                         |
| ------------------------- | ---------------------- | ------------------------ | ------------------------------------------------------------------ |
| `sdk.PipelineSecret(key)` | `pipeline_secret(key)` | `pipeline_secret{{key}}` | This pipeline's secrets.                                           |
| `sdk.GlobalSecret(key)`   | `global_secret(key)`   | `global_secret{{key}}`   | Global secrets. The secret has to allow your pipeline's namespace. |
| `sdk.PipelineObject(key)` | `pipeline_object(key)` | `pipeline_object{{key}}` | This pipeline's objects.                                           |
| `sdk.RunObject(key)`      | `run_object(key)`      | `run_object{{key}}`      | Objects stored on the current run.                                 |

```go
sdk.NewTask("deploy", "my-org/deployer:latest").
    Variables(map[string]string{
        "ENVIRONMENT":  "production",                    // Passed in as is.
        "DEPLOY_KEY":   sdk.PipelineSecret("deploy_key"), // Becomes the value of the pipeline secret "deploy_key".
        "BUILD_NUMBER": sdk.RunObject("build_number"),    // Becomes the value of the run object "build_number".
    })
```

Secrets and objects have to be stored before the task that uses them starts:

```bash
gofer secret pipeline put my-pipeline deploy_key        # Prompts for the value.
gofer secret global put slack_token -n "ops-.*"          # Admins only; usable from namespaces matching "ops-.*".
gofer pipeline object put my-pipeline logs_header ./header.txt
```

Gofer swaps in the real values right before each task starts, after its parents have finished. So a task can store
a run object and any task that depends on it can read it as a variable; see
[passing data between tasks](../object_store/index.html#passing-data-between-tasks) for a full example. Objects are
converted to UTF-8 text when they're inserted; if you need the raw bytes, fetch the object through the API inside
your task instead.

If a secret or object doesn't exist, or a global secret doesn't allow your namespace, the task fails before its
container starts and the reason says which key was missing.

## What happens when a task is run?

The high level flow is:

1. Gofer checks to make sure your task configuration is valid.
2. Gofer waits for the task's parents to finish and checks they ended in the states the task requires.
3. Gofer parses the task configuration's variables list. It replaces any secret or object strings with their actual
   values from the object or secret store.
4. Gofer then passes the details of your task to the configured scheduler, variables are passed in as environment variables.
5. Usually this means the scheduler will take the configuration and attempt to pull the `image` mentioned in the configuration.
6. Once the image is successfully pulled the container is then run with the settings passed.
