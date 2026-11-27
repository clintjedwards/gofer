# Tasks

A task is one container in a pipeline. It says what its image is made of, what to run in it, and when to run it
compared to the pipeline's other tasks.

In the [Getting Started](../../guide/create_your_first_pipeline_configuration.md) example, the task starts from
`ubuntu:latest` and runs a single command:

```go
pipeline.Task(sdk.NewTask("simple-task", "ubuntu:latest").
    Description("This task simply prints our hello-world message and exits!").
    Command("echo", "Hello from Gofer!"))
```

## The Task's Image

`gofer up` builds every task's image on your machine and uploads it to Gofer along with the pipeline config. A version
of a pipeline always runs exactly the images it was registered with, so a run on Friday runs the same thing as one on
Monday, even if a tag like `ubuntu:latest` moved in between.

`NewTask(id, base)` starts the image from a base image, like `golang:1.25` or `python:3-slim`. With nothing added, as
above, the image is the base image with your command.

### Copying files and running commands

`Copy` puts files from next to your pipeline config into the image, and `Run` runs a command in it. They happen in the
order you write them:

```go
sdk.NewTask("cleanup", "python:3-slim").
    Copy("./cleanup.py", "/cleanup.py").
    Run("pip install requests").
    Command("python", "/cleanup.py")
```

```rust
Task::new("cleanup", "python:3-slim")
    .copy("./cleanup.py", "/cleanup.py")
    .run("pip install requests")
    .command(vec!["python".into(), "/cleanup.py".into()])
```

- `Copy(from, to)` copies a file, or what's inside a folder. `from` is relative to the folder of the pipeline config,
  and can be outside it, like `../shared`. The folder's `.dockerignore` decides what to leave out, and so do
  folders that only hold a cache, like Cargo's `target/`. Copied files belong to root and keep only their executable
  bit, so the image comes out the same on every machine.
- `Run(command)` runs the command with `/bin/sh -c`, so the image needs a shell.

Variables and secrets aren't there while the image is built; a task gets them when it runs. That keeps them out of the
image, but it also means a run step can't use a secret to download something private. Vendor those dependencies, or
build them outside of Gofer and copy the result in.

### Keeping only what's needed

A build often needs far more than the program it makes. `Keep` moves only the paths you name onto a smaller base once
every step has run:

```go
sdk.NewTask("report", "golang:1.25").
    Copy("./report", "/src").
    Run("cd /src && go build -o /app/report .").
    Keep([]string{"/app/report"}, "debian:13-slim").
    Command("/app/report")
```

The compiler stays behind, so the image uploads and starts faster. Paths have to be full paths, and each one lands at
the same path on the new base. Nothing else carries over from the first base, like its environment variables or
working directory, so the new base needs whatever the kept files need to run.

### Compiler caches

Gofer keeps Go's and Cargo's caches between builds on its own, so a plain `go build` or `cargo build --release` is
only slow the first time. Put the `cargo build` in the first run step after your copies. A step in between could make
code that Cargo wouldn't notice changing, like a crate generated from a spec, so the steps after it go without the
Rust cache, and `gofer up` prints a tip saying which step was the reason.

### Dockerfiles

For anything `Copy` and `Run` can't do, build from a Dockerfile. Both paths are relative to the folder of the pipeline
config, and the context is the folder the Dockerfile can use files from, the same as with `docker build`:

```go
sdk.NewDockerfileTask("scan", "./scan/Dockerfile", "./scan")
```

The context's `.dockerignore` works the way it does with Docker, and so does a `Dockerfile.dockerignore` next to the
Dockerfile. Gofer doesn't pass build arguments, so each `ARG` has its default.

### How the image runs

`Command` and `Entrypoint` replace the image's own, the same way `docker run` does. `Script` runs a shell script with
`bash` if the image has it and `sh` if it doesn't. All three work for Dockerfile tasks too. They're built into the
image, so changing one makes a new image, which is quick since only the image's config changes.

### When images are built

