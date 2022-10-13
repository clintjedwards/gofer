# Common Tasks

Common Tasks are Gofer's way of allowing you to pre-setup tasks such that they can be set up once and used in multiple pipelines.

This allows you to do potentially complicated or single use setups that make it easier for different pipelines to consume without going through the same process.

An example of this might be having pipelines post to Slack. Setting up a new slack bot account for each and every pipeline that would want to post to slack is cumbersome and slows down productivity. Instead, Gofer's common tasks allow you to set up a single Slack bot, set up a single task, and have each pipeline just specify that task.

Common tasks work just like any other task except that they are registered just like triggers.

## Gofer Provided Common Tasks

| name                | image                                          | description                                                                                                                    |
| ------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| [debug](./debug.md) | ghcr.io/clintjedwards/gofer/tasks/debug:latest | Useful for debugging common tasks, simply prints out the env vars each run. A good example of how to setup other common tasks. |

## How to add new Common Tasks?

Just like custom tasks, common tasks are simply docker containers! Making them easily testable and portable. To create a new common task you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

The SDK provides simple functions to help in creating common tasks. To see an example of how a common task is structured and created view the [debug](./debug.md) task.
