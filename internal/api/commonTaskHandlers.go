package api

import (
	"bufio"
	"context"
	"errors"
	"strings"

	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetCommonTask(ctx context.Context, request *proto.GetCommonTaskRequest) (*proto.GetCommonTaskResponse, error) {
	if request.Name == "" {
		return &proto.GetCommonTaskResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	commonTask, exists := api.commonTasks.Get(request.Name)
	if !exists {
		return &proto.GetCommonTaskResponse{}, status.Error(codes.NotFound, "could not find common task")
	}

	return &proto.GetCommonTaskResponse{CommonTask: commonTask.ToProto()}, nil
}

func (api *API) ListCommonTasks(ctx context.Context, request *proto.ListCommonTasksRequest) (*proto.ListCommonTasksResponse, error) {
	protoCommonTasks := []*proto.CommonTask{}
	for _, commonTaskKey := range api.commonTasks.Keys() {
		commonTask, exists := api.commonTasks.Get(commonTaskKey)
		if !exists {
			continue
		}
		protoCommonTasks = append(protoCommonTasks, commonTask.ToProto())
	}

	return &proto.ListCommonTasksResponse{
		CommonTasks: protoCommonTasks,
	}, nil
}

func (api *API) GetCommonTaskInstallInstructions(ctx context.Context, request *proto.GetCommonTaskInstallInstructionsRequest) (*proto.GetCommonTaskInstallInstructionsResponse, error) {
	var registryAuth *models.RegistryAuth = nil
	if request.User != "" {
		registryAuth = &models.RegistryAuth{
			User: request.User,
			Pass: request.Pass,
		}
	}

	sc := scheduler.StartContainerRequest{
		ID:               installerContainerID(),
		ImageName:        request.Image,
		EnvVars:          map[string]string{},
		RegistryAuth:     registryAuth,
		AlwaysPull:       true,
		EnableNetworking: false,
		Entrypoint:       []string{"./commontask", "installer"},
	}

	resp, err := api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not start common task during installation instructions retrieval")
		return &proto.GetCommonTaskInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not start common task; %v", err)
	}

	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{SchedulerID: resp.SchedulerID})
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not get logs from common task installation run")
		return &proto.GetCommonTaskInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not get logs from common task installation run; %v", err)
	}

	lastLine := ""

	scanner := bufio.NewScanner(logReader)
	for scanner.Scan() {
		lastLine = scanner.Text()
	}
	err = scanner.Err()
	if err != nil {
		log.Error().Err(err).Msg("Could not properly read from logging stream")
		return &proto.GetCommonTaskInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not get logs from common task installation run; %v", err)
	}

	return &proto.GetCommonTaskInstallInstructionsResponse{
		Instructions: strings.TrimSpace(lastLine),
	}, nil
}

func (api *API) InstallCommonTask(ctx context.Context, request *proto.InstallCommonTaskRequest) (*proto.InstallCommonTaskResponse, error) {
	if request.Name == "" {
		return &proto.InstallCommonTaskResponse{}, status.Errorf(codes.FailedPrecondition, "name required")
	}

	if request.Image == "" {
		return &proto.InstallCommonTaskResponse{}, status.Errorf(codes.FailedPrecondition, "image required")
	}

	registration := models.CommonTaskRegistration{}
	registration.FromInstallCommonTaskRequest(request)

	err := api.db.InsertCommonTaskRegistration(&registration)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.InstallCommonTaskResponse{}, status.Errorf(codes.AlreadyExists, "common task is %s already installed", request.Name)
		}

		return &proto.InstallCommonTaskResponse{}, status.Errorf(codes.Internal, "common task could not be installed; %v", err)
	}

	api.commonTasks.Set(request.Name, &models.CommonTask{
		Name:          request.Name,
		Image:         request.Image,
		RegistryAuth:  registration.RegistryAuth,
		Variables:     registration.Variables,
		Documentation: &request.Documentation,
	})

	go api.events.Publish(models.EventInstalledCommonTask{
		Name:  request.Name,
		Image: request.Image,
	})

	return &proto.InstallCommonTaskResponse{}, nil
}

func (api *API) UninstallCommonTask(ctx context.Context, request *proto.UninstallCommonTaskRequest) (*proto.UninstallCommonTaskResponse, error) {
	api.commonTasks.Delete(request.Name)

	err := api.db.DeleteCommonTaskRegistration(request.Name)
	if err != nil {
		log.Error().Err(err).Msg("could not delete common task registration")
		return &proto.UninstallCommonTaskResponse{}, status.Errorf(codes.Internal, "could not delete common task registration: %v", err)
	}

	go api.events.Publish(models.EventUninstalledCommonTask{
		Name: request.Name,
	})

	// TODO(clintjedwards): We should alert all users that previously had registrations that they need to fix their pipeline.

	return &proto.UninstallCommonTaskResponse{}, nil
}

func (api *API) EnableCommonTask(ctx context.Context, request *proto.EnableCommonTaskRequest) (*proto.EnableCommonTaskResponse, error) {
	err := api.db.UpdateCommonTaskRegistration(request.Name, storage.UpdatableCommonTaskRegistrationFields{
		Status: ptr(models.CommonTaskStatusEnabled),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnableCommonTaskResponse{}, status.Errorf(codes.NotFound, "common task %q is not found", request.Name)
		}

		return &proto.EnableCommonTaskResponse{}, status.Errorf(codes.Internal, "common task could not be installed; %v", err)
	}

	// TODO(clintjedwards): This needs a get and swap
	task, exists := api.commonTasks.Get(request.Name)
	if !exists {
		_ = api.db.UpdateCommonTaskRegistration(request.Name, storage.UpdatableCommonTaskRegistrationFields{
			Status: ptr(models.CommonTaskStatusDisabled),
		})
		return &proto.EnableCommonTaskResponse{}, status.Errorf(codes.NotFound, "common task %q is not found", request.Name)
	}

	task.Status = models.CommonTaskStatusEnabled
	api.commonTasks.Set(request.Name, task)

	return &proto.EnableCommonTaskResponse{}, nil
}

func (api *API) DisableCommonTask(ctx context.Context, request *proto.DisableCommonTaskRequest) (*proto.DisableCommonTaskResponse, error) {
	err := api.db.UpdateCommonTaskRegistration(request.Name, storage.UpdatableCommonTaskRegistrationFields{
		Status: ptr(models.CommonTaskStatusDisabled),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisableCommonTaskResponse{}, status.Errorf(codes.NotFound, "common task %q is not found", request.Name)
		}

		return &proto.DisableCommonTaskResponse{}, status.Errorf(codes.Internal, "common task could not be installed; %v", err)
	}

	// TODO(clintjedwards): This needs a get and swap
	task, exists := api.commonTasks.Get(request.Name)
	if !exists {
		_ = api.db.UpdateCommonTaskRegistration(request.Name, storage.UpdatableCommonTaskRegistrationFields{
			Status: ptr(models.CommonTaskStatusDisabled),
		})
		return &proto.DisableCommonTaskResponse{}, status.Errorf(codes.NotFound, "common task %q is not found", request.Name)
	}

	task.Status = models.CommonTaskStatusDisabled
	api.commonTasks.Set(request.Name, task)

	return &proto.DisableCommonTaskResponse{}, nil
}
