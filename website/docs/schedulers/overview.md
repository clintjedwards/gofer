---
id: overview
title: Overview
sidebar_position: 1
---

# Scheduler

Gofer runs the containers you reference in the pipeline configuration via a container orchestrator referred to here as a "scheduler".

The vision of Gofer is for you to use whatever scheduler your team is most familiar with.

## Secrets

The way Gofer passes secret values to containers is that..._it doesn't_. This task is much better suited for the scheduler and as such Gofer relies on the user to set up their secrets such that it is available from the scheduler beforehand.

Once the secret is made available the user can simply add the appropriate location/name in the [secret](../pipeline-configuration/task/task-stanza#task-parameters) field for the relevant task.

Here is an example given with the assumption that [Nomad with Vault](https://www.nomadproject.io/docs/integrations/vault-integration) has been set up as the container orchestrator of choice and the container of choice is a test container that simply prints the secret value.

1. The test container is set up such that it accepts the secret value through an environment variable, in this case `TEST_SECRET`.
2. The secret desired is registered into Vault under the path `secrets/my-pipeline/TEST_SECRET`.
3. In the pipeline configuration for the test container. The task associated with test container is given the [secret stanza](../pipeline-configuration/task/task-stanza#task-parameters) key/value: `"TEST_SECRET": "secrets/my-pipeline/TEST_SECRET"`.
4. When this container is run, Gofer will pass Nomad the environment variable key and the location value mentioned in the secret stanza.
5. Nomad pulls the relevant secret from Vault before container execution and inserts it into the environment variable `TEST_SECRET`.

## Supported Schedulers

The only currently supported scheduler is [local docker](docker/overview).

## How to add new Schedulers?

Schedulers are pluggable! Simply implement a new scheduler by following [the given interface.](https://github.com/clintjedwards/gofer/blob/053ad33e30e9fdf21a005fffbe9ad849fe258ec1/internal/scheduler/scheduler.go#L63)

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
