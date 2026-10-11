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
- `gofer task get <pipeline_id> <run_id> <task_id>` shows the exact variables the task's container was given.
- `gofer task attach <pipeline_id> <run_id> <task_id>` opens a shell in a task's container while it's still running,
  if the server allows it (`api.allow_task_attach`).

If the CLI is acting strangely, `gofer context` shows who your token belongs to, the CLI configuration it's using,
and the server's version and settings.

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
