# Extensions

Extensions are Gofer's way of automating pipeline runs. Extensions are registered to your pipeline on creation and will alert Gofer when it's time to run your pipeline (usually when some event has occurred).

The most straight-forward example of this, is the event of passing time. Let's say you have a pipeline that needs to run every 5 mins. You would set your [pipeline](../pipeline_configuration/README.md) up with the [interval](./provided/interval.md) extension set to an interval of `5m`.

On startup, Gofer launches the interval extension as a long-running container. When your pipeline is created, it "subscribes" to the interval extension with an interval of `5m`. The interval extension starts a timer and when 5 minutes have passed an event is sent from the extension to Gofer, causing Gofer to run your pipeline.

## Gofer Provided Extensions

You can [create](#how-to-add-new-extensions) your own extensions, but Gofer provides some [provided extensions](./provided/README.md) for use.

## How do I install a Extension?

Extensions must first be installed by Gofer administrators before they can be used. They can be installed by the CLI. For more information on how to install a specific extension run:

```bash
gofer extension install -h
```

## How do I configure a Extension?

Extensions allow for both system and pipeline configuration[^1]. This is what makes them so dynamically useful!

### Pipeline Configuration

Most Extensions allow for some user specific configuration usually referred to as "Parameters" or "Pipeline configuration".

These variables are passed by the pipeline configuration file into the Extension when the pipeline is registered.

### System Configuration

Most Extensions have system configurations which allow the administrator or system to inject some needed variables. These are defined when the Extension is installed.

[^1]: See a specific Extension's documentation for the exact variables accepted and where they belong.

## How to add new Extensions?

Just like tasks, extensions are simply docker containers! Making them easily testable and portable. To create a new extension you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

The SDK provides an interface in which a well functioning GRPC service will be created from your concrete implementation.

```go
{{#include ../../../../sdk/go/plugins/extension.go:29:59}}
```

For an commented example of a simple extension you can follow to build your own, view the interval extension:

```go
{{#include ../../../../containers/extensions/interval/main.go}}
```
