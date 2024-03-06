package api

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/scheduler"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	grpc_retry "github.com/grpc-ecosystem/go-grpc-middleware/retry"
	"github.com/rs/zerolog/log"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
)

func (api *API) GetExtension(_ context.Context, request *proto.GetExtensionRequest) (*proto.GetExtensionResponse, error) {
	if request.Name == "" {
		return &proto.GetExtensionResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	extension, exists := api.extensions.Get(request.Name)
	if !exists {
		return &proto.GetExtensionResponse{}, status.Error(codes.NotFound, "could not find extension")
	}

	return &proto.GetExtensionResponse{Extension: extension.ToProto()}, nil
}

func (api *API) ListExtensions(_ context.Context, _ *proto.ListExtensionsRequest) (*proto.ListExtensionsResponse, error) {
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

func (api *API) RunExtensionInstaller(stream proto.Gofer_RunExtensionInstallerServer) error {
	if !isManagementUser(stream.Context()) {
		return status.Error(codes.PermissionDenied, "management token required for this action")
	}

	// Get the first message so we can attempt to set up the connection with the proper docker container.
	initMessageRaw, err := stream.Recv()
	if err != nil {
		log.Error().Err(err).Msg("could not set up stream")
		return status.Errorf(codes.Internal, "could not set up stream: %v", err)
	}

	initMessage, ok := initMessageRaw.MessageType.(*proto.RunExtensionInstallerClientMessage_Init_)
	if !ok {
		return status.Error(codes.FailedPrecondition, "first message must be init message, received wrong message type")
	}

	// Validate input
	if initMessage.Init.Image == "" {
		return status.Error(codes.FailedPrecondition, "extension image required")
	}

	var registryAuth *models.RegistryAuth
	if initMessage.Init.User != "" {
		registryAuth = &models.RegistryAuth{
			User: initMessage.Init.User,
			Pass: initMessage.Init.Pass,
		}
	}

	cert, key, err := api.getTLSFromFile(api.config.Extensions.TLSCertPath, api.config.Extensions.TLSKeyPath)
	if err != nil {
		return status.Errorf(codes.Internal, "could not obtain proper TLS for extension certifications; %v", err)
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
			// The system_name is simply a human readable name for the extension.
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
			// The system_key is a random string passed in by Gofer on the extensions init to act as a pre-shared
			// auth key between the two systems. When gofer makes a request to the extension, the extension verifies
			// that is has the correct auth and vice-versa.
			Key:    "GOFER_EXTENSION_SYSTEM_KEY",
			Value:  extensionKey,
			Source: models.VariableSourceSystem,
		},
		{
			// The gofer_host is the url of where the main gofer server. This is used by the extension to simply
			// communicate back to the original gofer host.
			Key:    "GOFER_EXTENSION_SYSTEM_GOFER_HOST",
			Value:  api.config.Server.Address,
			Source: models.VariableSourceSystem,
		},
		{
			Key:    "GOFER_EXTENSION_SYSTEM_HOST",
			Value:  "0.0.0.0:8082",
			Source: models.VariableSourceSystem,
		},
	}

	containerID := installerContainerID()

	sc := scheduler.StartContainerRequest{
		ID:           containerID,
		ImageName:    initMessage.Init.Image,
		EnvVars:      convertVarsToMap(systemExtensionVars),
		RegistryAuth: registryAuth,
		AlwaysPull:   true,
		Networking: &scheduler.Networking{
			Port: 8082,
		},
	}

	resp, err := api.scheduler.StartContainer(sc)
	if err != nil {
		log.Error().Err(err).Str("image", initMessage.Init.Image).
			Msg("could not start extension during running extension installer")
		return status.Errorf(codes.Internal, "could not start extension; %v", err)
	}
	defer api.scheduler.StopContainer(scheduler.StopContainerRequest{
		ID: containerID,
	})

	conn, err := grpcDial(resp.URL)
	if err != nil {
		log.Debug().Err(err).Msg("could not connect to extension for handling")
		return err
	}
	defer conn.Close()

	client := proto.NewExtensionServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(extensionKey))

	_, err = client.Info(ctx, &proto.ExtensionInfoRequest{}, grpc_retry.WithMax(30))
	if err != nil {
		if status.Code(err) == codes.Canceled {
			return nil
		}

		log.Error().Err(err).Str("image", initMessage.Init.Image).Msg("failed to communicate with extension info phase")
		return err
	}

	extensionConn, err := client.RunExtensionInstaller(ctx)
	if err != nil {
		return status.Errorf(codes.Internal, "could not connect to extension while running extension installer; %v", err)
	}

	// Simply copy the messages from the extension and client together.
	for {
		extensionMsgRaw, err := extensionConn.Recv()
		if err != nil {
			// If the context was cancelled, that means that the extension is done and we should process the installation.
			if strings.Contains(err.Error(), "context canceled") {
				return nil
			}

			// If the client disconnected, exit cleanly.
			if strings.Contains(err.Error(), "client disconnected") {
				return nil
			}

			return status.Errorf(codes.Internal, "could not receive message from extension during extension installation; %v", err)
		}

		switch extensionMsg := extensionMsgRaw.MessageType.(type) {
		case *proto.ExtensionRunExtensionInstallerExtensionMessage_ConfigSetting_:
			_ = stream.Send(&proto.RunExtensionInstallerExtensionMessage{
				MessageType: &proto.RunExtensionInstallerExtensionMessage_ConfigSetting_{
					ConfigSetting: &proto.RunExtensionInstallerExtensionMessage_ConfigSetting{
						Config: extensionMsg.ConfigSetting.Config,
						Value:  extensionMsg.ConfigSetting.Value,
					},
				},
			})

		case *proto.ExtensionRunExtensionInstallerExtensionMessage_Msg:
			_ = stream.Send(&proto.RunExtensionInstallerExtensionMessage{
				MessageType: &proto.RunExtensionInstallerExtensionMessage_Msg{
					Msg: extensionMsg.Msg,
				},
			})

		case *proto.ExtensionRunExtensionInstallerExtensionMessage_Query:
			_ = stream.Send(&proto.RunExtensionInstallerExtensionMessage{
				MessageType: &proto.RunExtensionInstallerExtensionMessage_Query{
					Query: extensionMsg.Query,
				},
			})

			clientResponseRaw, err := stream.Recv()
			if err != nil {
				return status.Errorf(codes.Internal, "could not receive message from client; %v", err)
			}

			clientResponse, ok := clientResponseRaw.MessageType.(*proto.RunExtensionInstallerClientMessage_Msg)
			if !ok {
				return status.Errorf(codes.Internal, "client sent incorrect message; sent init when should have sent regular msg;")
			}

			err = extensionConn.Send(&proto.ExtensionRunExtensionInstallerClientMessage{
				Msg: clientResponse.Msg,
			})
			if err != nil {
				return status.Errorf(codes.Internal, "could not send message to extension; %v", err)
			}
		default:
			return status.Errorf(codes.Internal, "received incorrect message type during extension installer; %T",
				extensionMsgRaw.MessageType)
		}
	}
}

