package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetTrigger(ctx context.Context, request *proto.GetTriggerRequest) (*proto.GetTriggerResponse, error) {
	if request.Kind == "" {
		return &proto.GetTriggerResponse{}, status.Error(codes.FailedPrecondition, "kind required")
	}

	trigger, exists := api.triggers[request.Kind]
	if !exists {
		log.Error().Msg("trigger was found in config, but not in run information")
		return &proto.GetTriggerResponse{}, status.Error(codes.NotFound, "trigger not found")
	}

	return &proto.GetTriggerResponse{Trigger: trigger.ToProto()}, nil
}

func (api *API) ListTriggers(ctx context.Context, request *proto.ListTriggersRequest) (*proto.ListTriggersResponse, error) {
	protoTriggers := []*proto.Trigger{}
	for _, trigger := range api.triggers {
		protoTriggers = append(protoTriggers, trigger.ToProto())
	}

	return &proto.ListTriggersResponse{
		Triggers: protoTriggers,
	}, nil
}

func (api *API) InstallTrigger(ctx context.Context, request *proto.InstallTriggerRequest) (*proto.InstallTriggerResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.InstallTriggerResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	triggerConfig := &config.Trigger{
		Kind:    request.Trigger.Kind,
		Image:   request.Trigger.Image,
		User:    request.Trigger.User,
		Pass:    request.Trigger.Pass,
		EnvVars: request.Trigger.EnvVars,
	}

	err := api.startTrigger(triggerConfig, generateToken(32))
	if err != nil {
		log.Error().Err(err).Msg("could not install trigger; could not start trigger")
		return &proto.InstallTriggerResponse{}, status.Errorf(codes.FailedPrecondition, "could not start trigger")
	}

	err = api.storage.AddTrigger(storage.AddTriggerRequest{
		Trigger: triggerConfig,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.InstallTriggerResponse{}, status.Errorf(codes.AlreadyExists, "trigger kind already exists;")
		}
		log.Error().Err(err).Msg("could not install trigger")
		return &proto.InstallTriggerResponse{}, status.Error(codes.Internal, "could not install trigger")
	}

	return &proto.InstallTriggerResponse{}, nil
}

func (api *API) UninstallTrigger(ctx context.Context, request *proto.UninstallTriggerRequest) (*proto.UninstallTriggerResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.UninstallTriggerResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	err := api.stopTrigger(request.Kind)
	if err != nil {
		log.Error().Err(err).Msg("could not stop trigger during uninstall process")
		return &proto.UninstallTriggerResponse{}, status.Error(codes.Internal, "could not uninstall trigger")
	}

	err = api.storage.DeleteTrigger(storage.DeleteTriggerRequest{
		Kind: request.Kind,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not uninstall trigger")
		return &proto.UninstallTriggerResponse{}, status.Error(codes.Internal, "could not uninstall trigger")
	}

	return &proto.UninstallTriggerResponse{}, nil
}
