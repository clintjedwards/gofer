package api

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetExtension(ctx context.Context, request *proto.GetExtensionRequest) (*proto.GetExtensionResponse, error) {
	if request.Name == "" {
		return &proto.GetExtensionResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	extension, exists := api.extensions.Get(request.Name)
	if !exists {
		return &proto.GetExtensionResponse{}, status.Error(codes.NotFound, "could not find extension")
	}

	return &proto.GetExtensionResponse{Extension: extension.ToProto()}, nil
}

func (api *API) ListExtensions(ctx context.Context, request *proto.ListExtensionsRequest) (*proto.ListExtensionsResponse, error) {
	protoExtensions := []*proto.Extension{}
	for _, extensionKey := range api.extensions.Keys() {
		extension, exists := api.extensions.Get(extensionKey)
		if !exists {
			continue
		}
		protoExtensions = append(protoExtensions, extension.ToProto())
	}

	return &proto.ListExtensionsResponse{
		Extensions: protoExtensions,
	}, nil
}

func (api *API) GetExtensionInstallInstructions(ctx context.Context, request *proto.GetExtensionInstallInstructionsRequest) (*proto.GetExtensionInstallInstructionsResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	cert, key, err := api.getTLSFromFile(api.config.Extensions.TLSCertPath, api.config.Extensions.TLSKeyPath)
	if err != nil {
		return &proto.GetExtensionInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not obtain proper TLS for extension certifications; %v", err)
	}

	// Temporary key since we don't need to continually talk to the container.
	extensionKey := generateToken(32)

	// We need to first populate the extensions with their required environment variables.
	// Order is important here maps later in the list will overwrite earlier maps.
	// We first include the Gofer defined environment variables and then the operator configured environment
	// variables.
	systemExtensionVars := []models.Variable{
		{
			Key:    "GOFER_EXTENSION_SYSTEM_TLS_CERT",
			Value:  string(cert),
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_TLS_KEY",
			Value:  string(key),
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_NAME",
			Value:  "Installer",
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_LOG_LEVEL",
			Value:  api.config.LogLevel,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_KEY",
			Value:  extensionKey,
			Source: models.VariableSourceSystem,
		},
	}

	var registryAuth *models.RegistryAuth
	if request.User != "" {
		registryAuth = &models.RegistryAuth{
			User: request.User,
			Pass: request.Pass,
		}
	}

	containerID := installerContainerID()

	sc := scheduler.StartContainerRequest{
		ID:           containerID,
		ImageName:    request.Image,
		EnvVars:      convertVarsToMap(systemExtensionVars),
		RegistryAuth: registryAuth,
		AlwaysPull:   true,
		Networking:   nil,
		Entrypoint:   &[]string{"./extension", "installer"},
	}

	_, err = api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not start extension during installation instructions retrieval")
		return &proto.GetExtensionInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not start extension; %v", err)
	}

	logReader, err := api.scheduler.GetLogs(scheduler.GetLogsRequest{ID: containerID})
	if err != nil {
		log.Error().Err(err).Str("image", request.Image).Msg("could not get logs from extension installation run")
		return &proto.GetExtensionInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not get logs from extension installation run; %v", err)
	}

	lastLine := ""

	scanner := bufio.NewScanner(logReader)
	for scanner.Scan() {
		lastLine = scanner.Text()
	}
	err = scanner.Err()
	if err != nil {
		log.Error().Err(err).Msg("Could not properly read from logging stream")
		return &proto.GetExtensionInstallInstructionsResponse{},
			status.Errorf(codes.Internal, "could not get logs from extension installation run; %v", err)
	}

	return &proto.GetExtensionInstallInstructionsResponse{
		Instructions: strings.TrimSpace(lastLine),
	}, nil
}

