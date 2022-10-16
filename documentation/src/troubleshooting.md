# Troubleshooting Gofer

This page provides various tips on how to troubleshoot and find issues/errors within Gofer.

## Debugging triggers

Triggers are simply long running docker containers that internally wait for an event to happen and then communicate with Gofer through a long GRPC poll on what should be the result of that event.

<!-- TODO(clintjedwards): Provide a debug Gofer trigger-->

## Debugging Common Tasks

Common Tasks are pre-setup containers that run at a user's request. Errors in common tasks should show up as errors for your pipeline as normal.

To aid in debugging common tasks in general, there is a [debug](./ref/common_tasks/debug.md) common task available. It simply prints out all environment variables found and takes some straight-forward parameters and configurations.

## Debugging Custom Tasks

When custom tasks aren't working quite right, it helps to have some simple tasks that you can use to debug. Gofer provides a few of these to aid in debugging.

| Name | Image                                  | Description                                                                                                                                                              |
| ---- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| envs | ghcr.io/clintjedwards/gofer/debug/envs | Simply prints out all environment variables found                                                                                                                        |
| fail | ghcr.io/clintjedwards/gofer/debug/fail | Purposely exist with a non-zero exit code. Useful for testing that pipeline failures or alerting works correctly.                                                        |
| log  | ghcr.io/clintjedwards/gofer/debug/log  | Prints a couple paragraphs of log lines with 1 second in-between, useful as a container that takes a while to finish and testing that log following is working correctly |
| wait | ghcr.io/clintjedwards/gofer/debug/wait | Wait a specified amount of time and then successfully exits.                                                                                                             |
