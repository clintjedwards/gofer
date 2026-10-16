# Troubleshooting Gofer

This page provides various tips on how to troubleshoot and find issues/errors within Gofer.

## Debugging runs

When a run fails, start with:

```bash
gofer run debug <pipeline_id> <run_id>
```

It prints a timeline of every task in the run, then details on each task that failed, was cancelled, or was skipped,
including the last few lines of output from the failed ones. Skipped tasks show which dependency wasn't met, and
tasks that failed before starting show why (like a secret or object that doesn't exist).

From there:

- `gofer task logs <pipeline_id> <run_id> <task_id>` shows a task's full output.
- `gofer task get <pipeline_id> <run_id> <task_id>` shows the exact variables the task's container was given, and
  how long the task spent in each phase.
- `gofer task attach <pipeline_id> <run_id> <task_id>` opens a shell in a task's container while it's still running,
  if the server allows it (`api.allow_task_attach`).

### Slow runs

When a run takes longer than it should, the timeline in `gofer run debug` (and on the run's page in the web UI) splits
each task into the time it spent waiting on its dependencies, setting up, and running. Setting up is everything
between a task being ready to go and its container running: resolving secrets and objects, pulling the image, and
starting the container. `gofer task get` breaks that down further, so a slow image pull stands out.

If the run itself took a while to begin, it was probably queued behind the pipeline's parallelism limit; the run's
start time shows how long it waited.

## Debugging the CLI

If the CLI is acting strangely, `gofer context` shows which server it's talking to and that server's version, who
your token belongs to, which config file was read and where each setting came from. It still prints the local half
when the server can't be reached.

## Debugging extensions

Extensions are simply long running containers that internally wait for an event to happen and then communicate with Gofer through its API.

There are a few avenues to debug extensions:

- `gofer extension list` shows each extension's state. When one failed to start, the reason column says why; a
  missing global secret or a setting the extension doesn't take shows up there.
- `gofer extension reload --dry-run` rereads Gofer's config and shows which extensions differ from it, and which
  entries have problems, without changing anything.
- `gofer extension logs <id>` will stream an extension's logs.
- Each extension has a `/api/debug` endpoint that dumps debug information about that extension.

## Debugging Tasks

When tasks aren't working quite right, it helps to have some simple tasks that you can use to debug. Gofer provides a few of these to aid in debugging.

| Name | Image                                  | Description                                                                                                                                                              |
| ---- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| envs | ghcr.io/clintjedwards/gofer/debug/envs | Simply prints out all environment variables found                                                                                                                        |
| fail | ghcr.io/clintjedwards/gofer/debug/fail | Purposely exits with a non-zero exit code. Useful for testing that pipeline failures or alerting works correctly.                                                        |
| log  | ghcr.io/clintjedwards/gofer/debug/log  | Prints a couple paragraphs of log lines with 1 second in-between, useful as a container that takes a while to finish and testing that log following is working correctly |
| wait | ghcr.io/clintjedwards/gofer/debug/wait | Wait a specified amount of time and then successfully exits.                                                                                                             |
