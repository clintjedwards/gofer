package api

import (
	"context"
	"errors"
	"fmt"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/jmoiron/sqlx"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) CreatePipelineExtensionSubscription(ctx context.Context, request *proto.CreatePipelineExtensionSubscriptionRequest) (*proto.CreatePipelineExtensionSubscriptionResponse, error) {
	if request.PipelineId == "" {
		return &proto.CreatePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.ExtensionName == "" {
		return &proto.CreatePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension name required")
	}

	if request.ExtensionLabel == "" {
		return &proto.CreatePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension label required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return nil, status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return nil, status.Error(codes.PermissionDenied, "access denied")
	}

	subscription := models.FromCreatePipelineExtensionSubscriptionRequest(request)
	subStorage := subscription.ToStorage()

	err = storage.InsideTx(api.db.DB, func(*sqlx.Tx) error {
		_, err = api.db.GetPipelineMetadata(api.db, request.NamespaceId, request.PipelineId)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return fmt.Errorf("%w; could not find pipeline", err)
			}
			return err
		}

		_, err = api.db.GetGlobalExtensionRegistration(api.db, request.ExtensionName)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return fmt.Errorf("%w; could not find extension", err)
			}
			return err
		}

		err = api.subscribeExtension(subscription)
		if err != nil {
			return err
		}

		err = api.db.InsertPipelineExtensionSubscription(api.db, subStorage)
		if err != nil {
			log.Error().Err(err).Str("namespace", request.NamespaceId).Str("pipeline", request.PipelineId).
				Str("extension_name", request.ExtensionName).Msg("could not complete extension subscription")
			return err
		}

		return nil
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return nil, status.Errorf(codes.FailedPrecondition, "entity does not exist;  %v", err.Error())
		}
		return nil, status.Errorf(codes.Internal, "could not subscribe to extension;  %v", err.Error())
	}

	go api.events.Publish(models.EventCompletedExtensionSubscriptionPipeline{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		Label:       request.ExtensionLabel,
		Name:        request.ExtensionName,
	})

	return &proto.CreatePipelineExtensionSubscriptionResponse{}, nil
}

func (api *API) ListPipelineExtensionSubscriptions(ctx context.Context, request *proto.ListPipelineExtensionSubscriptionsRequest) (
	*proto.ListPipelineExtensionSubscriptionsResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.ListPipelineExtensionSubscriptionsResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListPipelineExtensionSubscriptionsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	subscriptionsRaw, err := api.db.ListPipelineExtensionSubscriptions(api.db, request.NamespaceId, request.PipelineId)
	if err != nil {
		log.Error().Err(err).Msg("could not get subscriptions")
		return &proto.ListPipelineExtensionSubscriptionsResponse{}, status.Error(codes.Internal,
			"failed to retrieve subscriptions from database")
	}

	protoSubscriptions := []*proto.PipelineExtensionSubscription{}
	for _, subRaw := range subscriptionsRaw {
		var sub models.PipelineExtensionSubscription
		sub.FromStorage(&subRaw)
		protoSubscriptions = append(protoSubscriptions, sub.ToProto())
	}

	return &proto.ListPipelineExtensionSubscriptionsResponse{
		Subscriptions: protoSubscriptions,
	}, nil
}

