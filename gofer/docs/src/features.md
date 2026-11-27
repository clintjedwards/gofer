# Feature Guide

## Write your pipelines in a real programming language.

Other infrastructure tooling tried configuration languages(yaml, hcl).... and they kinda suck[^1]. The Gofer CLI allows
you to create your pipelines in a fully featured programming language. Pipelines can currently be written in Go or
Rust[^2].

## Images built from your code.

A task says what its image is made of: a base image plus files to copy in and commands to run, or a Dockerfile.
`gofer up` builds the images on your machine and uploads them with the pipeline config, so each version of a pipeline
runs exactly what it was registered with, and rolling back brings back the old images too. Only the tasks that changed
are built again, Go and Rust compiler caches are kept between builds, and a task that's only a base image and a
command doesn't need Docker at all. See [the task's image](./ref/pipeline_configuration/tasks.md#the-tasks-image).

## DAG(Directed Acyclic Graph) Support.

Gofer provides the ability to run your containers in reference to your other containers.

With DAG support you can run containers:

- In parallel.
- After other containers.
- When particular containers fail.
- When particular containers succeed.

## OpenAPI compatible

Gofer uses [OpenAPI](https://www.openapis.org/) to construct its API surface. This means that Gofer's API is easy to
use, well defined, and can easily be developed for in any language.

The use of OpenAPI gives us three main advantages:

1. The most up-to-date API contract can always be found by reading
   [the openapi spec files](https://github.com/clintjedwards/gofer/blob/main/gofer/docs/src/assets/openapi.json)
   included in the source.
2. Developing against the API for developers working within Golang/Rust simply means importing the provided sdk.
3. Developing against the API for developers not working within the Go/Rust language means simply generating an sdk
   using your [language specific tooling](https://openapi.tools/)

## Namespaces

Gofer allows you to separate out your pipelines into different namespaces, allowing you to organize your teams and set
permissions based on those namespaces.

## Extensions

Extensions are the way users can add extra functionality to their pipelines. For instance the ability to automate their
pipelines by waiting on bespoke events (like the passage of time).

Extensions are nothing more than containers themselves that talk to the main process when they require activity.

Gofer out of the box provides some default extensions like [cron](./ref/extensions/provided/cron.md) and
[interval](./ref/extensions/provided/interval.md). But even more powerful than that, it accepts any type of extension
you can think up and code using the included extension sdk.

You can view how extensions work by visiting the sample extension
['interval'](https://github.com/clintjedwards/gofer/blob/main/containers/extensions/interval)

Extensions are brought up alongside Gofer as long-running containers that it launches and manages.

## Sharing Files Between Tasks

Every task in a run shares a folder at `/gofer/run`, so one task can leave files for the tasks after it, the same as on
one machine. It's deleted when the run ends.
[More about passing data between tasks.](./guide/patterns/passing_data_between_tasks.md)

## Object Store

Gofer provides a built in [object store](./ref/object_store/index.html) you can access with the Gofer CLI. It's for
values and files that need to outlast a run, like caches.

## Secret Store

Gofer provides a built in [secret store](./ref/secret_store/index.html) you can access with the Gofer CLI. This secret
store provides a way to pass secret values needed by your pipeline configuration into Gofer.

## Events

Gofer provides a list of events for the most common actions performed. You can view this event stream via the Gofer API,
allowing you to build on top of Gofer's actions and even using Gofer as a trigger for other events.

## External Events

Gofer allows extensions to consume external events. This allows for extensions to respond to webhooks from favorite
sites like Github and more.

## Pluggable Storage

Gofer's object store and secret store each sit behind a small interface, so another backend, like
[AWS S3](https://aws.amazon.com/s3/) for objects, can be added without changing the rest of Gofer. It comes with a
[filesystem object store](./ref/object_store/filesystem.md) and a [sqlite secret store](./ref/secret_store/sqlite.md),
which need no setup. Containers run on [Docker](./ref/docker.md).

[^1]: Initially why configuration languages are used made sense, namely lowering the bar for users who might not know
    how to program and making it simpler overall to maintain(read: not shoot yourself in the foot with crazy inheritance
    structures). But, in practice, we've found that they kinda suck. Nobody wants to learn yet another language for this
    one specific thing. Furthermore, using a separate configuration language doesn't allow you to plug into years of
    practice/tooling/testing teams have with a certain favorite language.

<!-- prettier-ignore -->
[^2]: All pipelines eventually reduce to json, so given the correct libraries your pipelines can be written in any
    language you like!
