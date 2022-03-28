---
id: overview
title: Overview
sidebar_position: 1
---

# Triggers

Triggers are Gofer's way of automating pipeline runs. Triggers are registered to your pipeline on creation and will alert Gofer when it's time to run your pipeline when some event has occurred.

The most straight-forward example of this is the event of passing time. Let's say you have a pipeline that needs to run every 5 mins. You would set your [pipeline](../pipeline-configuration/trigger/trigger-stanza) up with the [interval](interval/overview) trigger set to an interval of `5m`.

On startup, Gofer launches the interval trigger as a long-running container. When your pipeline is created, it "subscribes" to the interval trigger with an interval of `5m`. The interval trigger starts a timer and when 5 minutes have passed an event is sent from the trigger to Gofer, causing Gofer to run your pipeline.

## Supported Triggers

| name                          | image                                                           | included | description                                                                                         |
| ----------------------------- | --------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------------------- |
| [interval](interval/overview) | ghcr.io/clintjedwards/gofer-containers/triggers/interval:latest | yes      | Interval triggers an event after a predetermined amount of time has passed.                         |
| [cron](cron/overview)         | ghcr.io/clintjedwards/gofer-containers/triggers/cron:latest     | yes      | Cron is used for longer termed intervals. For instance, running a pipeline every year on Christmas. |
| [github](github/overview)     | ghcr.io/clintjedwards/gofer-containers/triggers/github:latest   | yes      | Allow your pipelines to run based on branch, tag, or release activity.                              |

## How to add new Triggers?

Just like [tasks](../pipeline-configuration/task/task-stanza), triggers are simply docker containers! Making them easily testable and portable. To create a new trigger you simply use the included [Gofer SDK](https://pkg.go.dev/github.com/clintjedwards/gofer/sdk).

The SDK provides an interface in which a well functioning GRPC service will be created from your concrete implementation.

```go
type TriggerServerInterface interface {
	// Check blocks until the trigger has a pipeline that should be run, then it returns. This is ideal for setting
	// the check endpoint as an channel result.
	Check(context.Context, *sdkProto.CheckRequest) (*sdkProto.CheckResponse, error)

	// Info returns information on the specific plugin
	Info(context.Context, *sdkProto.InfoRequest) (*sdkProto.InfoResponse, error)

	// Subscribe allows a trigger to keep track of all pipelines currently
	// dependant on that trigger so that we can trigger them at appropriate times.
	Subscribe(context.Context, *sdkProto.SubscribeRequest) (*sdkProto.SubscribeResponse, error)

	// Unsubscribe allows pipelines to remove their trigger subscriptions. This is
	// useful if the pipeline no longer needs to be notified about a specific
	// trigger automation.
	Unsubscribe(context.Context, *sdkProto.UnsubscribeRequest) (*sdkProto.UnsubscribeResponse, error)

	// Shutdown tells the trigger to cleanup and gracefully shutdown. If a trigger
	// does not shutdown in a time defined by the gofer API the trigger will
	// instead be Force shutdown(SIGKILL). This is to say that all triggers should
	// lean toward quick cleanups and shutdowns.
	Shutdown(context.Context, *sdkProto.ShutdownRequest) (*sdkProto.ShutdownResponse, error)

	// ExternalEvent are json blobs of gofer's /events endpoint. Normally
	// webhooks.
	ExternalEvent(context.Context, *sdkProto.ExternalEventRequest) (*sdkProto.ExternalEventResponse, error)
}
```
