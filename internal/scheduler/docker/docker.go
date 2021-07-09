package docker

import (
	"bufio"
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"io"
	"os"
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
	cancelled   map[string]time.Time
	secretsPath string // Path to local file containing docker secrets.
	*client.Client
}

const envvarFormat = "%s=%s"

func New(prune bool, pruneInterval time.Duration, secretsPath string) (Orchestrator, error) {
	docker, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		return Orchestrator{}, nil
	}

	// Check connection to docker
	_, err = docker.Info(context.Background())
	if err != nil {
		return Orchestrator{}, fmt.Errorf("could not connect to docker; is docker installed?")
	}

	// Check existence of secrets file
	if secretsPath != "" {
		if _, err := os.Stat(secretsPath); errors.Is(err, os.ErrNotExist) {
			return Orchestrator{}, fmt.Errorf("could not open secrets file")
		}
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
		Client:      docker,
		cancelled:   cancelled,
		secretsPath: secretsPath,
	}, nil
}

// readSecretsFile reads in the secrets from specified file and stores them in a map so they can be mapped to envVars
// in the container.
func (orch *Orchestrator) readSecretsFile() (map[string]string, error) {
	secrets := map[string]string{}

	file, err := os.Open(orch.secretsPath)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		lineSplit := strings.Split(line, "=")
		if len(lineSplit) != 2 {
			continue
		}

		secrets[lineSplit[0]] = lineSplit[1]
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	return secrets, nil
}

// populateSecrets reads in requested secrets from secrets file. For Docker specifically all that is needed to
// retrieve secrets is that they are in the same form as the environment variable they are tied to.
// So in Gofer if for the secret key, placing it as "SECRET_LOGS_HEADER" would automatically search for the same
// string in the secrets file.
// We leave the secret empty if not found so that the user has the ability to verify the secrets exists by requiring it.
func populateSecrets(requested, stored map[string]string) map[string]string {
	secrets := map[string]string{}
	for key := range requested {
		secret, exists := stored[key]
		if !exists {
			secrets[key] = ""
			continue
		}

		secrets[key] = secret
	}

	return secrets
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

	if orch.secretsPath == "" && len(req.Secrets) > 0 {
		return scheduler.StartContainerResponse{}, fmt.Errorf("secrets requested, but docker scheduler secretsPath is not set;" +
			" secretsPath config must be set to activate docker secrets.")
	}

	if len(req.Secrets) > 0 && orch.secretsPath != "" {
		storedSecrets, err := orch.readSecretsFile()
		if err != nil {
			return scheduler.StartContainerResponse{}, err
		}

		secretMap := populateSecrets(req.Secrets, storedSecrets)

		// We combine regular envs and secrets because they're treated in the same way once they get to the docker
		// container level. The order is important here. We do secrets first because if the user made a mistake
		// and there is collision we don't want them treating a regular var like a secret.
		envMap = mergeMaps(secretMap, req.EnvVars)
	}

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

// mergeMaps combines many string maps in a "last one in wins" format.
func mergeMaps(maps ...map[string]string) map[string]string {
	newMap := map[string]string{}

	for _, extraMap := range maps {
		for key, value := range extraMap {
			newMap[key] = value
		}
	}

	return newMap
}