func (api *API) InstallExtension(ctx context.Context, request *proto.InstallExtensionRequest) (*proto.InstallExtensionResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return &proto.InstallExtensionResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	if request.Image == "" {
		return &proto.InstallExtensionResponse{}, status.Error(codes.FailedPrecondition, "image required")
	}

	registration := models.ExtensionRegistration{}
	registration.FromInstallExtensionRequest(request)

	cert, key, err := api.getTLSFromFile(api.config.Extensions.TLSCertPath, api.config.Extensions.TLSKeyPath)
	if err != nil {
		return &proto.InstallExtensionResponse{}, status.Errorf(codes.Internal, "could not obtain proper TLS for extension certifications; %v", err)
	}

	err = api.startExtension(registration, string(cert), string(key))
	if err != nil {
		return &proto.InstallExtensionResponse{}, status.Errorf(codes.Internal, "could not start extension; %v", err)
	}

	err = api.db.InsertGlobalExtensionRegistration(api.db, registration.ToStorage())
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.InstallExtensionResponse{}, status.Errorf(codes.AlreadyExists, "extension is %s already installed", request.Name)
		}

		return &proto.InstallExtensionResponse{}, status.Errorf(codes.Internal, "extension could not be installed; %v", err)
	}

	go api.events.Publish(events.EventExtensionInstalled{
		Name:  request.Name,
		Image: request.Image,
	})

	return &proto.InstallExtensionResponse{}, nil
}

func (api *API) UninstallExtension(ctx context.Context, request *proto.UninstallExtensionRequest) (*proto.UninstallExtensionResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return nil, status.Error(codes.FailedPrecondition, "name required")
	}

	api.extensions.Delete(request.Name)

	err := api.db.DeleteGlobalExtensionRegistration(api.db, request.Name)
	if err != nil {
		return &proto.UninstallExtensionResponse{}, status.Error(codes.Internal, "error deleting extension registration")
	}

	go api.events.Publish(events.EventExtensionUninstalled{
		Name: request.Name,
	})

	return &proto.UninstallExtensionResponse{}, nil
}

func (api *API) EnableExtension(ctx context.Context, request *proto.EnableExtensionRequest) (*proto.EnableExtensionResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return nil, status.Error(codes.FailedPrecondition, "name required")
	}

	err := api.db.UpdateGlobalExtensionRegistration(api.db, request.Name, storage.UpdatableGlobalExtensionRegistrationFields{
		Status: ptr(string(models.ExtensionStatusEnabled)),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnableExtensionResponse{}, status.Errorf(codes.NotFound, "extension %q is not found", request.Name)
		}

		return &proto.EnableExtensionResponse{}, status.Errorf(codes.Internal, "extension could not be installed; %v", err)
	}

	err = api.extensions.Swap(request.Name, func(value *models.Extension, exists bool) (*models.Extension, error) {
		if !exists {
			_ = api.db.UpdateGlobalExtensionRegistration(api.db, request.Name, storage.UpdatableGlobalExtensionRegistrationFields{
				Status: ptr(string(models.ExtensionStatusDisabled)),
			})

			return nil, fmt.Errorf("extension %q not found", request.Name)
		}

		value.Registration.Status = models.ExtensionStatusEnabled
		return value, nil
	})
	if err != nil {
		return &proto.EnableExtensionResponse{}, status.Errorf(codes.NotFound, "extension %q is not found", request.Name)
	}

	return &proto.EnableExtensionResponse{}, nil
}

func (api *API) DisableExtension(ctx context.Context, request *proto.DisableExtensionRequest) (*proto.DisableExtensionResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Name == "" {
		return nil, status.Error(codes.FailedPrecondition, "name required")
	}

	err := api.db.UpdateGlobalExtensionRegistration(api.db, request.Name, storage.UpdatableGlobalExtensionRegistrationFields{
		Status: ptr(string(models.ExtensionStatusDisabled)),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisableExtensionResponse{}, status.Errorf(codes.NotFound, "extension %q is not found", request.Name)
		}

		return &proto.DisableExtensionResponse{}, status.Errorf(codes.Internal, "extension could not be installed; %v", err)
	}

	err = api.extensions.Swap(request.Name, func(value *models.Extension, exists bool) (*models.Extension, error) {
		if !exists {
			_ = api.db.UpdateGlobalExtensionRegistration(api.db, request.Name, storage.UpdatableGlobalExtensionRegistrationFields{
				Status: ptr(string(models.ExtensionStatusEnabled)),
			})

			return nil, fmt.Errorf("extension %q not found", request.Name)
		}

		value.Registration.Status = models.ExtensionStatusDisabled
		return value, nil
	})
	if err != nil {
		return &proto.DisableExtensionResponse{}, status.Errorf(codes.NotFound, "extension %q is not found", request.Name)
	}

	return &proto.DisableExtensionResponse{}, nil
}