func (api *API) GetPipelineExtensionSubscription(ctx context.Context, request *proto.GetPipelineExtensionSubscriptionRequest) (
	*proto.GetPipelineExtensionSubscriptionResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.GetPipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.ExtensionName == "" {
		return &proto.GetPipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension name required")
	}

	if request.ExtensionLabel == "" {
		return &proto.GetPipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension label required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetPipelineExtensionSubscriptionResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	subRaw, err := api.db.GetPipelineExtensionSubscription(api.db, request.NamespaceId, request.PipelineId, request.ExtensionName, request.ExtensionLabel)
	if err != nil {
		log.Error().Err(err).Msg("could not get subscription")
		return &proto.GetPipelineExtensionSubscriptionResponse{}, status.Error(codes.Internal, "failed to retrieve subscription from database")
	}

	var sub models.PipelineExtensionSubscription
	sub.FromStorage(&subRaw)

	return &proto.GetPipelineExtensionSubscriptionResponse{Subscription: sub.ToProto()}, nil
}

func (api *API) EnablePipelineExtensionSubscription(ctx context.Context, request *proto.EnablePipelineExtensionSubscriptionRequest) (
	*proto.EnablePipelineExtensionSubscriptionResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.EnablePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.ExtensionName == "" {
		return &proto.EnablePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension name required")
	}

	if request.ExtensionLabel == "" {
		return &proto.EnablePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension label required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.EnablePipelineExtensionSubscriptionResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.EnablePipelineExtensionSubscriptionResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.db.UpdatePipelineExtensionSubscription(api.db, request.NamespaceId, request.PipelineId, request.ExtensionName,
		request.ExtensionLabel, storage.UpdateablePipelineExtensionSubscriptionFields{
			Status: ptr(string(models.ExtensionSubscriptionStatusActive)),
		})
	if err != nil {
		log.Error().Err(err).Str("namespace", request.NamespaceId).Str("pipeline", request.PipelineId).
			Str("extension_name", request.ExtensionName).Msg("could not update extension subscription status")
	}

	return &proto.EnablePipelineExtensionSubscriptionResponse{}, nil
}

func (api *API) DisablePipelineExtensionSubscription(ctx context.Context, request *proto.DisablePipelineExtensionSubscriptionRequest) (
	*proto.DisablePipelineExtensionSubscriptionResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.DisablePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.ExtensionName == "" {
		return &proto.DisablePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension name required")
	}

	if request.ExtensionLabel == "" {
		return &proto.DisablePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension label required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DisablePipelineExtensionSubscriptionResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DisablePipelineExtensionSubscriptionResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.db.UpdatePipelineExtensionSubscription(api.db, request.NamespaceId, request.PipelineId, request.ExtensionName,
		request.ExtensionLabel, storage.UpdateablePipelineExtensionSubscriptionFields{
			Status: ptr(string(models.ExtensionSubscriptionStatusDisabled)),
		})
	if err != nil {
		log.Error().Err(err).Str("namespace", request.NamespaceId).Str("pipeline", request.PipelineId).
			Str("extension_name", request.ExtensionName).Msg("could not update extension subscription status")
	}

	return &proto.DisablePipelineExtensionSubscriptionResponse{}, nil
}

func (api *API) DeletePipelineExtensionSubscription(ctx context.Context, request *proto.DeletePipelineExtensionSubscriptionRequest) (
	*proto.DeletePipelineExtensionSubscriptionResponse, error,
) {
	if request.PipelineId == "" {
		return &proto.DeletePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.ExtensionName == "" {
		return &proto.DeletePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension name required")
	}

	if request.ExtensionLabel == "" {
		return &proto.DeletePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "extension label required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeletePipelineExtensionSubscriptionResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineExtensionSubscriptionResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.db.DeletePipelineExtensionSubscription(api.db, request.NamespaceId, request.PipelineId, request.ExtensionName, request.ExtensionLabel)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeletePipelineExtensionSubscriptionResponse{}, status.Error(codes.FailedPrecondition, "subscription not found")
		}
		log.Error().Err(err).Msg("could not get subscription")
		return &proto.DeletePipelineExtensionSubscriptionResponse{}, status.Error(codes.Internal, "failed to retrieve subscription from database")
	}

	err = api.unsubscribeExtension(request.NamespaceId, request.PipelineId, request.ExtensionName, request.ExtensionLabel)
	if err != nil {
		log.Error().Err(err).Str("namespace", request.NamespaceId).Str("pipeline", request.PipelineId).
			Str("extension_name", request.ExtensionName).Msg("could not complete extension subscription removal")
		return nil, status.Errorf(codes.FailedPrecondition, "could not subscribe to extension removal;  %v", err.Error())
	}

	go api.events.Publish(models.EventCompletedExtensionSubscriptionRemovalPipeline{
		NamespaceID: request.NamespaceId,
		PipelineID:  request.PipelineId,
		Label:       request.ExtensionLabel,
		Name:        request.ExtensionName,
	})

	return &proto.DeletePipelineExtensionSubscriptionResponse{}, nil
}
