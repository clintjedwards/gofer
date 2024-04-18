# Extensions

Extensions are Gofer's way of adding additional functionality to pipelines. You can subscribe your pipeline to an extension, allowing that extension to give your pipeline extra powers.

The most straight-forward example of this, is the interval extension. This extension allows your pipeline to run everytime some amount of time has passed. Let's say you have a pipeline that needs to run every 5 mins. You would subscribe your pipeline to the [interval](./provided/interval.md) extension using the gofer cli command `gofer extension sub internal every_5_seconds` set to an interval of `5m`.

On startup, Gofer launches the interval extension as a long-running container. When your pipeline subscribes to it. The interval extension starts a timer and when 5 minutes have passed the extension sends an API request to Gofer, causing Gofer to run your pipeline.

## Gofer Provided Extensions

You can [create](#how-to-add-new-extensions) your own extensions, but Gofer provides some [provided extensions](./provided/index.html) for use.

## How do I install a Extension?

Extensions must first be installed by Gofer administrators before they can be used. They can be installed by the CLI. For more information on how to install a specific extension run:

```bash
gofer extension install -h
```

## How do I configure a Extension?

Extensions allow for both system and pipeline configuration[^1]. Meaning they have both Global settings that apply to all pipelines
and Pipeline specific settings. This is what makes them so dynamically useful!

### Pipeline Configuration

Most Extensions allow for some pipeline specific configuration usually referred to as "Parameters" or "Pipeline configuration".

These variables are passed when the user subscribes their pipeline to the extension. Each extension defines what this might be
in it's documentation.

### System Configuration

Most extensions have system configurations which allow the administrator or system to inject some needed variables. These are defined when the Extension is installed.

[^1]: See a specific Extension's documentation for the exact variables accepted and where they belong.

## How to add new Extensions/ How do I create my own?

Just like tasks, extensions are simply containers! Making them easily testable and portable. To create a new extension you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

The SDK provides an interface in which a well functioning GRPC service will be created from your concrete implementation.

//TODO()

<!-- ```go
```

For an commented example of a simple extension you can follow to build your own, view the [interval extension](https://github.com/clintjedwards/gofer/tree/main/containers/extensions/interval):

```go
``` -->
