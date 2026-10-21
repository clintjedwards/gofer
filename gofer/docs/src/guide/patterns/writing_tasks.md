# Writing Tasks

## Make tasks safe to run twice.

Write your code in whatever language you want, but make it idempotent. Runs get retried by hand, started twice by
accident, or cancelled halfway through, and Gofer doesn't guarantee a task only ever runs once. A deploy task that
checks what's already there before changing anything is much easier to live with than one that assumes a clean
slate.

Cancelling is part of this. When a task is cancelled, Gofer asks its container to stop (Docker sends `SIGTERM`) and
then kills it if it's still running after `task_execution_stop_timeout_seconds`, 5 minutes by default. If your task holds a
lock or is halfway through a write when that happens, catch the signal and clean up, or make sure the next run can
pick up from a half finished state.

## Give long tasks a timeout.

A task that hangs holds its run open, and a slot in the pipeline's parallelism, until something stops it. Give tasks
a timeout close to how long they should actually take, with `Timeout(10 * time.Minute)` in Go or
`.timeout(Duration::from_secs(600))` in Rust. The clock starts when the container starts, so pulling the image doesn't
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

## Pin your image tags.

Gofer keeps every version of your pipeline config and records which version each run used, so you can always look
back at what ran. That only holds if the images are pinned too. With `:latest`, the config says the same thing on
Monday and Friday while the code inside the image changed in between.

The Docker scheduler also only pulls an image when the host doesn't have it yet. So without
`AlwaysPullNewestImage(true)`, a `:latest` task runs whatever version that host happened to pull last, and two hosts
can run different code for the same task. Prefer a version tag (`my-org/deployer:1.4.2`) or a digest
(`my-org/deployer@sha256:...`). Save `AlwaysPullNewestImage` for images where you really do want the newest build every
time, like your own tooling images, and accept that each run pays for the pull.

## Keep your images lean.

There's no guarantee a task lands on a machine that already has its image. Large images take a long time to download
and a lot of disk space, and that time comes out of every run that has to pull them. Start from a small base image and
only install what the task needs.

## Fail loudly.

Gofer decides whether a task succeeded from its exit code, and
[task dependencies](../../ref/pipeline_configuration/tasks.md#task-dependencies-dags) decide what runs next from that.
A script that swallows an error and exits 0 lets the publish step run after tests that actually failed. Start shell
scripts with `set -e` (or `set -eu`), and make sure your own programs exit non-zero when something goes wrong.
