package api

import (
	"bufio"
	"context"
	"errors"
	"fmt"
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
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

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
			Key:    "GOFER_PLUGIN_SYSTEM_TLS_CERT",
			Value:  string(cert),
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_TLS_KEY",
			Value:  string(key),
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_NAME",
			Value:  "Installer",
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_LOG_LEVEL",
			Value:  api.config.LogLevel,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_PLUGIN_SYSTEM_KEY",
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

	_, err = api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not start trigger during installation instructions retrieval")
		return &proto.GetTriggerInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not start trigger; %v", err)
	}

	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{ID: installerContainerID()})
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
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

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
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return nil, status.Error(codes.FailedPrecondition, "name required")
	}

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
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return nil, status.Error(codes.FailedPrecondition, "name required")
	}

	err := api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
		Status: ptr(models.TriggerStatusEnabled),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
		}

		return &proto.EnableTriggerResponse{}, status.Errorf(codes.Internal, "trigger could not be installed; %v", err)
	}

	err = api.triggers.Swap(request.Name, func(value *models.Trigger, exists bool) (*models.Trigger, error) {
		if !exists {
			_ = api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
				Status: ptr(models.TriggerStatusDisabled),
			})

			return nil, fmt.Errorf("trigger %q not found", request.Name)
		}

		value.Registration.Status = models.TriggerStatusEnabled
		return value, nil
	})
	if err != nil {
		return &proto.EnableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
	}

	return &proto.EnableTriggerResponse{}, nil
}

func (api *API) DisableTrigger(ctx context.Context, request *proto.DisableTriggerRequest) (*proto.DisableTriggerResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return nil, status.Error(codes.FailedPrecondition, "name required")
	}

	err := api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
		Status: ptr(models.TriggerStatusDisabled),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
		}

		return &proto.DisableTriggerResponse{}, status.Errorf(codes.Internal, "trigger could not be installed; %v", err)
	}

	err = api.triggers.Swap(request.Name, func(value *models.Trigger, exists bool) (*models.Trigger, error) {
		if !exists {
			_ = api.db.UpdateTriggerRegistration(request.Name, storage.UpdatableTriggerRegistrationFields{
				Status: ptr(models.TriggerStatusEnabled),
			})

			return nil, fmt.Errorf("trigger %q not found", request.Name)
		}

		value.Registration.Status = models.TriggerStatusDisabled
		return value, nil
	})
	if err != nil {
		return &proto.DisableTriggerResponse{}, status.Errorf(codes.NotFound, "trigger %q is not found", request.Name)
	}

	return &proto.DisableTriggerResponse{}, nil
}
