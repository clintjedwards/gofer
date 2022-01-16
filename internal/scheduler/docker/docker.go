package docker

import (
	"context"
	"encoding/base64"
	"fmt"
	"io"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/docker/docker/api/types"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/api/types/filters"
	"github.com/docker/docker/client"
	"github.com/docker/docker/pkg/stdcopy"
	"github.com/docker/go-connections/nat"
	"github.com/rs/zerolog/log"
)

type Orchestrator struct {
	// cancelled keeps track of cancelled containers. This is needed due to there being no way to differentiate a
	// container that was stopped in docker from a container that exited naturally.
	// When we cancel a container we insert it into this map so that downstream readers of GetState can relay the
	// cancellation to its users.
	//
	// To avoid weird situations in which a container was cancelled, but GetState was never called afterwards(therefore
	// creating a situation in which the cancellation is never removed from the map), we automatically clean up
	// cancellations after they've not been reaped for a day.
	cancelled map[string]time.Time
	*client.Client
}

const envvarFormat = "%s=%s"

func New(prune bool, pruneInterval time.Duration) (Orchestrator, error) {
	docker, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		return Orchestrator{}, nil
	}

	// Check connection to docker
	_, err = docker.Info(context.Background())
	if err != nil {
		return Orchestrator{}, fmt.Errorf("could not connect to docker; is docker installed?")
	}

	// As we run docker containers we might not want to automatically remove them so that its possible for an operator
	// to debug. But we can't leave them lying around due to the fact that each container takes up some amount of space.
	// to mitigate these two things we run ContainerPrune on a loop to make sure we're periodically cleaning up containers
	// after some time.
	if prune {
		go func() {
			for {
				report, err := docker.ContainersPrune(context.Background(), filters.Args{})
				if err != nil {
					log.Debug().Err(err).Msg("docker: could not prune containers")
				}
				log.Debug().Int("containers_deleted", len(report.ContainersDeleted)).
					Uint64("space_reclaimed", report.SpaceReclaimed).Msg("docker: pruned containers")

				time.Sleep(pruneInterval)
			}
		}()
	}

	// Start a goroutine to handle the reaping of cancellations.
	cancelled := map[string]time.Time{}
	go func() {
		for {
			for container, insertTime := range cancelled {
				if insertTime.Before(time.Now().AddDate(0, 0, -1)) {
					delete(cancelled, container)
				}
			}
			time.Sleep(time.Hour * 24)
		}
	}()

	return Orchestrator{
		Client:    docker,
		cancelled: cancelled,
	}, nil
}

func (orch *Orchestrator) StartContainer(req scheduler.StartContainerRequest) (scheduler.StartContainerResponse, error) {
	ctx := context.Background()

	var dockerRegistryAuth string
	if req.RegistryUser != "" {
		authString := fmt.Sprintf("%s:%s", req.RegistryUser, req.RegistryPass)
		dockerRegistryAuth = base64.StdEncoding.EncodeToString([]byte(authString))
	}

	if req.AlwaysPull {
		r, err := orch.ImagePull(ctx, req.ImageName, types.ImagePullOptions{
			RegistryAuth: dockerRegistryAuth,
		})
		if err != nil {
			if strings.Contains(err.Error(), "manifest unknown") {
				return scheduler.StartContainerResponse{}, fmt.Errorf("image '%s' not found or missing auth: %w", req.ImageName, scheduler.ErrNoSuchImage)
			}
			return scheduler.StartContainerResponse{}, err
		}
		_, _ = io.Copy(io.Discard, r)

		defer r.Close() // We don't care about pull logs only the errors
	} else {
		list, _ := orch.ImageList(ctx, types.ImageListOptions{
			Filters: filters.NewArgs(filters.KeyValuePair{
				Key: "reference", Value: req.ImageName,
			}),
		})

		if len(list) == 0 {
			r, err := orch.ImagePull(ctx, req.ImageName, types.ImagePullOptions{})
			if err != nil {
				if strings.Contains(err.Error(), "manifest unknown") {
					return scheduler.StartContainerResponse{}, fmt.Errorf("image '%s' not found or missing auth: %w", req.ImageName, scheduler.ErrNoSuchImage)
				}
				return scheduler.StartContainerResponse{}, err
			}
			_, _ = io.Copy(io.Discard, r) // We wait on the readcloser so that we know when it has finished

			defer r.Close() // We don't care about pull logs only the errors
		}
	}

	envMap := req.EnvVars

	containerConfig := &container.Config{
		Image:        req.ImageName,
		Env:          convertEnvVars(envMap),
		ExposedPorts: nat.PortSet{},
	}

	hostConfig := &container.HostConfig{}

	if req.EnableNetworking {
		port, err := nat.NewPort("tcp", "8080")
		if err != nil {
			return scheduler.StartContainerResponse{}, err
		}
		containerConfig.ExposedPorts = nat.PortSet{port: struct{}{}}

		hostPortMap := nat.PortBinding{
			HostIP:   "127.0.0.1",
			HostPort: "0", // Automatically allocate a port from freely available ephemeral port(32768-61000)
		}

		hostConfig.PortBindings = nat.PortMap{
			"8080/tcp": []nat.PortBinding{
				hostPortMap,
			},
		}
	}

	removeOptions := types.ContainerRemoveOptions{
		RemoveVolumes: true,
		Force:         true,
	}

	_ = orch.ContainerRemove(ctx, req.ID, removeOptions)

	createResp, err := orch.ContainerCreate(ctx, containerConfig, hostConfig, nil, nil, req.ID)
	if err != nil {
		return scheduler.StartContainerResponse{}, err
	}

	err = orch.ContainerStart(ctx, createResp.ID, types.ContainerStartOptions{})
	if err != nil {
		return scheduler.StartContainerResponse{}, err
	}

	containerInfo, err := orch.ContainerInspect(ctx, createResp.ID)
	if err != nil {
		return scheduler.StartContainerResponse{}, err
	}

	if len(containerInfo.NetworkSettings.Ports) == 0 && req.EnableNetworking {
		return scheduler.StartContainerResponse{
			SchedulerID: createResp.ID,
		}, fmt.Errorf("could not start container; check logs for errors")
	}

	rawHostPort := nat.PortBinding{
		HostIP:   "",
		HostPort: "",
	}
	if req.EnableNetworking {
		rawHostPort = containerInfo.NetworkSettings.Ports["8080/tcp"][0]
	}

	return scheduler.StartContainerResponse{
		SchedulerID: createResp.ID,
		URL:         fmt.Sprintf("%s:%s", rawHostPort.HostIP, rawHostPort.HostPort),
	}, nil
}

