---
id: overview
title: Overview
sidebar_position: 1
---

# Scheduler

Gofer runs the containers you reference in the pipeline configuration via a container orchestrator referred to here as a "scheduler".

The vision of Gofer is for you to use whatever scheduler your team is most familiar with.

## Supported Schedulers

The only currently supported scheduler is [local docker](docker/overview).

## How to add new Schedulers?

Schedulers are pluggable! Simply implement a new scheduler by following [the given interface.](https://github.com/clintjedwards/gofer/blob/main/internal/scheduler/scheduler.go#L63)

```go
type Engine interface {
	// StartContainer launches a new container on scheduler. Scheduler should return a unique "schedulerID" to allow
	// the ability to refers specifically to the container on subsequent calls.
	StartContainer(request StartContainerRequest) (response StartContainerResponse, err error)

	// StopContainer attempts to stop a specific container identified by the aforementioned "schedulerID". The scheduler
	// should attempt to gracefully stop the container, unless the timeout is reached.
	StopContainer(request StopContainerRequest) error

	// GetState returns the current state of the container translated to the "models.ContainerState" enum.
	GetState(request GetStateRequest) (response GetStateResponse, err error)

	// GetLogs reads logs from the container and passes it back to the caller via an io.Reader. This io.reader can
	// be written to from a goroutine so that they user gets logs as they are streamed from the container.
	// Finally once finished the io.reader should be close with an EOF denoting that there are no more logs to be read.
	GetLogs(request GetLogsRequest) (logs io.Reader, err error)
}
```
