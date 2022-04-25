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

func (api *API) GetNotifier(ctx context.Context, request *proto.GetNotifierRequest) (*proto.GetNotifierResponse, error) {
	if request.Kind == "" {
		return &proto.GetNotifierResponse{}, status.Error(codes.FailedPrecondition, "kind required")
	}

	notifier, exists := api.notifiers.Get(request.Kind)
	if !exists {
		log.Error().Msg("notifier was found in config, but not in run information")
		return &proto.GetNotifierResponse{}, status.Error(codes.NotFound, "notifier not found")
	}

	return &proto.GetNotifierResponse{Notifier: notifier.ToProto()}, nil
}

func (api *API) ListNotifiers(ctx context.Context, request *proto.ListNotifiersRequest) (*proto.ListNotifiersResponse, error) {
	protoNotifiers := []*proto.Notifier{}
	for _, id := range api.notifiers.Keys() {
		notifier, exists := api.notifiers.Get(id)
		if !exists {
			continue
		}
		protoNotifiers = append(protoNotifiers, notifier.ToProto())
	}

	return &proto.ListNotifiersResponse{
		Notifiers: protoNotifiers,
	}, nil
}

func (api *API) InstallNotifier(ctx context.Context, request *proto.InstallNotifierRequest) (*proto.InstallNotifierResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.InstallNotifierResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	notifierConfig := &config.Notifier{
		Kind:    request.Notifier.Kind,
		Image:   request.Notifier.Image,
		User:    request.Notifier.User,
		Pass:    request.Notifier.Pass,
		EnvVars: request.Notifier.EnvVars,
	}

	api.registerNotifier(*notifierConfig)

	err := api.storage.AddNotifier(storage.AddNotifierRequest{
		Notifier: notifierConfig,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.InstallNotifierResponse{}, status.Errorf(codes.AlreadyExists, "notifier kind already exists;")
		}
		log.Error().Err(err).Msg("could not install notifier")
		return &proto.InstallNotifierResponse{}, status.Error(codes.Internal, "could not install notifier")
	}

	return &proto.InstallNotifierResponse{}, nil
}

func (api *API) UninstallNotifier(ctx context.Context, request *proto.UninstallNotifierRequest) (*proto.UninstallNotifierResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.UninstallNotifierResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	api.notifiers.Delete(request.Kind)

	err := api.storage.DeleteNotifier(storage.DeleteNotifierRequest{
		Kind: request.Kind,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not uninstall notifier")
		return &proto.UninstallNotifierResponse{}, status.Error(codes.Internal, "could not uninstall notifier")
	}

	return &proto.UninstallNotifierResponse{}, nil
}
