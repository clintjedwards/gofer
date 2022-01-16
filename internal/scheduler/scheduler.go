// Package scheduler defines the interface in which a scheduler must adhere to. A scheduler is the mechanism in which
// gofer uses to schedule taskruns/containers.
package scheduler

import (
	"errors"
	"io"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
)

type EngineType string

const (
	// EngineDocker uses local docker instance to schedule tasks.
	EngineDocker EngineType = "docker"
)

// ErrNoSuchContainer is returned when a container requested could not be located on the scheduler.
var ErrNoSuchContainer = errors.New("scheduler: entity not found")

// ErrNoSuchImage is returned when the requested container image could not be pulled.
var ErrNoSuchImage = errors.New("scheduler: docker image not found")

type StartContainerRequest struct {
	ID        string            // The schedulerID of the container
	ImageName string            // The docker image repository endpoint of the container; tag can be included.
	EnvVars   map[string]string // Environment variables to be passed to the container

	RegistryUser string // Username for auth registry
	RegistryPass string // Password for auth registry

	// Even if the container exists attempt to pull from repository. This is useful if your containers
	// don't use proper tagging or versioning.
	AlwaysPull bool

	// Networking is used to communicate to the container via RPC. This is only needed by triggers.
	EnableNetworking bool
}

type StartContainerResponse struct {
	SchedulerID string // a unique way to identify the container that has started.
	URL         string // optional endpoint if "EnableNetworking" was used.
}

type StopContainerRequest struct {
	SchedulerID string        // unique identification for container to stop.
	Timeout     time.Duration // The total time the scheduler should wait for a graceful stop before issueing a SIGKILL.
}

type GetStateRequest struct {
	SchedulerID string // unique identification for container to retrive.
}

type GetStateResponse struct {
	ExitCode int
	State    models.ContainerState
}

type GetLogsRequest struct {
	SchedulerID string
}

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