func (api *API) RunPipelineConfigurator(stream proto.Gofer_RunPipelineConfiguratorServer) error {
	if !isManagementUser(stream.Context()) {
		return status.Error(codes.PermissionDenied, "management token required for this action")
	}

	// Get the first message so we can attempt to set up the connection with the proper docker container.
	initMessageRaw, err := stream.Recv()
	if err != nil {
		log.Error().Err(err).Msg("could not set up stream")
		return status.Errorf(codes.Internal, "could not set up stream: %v", err)
	}

	initMessage, ok := initMessageRaw.MessageType.(*proto.RunPipelineConfiguratorClientMessage_Init_)
	if !ok {
		return status.Error(codes.FailedPrecondition, "first message must be init message, received wrong message type")
	}

	// Validate input
	if initMessage.Init.Name == "" {
		return status.Error(codes.FailedPrecondition, "extension name required")
	}

	extension, exists := api.extensions.Get(initMessage.Init.Name)
	if !exists {
		return status.Error(codes.FailedPrecondition, "extension does not exist")
	}

	conn, err := grpcDial(extension.URL)
	if err != nil {
		log.Error().Err(err).Str("name", extension.Registration.Name).Msg("could not connect to extension")
	}
	defer conn.Close()

	client := proto.NewExtensionServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*extension.Key))
	extensionConn, err := client.RunPipelineConfigurator(ctx)
	if err != nil {
		return status.Errorf(codes.Internal, "could not connect to extension while running pipeline configuration; %v", err)
	}

	// Simply copy the messages from the extension and client together.
	for {
		extensionMsgRaw, err := extensionConn.Recv()
		if err != nil {
			// If the context was cancelled, that means that the extension is done and we should process the installation.
			if strings.Contains(err.Error(), "context canceled") {
				return nil
			}

			// If the client disconnected, exit cleanly.
			if strings.Contains(err.Error(), "client disconnected") {
				return nil
			}

			return status.Errorf(codes.Internal, "could not receive message from extension during pipeline configuration; %v", err)
		}

		switch extensionMsg := extensionMsgRaw.MessageType.(type) {
		case *proto.ExtensionRunPipelineConfiguratorExtensionMessage_ParamSetting_:
			_ = stream.Send(&proto.RunPipelineConfiguratorExtensionMessage{
				MessageType: &proto.RunPipelineConfiguratorExtensionMessage_ParamSetting_{
					ParamSetting: &proto.RunPipelineConfiguratorExtensionMessage_ParamSetting{
						Param: extensionMsg.ParamSetting.Param,
						Value: extensionMsg.ParamSetting.Value,
					},
				},
			})

		case *proto.ExtensionRunPipelineConfiguratorExtensionMessage_Msg:
			_ = stream.Send(&proto.RunPipelineConfiguratorExtensionMessage{
				MessageType: &proto.RunPipelineConfiguratorExtensionMessage_Msg{
					Msg: extensionMsg.Msg,
				},
			})

		case *proto.ExtensionRunPipelineConfiguratorExtensionMessage_Query:
			_ = stream.Send(&proto.RunPipelineConfiguratorExtensionMessage{
				MessageType: &proto.RunPipelineConfiguratorExtensionMessage_Query{
					Query: extensionMsg.Query,
				},
			})

			clientResponseRaw, err := stream.Recv()
			if err != nil {
				return status.Errorf(codes.Internal, "could not receive message from client; %v", err)
			}

			clientResponse, ok := clientResponseRaw.MessageType.(*proto.RunPipelineConfiguratorClientMessage_Msg)
			if !ok {
				return status.Errorf(codes.Internal, "client sent incorrect message; sent init when should have sent regular msg;")
			}

			err = extensionConn.Send(&proto.ExtensionRunPipelineConfiguratorClientMessage{
				Msg: clientResponse.Msg,
			})
			if err != nil {
				return status.Errorf(codes.Internal, "could not send message to extension; %v", err)
			}
		default:
			return status.Errorf(codes.Internal, "received incorrect message type during pipeline configuration; %T",
				extensionMsgRaw.MessageType)
		}
	}
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

	_, exists := api.extensions.Get(request.Name)
	if !exists {
		return nil, status.Errorf(codes.FailedPrecondition, "no extension %q found", request.Name)
	}

	containerID := extensionContainerID(request.Name)

	api.scheduler.StopContainer(scheduler.StopContainerRequest{
		ID: containerID,
	})

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