`gofer up` hashes the files each task copies in, and only builds a task again when its files, how it's built or how
it runs changed. The rest keep the image they already have. The changes `gofer up` shows say
`tasks.<id>.files: changed` when it's a task's files.

- A task that's only a base image and a command doesn't need Docker. Gofer reads the base from its registry and only
  changes the image's config. Anything with `Copy`, `Run`, `Keep` or a Dockerfile is built with Docker, on the machine
  that runs `gofer up`.
- `gofer up --rebuild` builds every task from scratch, with empty compiler caches. Use it when a base image's tag has
  moved and you want what it points to now.
- Base images are looked up on every build, so building needs the network even when everything is cached.
- Images are always for linux/amd64. On another kind of machine, like an ARM Mac, builds use emulation, which can make
  compiling slow.
- When a build fails, `gofer up` shows its last lines and where its full log is, under `~/.cache/gofer/builds/`.

Gofer keeps the images of each pipeline's newest versions, set by `images.retention_versions` in the
[server configuration](../server_configuration/configuration_reference.md#images). On your side, `gofer builder prune`
empties the builder's cache, which never grows past `builder_cache_max_gb` in the
[CLI configuration](../../cli/configuration.md).

### Private base images

If a task's base image is in a registry that needs a login, log in with Docker the way you usually would, then run
`gofer up`:

```bash
docker login registry.example.com
```

`gofer up` builds the image on your machine with the same logins Docker uses, including credential helpers and
`DOCKER_AUTH_CONFIG`. A build only gets the logins for the registries it downloads from, and they never end up in the
image. The Gofer server never needs them, since it runs the image `gofer up` uploaded instead of downloading one.

In a Dockerfile, a `FROM` can use a variable from an `ARG`, like `FROM ${BASE}`, and Gofer uses the `ARG`'s default to
work out which registry the image is in.

Some registries hand out an identity token in place of a password. Gofer can use one of those for a task it builds with
Docker, but not yet for a task that's only a base image and a command, since that's downloaded without Docker.

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
pipeline.Task(sdk.NewTask("run-tests", "alpine:latest").
    Command("sh", "-c", "./run_tests.sh"))

// Only publish if the tests pass.
pipeline.Task(sdk.NewTask("publish-release", "alpine:latest").
    DependsOn("run-tests", sdk.RequiredParentStatusSuccess).
    Command("echo", "publishing release"))

// Tell someone the tests broke.
pipeline.Task(sdk.NewTask("alert", "alpine:latest").
    DependsOn("run-tests", sdk.RequiredParentStatusFailure).
    Command("echo", "the tests failed"))
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

The exceptions are `GOFER_API_BASE_URL` and `GOFER_TOKEN`, which a run variable can't set; starting a run with
either one is rejected. A task with `InjectAPIToken` sends its token to whatever `GOFER_API_BASE_URL` points at, so if
anyone who can start a run could change it, they could collect a token that rewrites the pipeline.

Your pipeline configuration can still override any `GOFER_` variable, and the task trusts whatever you set, so be
careful doing that.

Variable names are always uppercased, so `Variables(map[string]string{"log_level": "debug"})` shows up in the
container as `LOG_LEVEL`. Names can't be empty or contain `=`, since they become environment variable names.

You can see exactly what a task was given with `gofer task get <pipeline_id> <run_id> <task_id>`.

These are the system variables Gofer injects into every task:

| Key                 | Description                                                                                                                                                                                  |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GOFER_NAMESPACE`   | The namespace the pipeline lives in.                                                                                                                                                         |
| `GOFER_API_BASE_URL`| The address the task can reach Gofer's API at, taken from the server's `server.extension_address`.                                                                                          |
| `GOFER_PIPELINE_ID` | The pipeline identification string.                                                                                                                                                          |
| `GOFER_RUN_ID`      | The run identification number.                                                                                                                                                               |
| `GOFER_TASK_ID`     | The task execution identification string.                                                                                                                                                    |
| `GOFER_TASK_IMAGE`  | The digest of the image the task runs, the one `gofer up` built for it.                                                                                                                      |
| `GOFER_TOKEN`       | Optional. Only set when the task uses `InjectAPIToken`. It's the same variable the Gofer CLI reads its token from. See [passing data between tasks](../../guide/patterns/passing_data_between_tasks.md) for an example. |

## The gofer CLI

Every task can run the [Gofer CLI](../../cli/index.html) as `gofer`, whatever its image is. It's Gofer's own
binary, so it's always the same version as the server, and Gofer adds it to the container when the task starts
rather than putting it in the image:

- It's at `/gofer/bin/gofer`, and `/gofer/bin` goes on the end of the task's PATH. An image that has a `gofer` of its
  own keeps using that one.
- Gofer sets the variables it needs, so a task with `InjectAPIToken` can run commands like
  `gofer pipeline object put report /gofer/run/report.html --force` with no setup.
- It works in images without a shell or CA certificates, like distroless or `debian:13-slim`. Over HTTPS, the image
  needs whatever certificates Gofer's own certificate is signed with, or the CLI uses the public ones Mozilla
  publishes when the image has none.
- Nothing about a task needs it, so task code doesn't have to know about Gofer.

It's only there when Gofer itself is a static build, which every build from the repository is. A Gofer built for
glibc logs that tasks won't get it when it starts, since that binary won't start in images like Alpine.

## Using Secrets and Objects in Variables

A variable's value can pull from Gofer's [secret store](../secret_store/index.html) or
[object store](../object_store/index.html) instead of being written into the pipeline config. That keeps secrets out
of your code.

You do this with a special string as the value. The SDK has a helper function for each kind, or you can write the
string yourself:

| Go SDK                    | Rust SDK               | String                   | Pulls from                                                         |
| ------------------------- | ---------------------- | ------------------------ | ------------------------------------------------------------------ |
| `sdk.PipelineSecret(key)` | `pipeline_secret(key)` | `pipeline_secret{{key}}` | This pipeline's secrets.                                           |
| `sdk.GlobalSecret(key)`   | `global_secret(key)`   | `global_secret{{key}}`   | Global secrets. The secret has to allow your pipeline's namespace. |
| `sdk.PipelineObject(key)` | `pipeline_object(key)` | `pipeline_object{{key}}` | This pipeline's objects.                                           |

```go
sdk.NewTask("deploy", "my-org/deployer:1.4").
    Variables(map[string]string{
        "ENVIRONMENT": "production",                     // Passed in as is.
        "DEPLOY_KEY":  sdk.PipelineSecret("deploy_key"),  // Becomes the value of the pipeline secret "deploy_key".
        "HEADER":      sdk.PipelineObject("logs_header"), // Becomes the value of the pipeline object "logs_header".
    })
```

Secrets and objects have to be stored before the task that uses them starts:

```bash
gofer secret pipeline put my-pipeline deploy_key        # Prompts for the value.
gofer secret global put slack_token -n "ops-.*"          # Admins only; usable from namespaces matching "ops-.*".
gofer pipeline object put logs_header ./header.txt --pipeline my-pipeline
```

Gofer swaps in the real values right before each task starts, after its parents have finished. Objects are converted
to UTF-8 text when they're inserted; if you need the raw bytes, fetch the object through the API inside your task
instead. To hand a value from one task to the next within a run, write it to the folder every task in the run shares;
see [passing data between tasks](../../guide/patterns/passing_data_between_tasks.md).

If a secret or object doesn't exist, or a global secret doesn't allow your namespace, the task fails before its
container starts and the reason says which key was missing.

## What happens when a task is run?

The high level flow is:

1. Gofer checks to make sure your task configuration is valid.
2. Gofer waits for the task's parents to finish and checks they ended in the states the task requires.
3. Gofer parses the task configuration's variables list. It replaces any secret or object strings
   with their actual values from the object or secret store.
4. Gofer loads the task's image into Docker from what `gofer up` uploaded, if Docker doesn't have it already.
5. Gofer starts the container, with the variables as environment variables, the run's folder at `/gofer/run`, and
   the `gofer` CLI at `/gofer/bin/gofer`.
