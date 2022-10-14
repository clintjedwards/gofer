# Triggers

Triggers are Gofer's way of automating pipeline runs. Triggers are registered to your pipeline on creation and will alert Gofer when it's time to run your pipeline when some event has occurred.

The most straight-forward example of this is the event of passing time. Let's say you have a pipeline that needs to run every 5 mins. You would set your [pipeline](../pipeline_configuration/README.md) up with the [interval](../triggers/interval.md) trigger set to an interval of `5m`.

On startup, Gofer launches the interval trigger as a long-running container. When your pipeline is created, it "subscribes" to the interval trigger with an interval of `5m`. The interval trigger starts a timer and when 5 minutes have passed an event is sent from the trigger to Gofer, causing Gofer to run your pipeline.

## Gofer Provided Triggers

| name                                | image                                                | included by default | description                                                                                         |
| ----------------------------------- | ---------------------------------------------------- | ------------------- | --------------------------------------------------------------------------------------------------- |
| [interval](../triggers/interval.md) | ghcr.io/clintjedwards/gofer/triggers/interval:latest | yes                 | Interval triggers an event after a predetermined amount of time has passed.                         |
| [cron](../triggers/cron.md)         | ghcr.io/clintjedwards/gofer/triggers/cron:latest     | yes                 | Cron is used for longer termed intervals. For instance, running a pipeline every year on Christmas. |
| [github](../triggers/github.md)     | ghcr.io/clintjedwards/gofer/triggers/github:latest   | no                  | Allow your pipelines to run based on branch, tag, or release activity.                              |

## How do I install a Trigger?

Triggers are installed by the CLI. For more information run:

```bash
gofer triggers install -h
```

## How do I configure a Trigger?

Triggers allow for both system and user configuration[^1]. This is what makes them so dynamically useful!

### Pipeline Configuration

Most Triggers allow for some user specific configuration usually referred to as "Parameters" or "Pipeline configuration".

These variables are passed by the pipeline configuration file into the Trigger when the pipeline is registered.

### System Configuration

Most Triggers have system configurations which allow the administrator or system to inject some needed variables. These are defined when the Trigger is installed.

[^1]: See the Trigger's documentation for the exact variables and where they belong.

## How to add new Triggers?

Just like tasks, triggers are simply docker containers! Making them easily testable and portable. To create a new trigger you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

The SDK provides an interface in which a well functioning GRPC service will be created from your concrete implementation.

```go
{{#include ../../../../sdk/go/plugins/trigger.go:29:59}}
```
