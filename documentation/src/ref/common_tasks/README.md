# Common Tasks

Triggers are Gofer's way of automating pipeline runs. Triggers are registered to your pipeline on creation and will alert Gofer when it's time to run your pipeline when some event has occurred.

The most straight-forward example of this is the event of passing time. Let's say you have a pipeline that needs to run every 5 mins. You would set your [pipeline](../pipeline-configuration/trigger/trigger-stanza) up with the [interval](interval/overview) trigger set to an interval of `5m`.

On startup, Gofer launches the interval trigger as a long-running container. When your pipeline is created, it "subscribes" to the interval trigger with an interval of `5m`. The interval trigger starts a timer and when 5 minutes have passed an event is sent from the trigger to Gofer, causing Gofer to run your pipeline.

## Gofer Provided Triggers

| name                      | image                                                | included by default | description                                                                                         |
| ------------------------- | ---------------------------------------------------- | ------------------- | --------------------------------------------------------------------------------------------------- |
| [interval](./interval.md) | ghcr.io/clintjedwards/gofer/triggers/interval:latest | yes                 | Interval triggers an event after a predetermined amount of time has passed.                         |
| [cron](./cron.md)         | ghcr.io/clintjedwards/gofer/triggers/cron:latest     | yes                 | Cron is used for longer termed intervals. For instance, running a pipeline every year on Christmas. |
| [github](./github.md)     | ghcr.io/clintjedwards/gofer/triggers/github:latest   | no                  | Allow your pipelines to run based on branch, tag, or release activity.                              |

## How to add new Triggers?

Just like tasks, triggers are simply docker containers! Making them easily testable and portable. To create a new trigger you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

The SDK provides an interface in which a well functioning GRPC service will be created from your concrete implementation.

```go
{{#include ../../../../sdk/go/plugins/trigger.go:29:59}}
```