func (orch *Orchestrator) StopContainer(req scheduler.StopContainerRequest) error {
	ctx := context.Background()

	orch.cancelled[req.SchedulerID] = time.Now()
	err := orch.ContainerStop(ctx, req.SchedulerID, &req.Timeout)
	if err != nil {
		if strings.Contains(err.Error(), "No such container") {
			return scheduler.ErrNoSuchContainer
		}
		return err
	}

	return nil
}

func (orch *Orchestrator) GetState(gs scheduler.GetStateRequest) (scheduler.GetStateResponse, error) {
	containerInfo, err := orch.ContainerInspect(context.Background(), gs.SchedulerID)
	if err != nil {
		if strings.Contains(err.Error(), "No such container") {
			return scheduler.GetStateResponse{
				ExitCode: 0,
				State:    models.ContainerStateUnknown,
			}, scheduler.ErrNoSuchContainer
		}

		return scheduler.GetStateResponse{
			ExitCode: 0,
			State:    models.ContainerStateUnknown,
		}, err
	}

	switch containerInfo.State.Status {
	case "created":
		fallthrough
	case "running":
		return scheduler.GetStateResponse{
			ExitCode: 0,
			State:    models.ContainerStateRunning,
		}, nil
	case "exited":
		_, wasCancelled := orch.cancelled[gs.SchedulerID]
		if wasCancelled {
			return scheduler.GetStateResponse{
				ExitCode: containerInfo.State.ExitCode,
				State:    models.ContainerStateCancelled,
			}, nil
		}
		delete(orch.cancelled, gs.SchedulerID)

		if containerInfo.State.ExitCode == 0 {
			return scheduler.GetStateResponse{
				ExitCode: containerInfo.State.ExitCode,
				State:    models.ContainerStateSuccess,
			}, nil
		}

		return scheduler.GetStateResponse{
			ExitCode: containerInfo.State.ExitCode,
			State:    models.ContainerStateFailed,
		}, nil
	default:
		log.Debug().Str("state", containerInfo.State.Status).Msg("abnormal container state")
		return scheduler.GetStateResponse{
			ExitCode: 0,
			State:    models.ContainerStateUnknown,
		}, nil
	}
}

// GetLogs streams the logs from a docker container to an io.Reader.
//
// To do this we first have to de-muliplex the docker logs as they start in a custom format
// where both stdout and stderr are part of the same stream. The de-multiplexing is done by
// the StdCopy function that docker provides.
//
// Since we need to de-multiplex the stream, but still stream it to the caller, we pass the
// StdCopy function an io.Pipe which simply works a single spaced buffer. For every write
// the caller must read before another write can move forward.
func (orch *Orchestrator) GetLogs(gl scheduler.GetLogsRequest) (io.Reader, error) {
	demuxr, demuxw := io.Pipe()

	out, err := orch.ContainerLogs(context.Background(), gl.SchedulerID, types.ContainerLogsOptions{
		ShowStdout: true,
		ShowStderr: true,
		Follow:     true,
	})
	if err != nil {
		if strings.Contains(err.Error(), "No such container") {
			return nil, scheduler.ErrNoSuchContainer
		}

		return nil, err
	}

	go func() {
		byteCount, err := stdcopy.StdCopy(demuxw, demuxw, out)
		if err != nil {
			log.Error().Err(err).Msg("docker: could not demultiplex/parse log stream")
		}
		demuxw.Close()
		log.Debug().Int64("bytes written", byteCount).Msg("docker: finished demultiplexing")
	}()

	return demuxr, nil
}

func convertEnvVars(envvars map[string]string) []string {
	output := []string{}
	for key, value := range envvars {
		output = append(output, fmt.Sprintf(envvarFormat, key, value))
	}

	return output
}
