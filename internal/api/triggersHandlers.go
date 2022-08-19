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

func (api *API) GetTrigger(ctx context.Context, request *proto.GetTriggerRequest) (*proto.GetTriggerResponse, error) {
	if request.Name == "" {
		return &proto.GetTriggerResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	trigger, exists := api.triggers.Get(request.Name)
	if !exists {
		return &proto.GetTriggerResponse{}, status.Error(codes.NotFound, "could not find trigger")
	}

	return &proto.GetTriggerResponse{Trigger: trigger.ToProto()}, nil
}

func (api *API) ListTriggers(ctx context.Context, request *proto.ListTriggersRequest) (*proto.ListTriggersResponse, error) {
	protoTriggers := []*proto.Trigger{}
	for _, triggerKey := range api.triggers.Keys() {
		trigger, exists := api.triggers.Get(triggerKey)
		if !exists {
			continue
		}
		protoTriggers = append(protoTriggers, trigger.ToProto())
	}

	return &proto.ListTriggersResponse{
		Triggers: protoTriggers,
	}, nil
}

func (api *API) GetTriggerInstallInstructions(ctx context.Context, request *proto.GetTriggerInstallInstructionsRequest) (*proto.GetTriggerInstallInstructionsResponse, error) {
	cert, key, err := api.getTLSFromFile(api.config.Triggers.TLSCertPath, api.config.Triggers.TLSKeyPath)
	if err != nil {
		return &proto.GetTriggerInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not obtain proper TLS for trigger certifications; %v", err)
	}

	triggerKey := generateToken(32)

	// We need to first populate the triggers with their required environment variables.
	// Order is important here maps later in the list will overwrite earlier maps.
	// We first include the Gofer defined environment variables and then the operator configured environment
	// variables.
	systemTriggerVars := []models.Variable{
		{
			Key:    "GOFER_TRIGGER_TLS_CERT",
			Value:  string(cert),
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_TLS_KEY",
			Value:  string(key),
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_NAME",
			Value:  "Installer",
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_LOG_LEVEL",
			Value:  api.config.LogLevel,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_TRIGGER_KEY",
			Value:  triggerKey,
			Source: models.VariableSourceSystem,
		},
	}

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
		EnvVars:          convertVarsToMap(systemTriggerVars),
		RegistryAuth:     registryAuth,
		AlwaysPull:       true,
		EnableNetworking: false,
		Entrypoint:       &[]string{"./trigger", "installer"},
	}

	resp, err := api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not start trigger during installation instructions retrieval")
		return &proto.GetTriggerInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not start trigger; %v", err)
	}

	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{SchedulerID: resp.SchedulerID})
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not get logs from trigger installation run")
		return &proto.GetTriggerInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not get logs from trigger installation run; %v", err)
	}

	lastLine := ""

	scanner := bufio.NewScanner(logReader)
	for scanner.Scan() {
		lastLine = scanner.Text()
	}
	err = scanner.Err()
	if err != nil {
		log.Error().Err(err).Msg("Could not properly read from logging stream")
		return &proto.GetTriggerInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not get logs from trigger installation run; %v", err)
	}

	return &proto.GetTriggerInstallInstructionsResponse{
		Instructions: strings.TrimSpace(lastLine),
	}, nil
}

func (api *API) InstallTrigger(ctx context.Context, request *proto.InstallTriggerRequest) (*proto.InstallTriggerResponse, error) {
	if request.Name == "" {
		return &proto.InstallTriggerResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	if request.Image == "" {
		return &proto.InstallTriggerResponse{}, status.Error(codes.FailedPrecondition, "image required")
	}

	registration := models.TriggerRegistration{}
	registration.FromInstallTriggerRequest(request)

	cert, key, err := api.getTLSFromFile(api.config.Triggers.TLSCertPath, api.config.Triggers.TLSKeyPath)
	if err != nil {
		return &proto.InstallTriggerResponse{}, status.Errorf(codes.Internal, "could not obtain proper TLS for trigger certifications; %v", err)
	}

	err = api.startTrigger(registration, string(cert), string(key))
	if err != nil {
		return &proto.InstallTriggerResponse{}, status.Errorf(codes.Internal, "could not start trigger; %v", err)
	}

	err = api.db.InsertTriggerRegistration(&registration)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.InstallTriggerResponse{}, status.Errorf(codes.AlreadyExists, "trigger is %s already installed", request.Name)
		}

		return &proto.InstallTriggerResponse{}, status.Errorf(codes.Internal, "trigger could not be installed; %v", err)
	}

	go api.events.Publish(models.EventInstalledTrigger{
		Name:  request.Name,
		Image: request.Image,
	})

	return &proto.InstallTriggerResponse{}, nil
}

func (api *API) UninstallTrigger(ctx context.Context, request *proto.UninstallTriggerRequest) (*proto.UninstallTriggerResponse, error) {
	api.triggers.Delete(request.Name)

	err := api.db.DeleteTriggerRegistration(request.Name)
	if err != nil {
		return &proto.UninstallTriggerResponse{}, status.Error(codes.Internal, "error deleting trigger registration")
	}

	go api.events.Publish(models.EventUninstalledTrigger{
		Name: request.Name,
	})

	// TODO(clintjedwards): We should alert all users that previously had registrations that they need to fix their pipeline.

	return &proto.UninstallTriggerResponse{}, nil
}

func (api *API) EnableTrigger(ctx context.Context, request *proto.EnableTriggerRequest) (*proto.EnableTriggerResponse, error) {
	err := api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
		Status: ptr(models.TriggerStatusEnabled),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
		}

		return &proto.EnableTriggerResponse{}, status.Errorf(codes.Internal, "trigger could not be installed; %v", err)
	}

	// TODO(clintjedwards): This needs a get and swap
	trigger, exists := api.triggers.Get(request.Name)
	if !exists {
		_ = api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
			Status: ptr(models.TriggerStatusDisabled),
		})
		return &proto.EnableTriggerResponse{}, status.Errorf(codes.FailedPrecondition, "trigger %q is not found", request.Name)
	}

	trigger.Registration.Status = models.TriggerStatusEnabled
	api.triggers.Set(request.Name, trigger)

	return &proto.EnableTriggerResponse{}, nil
}

func (api *API) DisableTrigger(ctx context.Context, request *proto.DisableTriggerRequest) (*proto.DisableTriggerResponse, error) {
	err := api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
		Status: ptr(models.TriggerStatusDisabled),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
		}

		return &proto.DisableTriggerResponse{}, status.Errorf(codes.Internal, "trigger could not be installed; %v", err)
	}

	// TODO(clintjedwards): This needs a get and swap
	task, exists := api.triggers.Get(request.Name)
	if !exists {
		_ = api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
			Status: ptr(models.TriggerStatusDisabled),
		})
		return &proto.DisableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
	}

	task.Registration.Status = models.TriggerStatusDisabled
	api.triggers.Set(request.Name, task)

	return &proto.DisableTriggerResponse{}, nil
}
