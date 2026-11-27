# Writing Tasks

## Make tasks safe to run twice.

Write your code in whatever language you want, but make it idempotent. Runs get retried by hand, started twice by
accident, or cancelled halfway through, and Gofer doesn't guarantee a task only ever runs once. A deploy task that
checks what's already there before changing anything is much easier to live with than one that assumes a clean
slate.

Cancelling is part of this. When a task is cancelled, Gofer asks its container to stop (Docker sends `SIGTERM`) and then
kills it if it's still running after `task_execution_stop_timeout_seconds`, 5 minutes by default. If your task holds a
lock or is halfway through a write when that happens, catch the signal and clean up, or make sure the next run can pick
up from a half finished state.

## Give long tasks a timeout.

A task that hangs holds its run open, and a slot in the pipeline's parallelism, until something stops it. Give tasks
a timeout close to how long they should actually take, with `Timeout(10 * time.Minute)` in Go or
`.timeout(Duration::from_secs(600))` in Rust. The clock starts when the container starts, so loading the image doesn't
count.

When a task runs past its timeout, Gofer stops it the same way it does for a cancel, then marks the task and the run
as failed with the reason `timeout`. The description says which limit was hit: the task's own, or the server's
`task_execution_max_timeout_seconds` (24 hours by default). The server's limit also applies to tasks that don't set a
timeout and caps any that ask for longer.

## Take configuration from environment variables.

Gofer passes everything into a task as environment variables, so your code should read its input and configuration
from them, the way [12-factor apps](https://12factor.net/config) do. That keeps one image usable across pipelines and
lets you override a value for a single run with `gofer pipeline run my-pipeline -v KEY=VALUE`. See
[task environment variables](../../ref/pipeline_configuration/tasks.md#task-environment-variables-and-configuration)
for where variables come from and which one wins.

## Pin your base images.

Each version of a pipeline runs exactly the images `gofer up` built for it, so a version never changes under you.
Which base a tag like `:latest` gave you is decided when the image is built, though, and a task is only built again
when it changes. Two tasks on `python:3` can end up on different Pythons, depending on when each one last changed.
A version tag (`python:3.13-slim`) or a digest (`python@sha256:...`) makes builds come out the same whenever they
happen. When you do want what a tag points to now, `gofer up --rebuild` builds every task from scratch.

## Keep your images lean.

Every image `gofer up` builds is uploaded to Gofer, kept for the pipeline's newest versions, and loaded into Docker
the first time it runs. Large images take longer each of those times and use more disk. Start from a small base image,
and use [`Keep`](../../ref/pipeline_configuration/tasks.md#keeping-only-whats-needed) to leave compilers and build
tools behind.

## Fail loudly.

Gofer decides whether a task succeeded from its exit code, and
[task dependencies](../../ref/pipeline_configuration/tasks.md#task-dependencies-dags) decide what runs next from that.
A script that swallows an error and exits 0 lets the publish step run after tests that actually failed. Start shell
scripts with `set -e` (or `set -eu`), and make sure your own programs exit non-zero when something goes wrong.
