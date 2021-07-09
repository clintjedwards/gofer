package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) ListTriggerEvents(ctx context.Context, request *proto.ListTriggerEventsRequest) (*proto.ListTriggerEventsResponse, error) {
	if request.PipelineId == "" {
		return &proto.ListTriggerEventsResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.PipelineTriggerLabel == "" {
		return &proto.ListTriggerEventsResponse{}, status.Error(codes.FailedPrecondition, "subscription id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	events, err := api.storage.GetAllTriggerEvents(storage.GetAllTriggerEventsRequest{
		NamespaceID:          request.NamespaceId,
		PipelineID:           request.PipelineId,
		PipelineTriggerLabel: request.PipelineTriggerLabel,
		Offset:               int(request.Offset),
		Limit:                int(request.Limit),
	})
	if err != nil {
		log.Error().Err(err).Msg("could not get events")
		return &proto.ListTriggerEventsResponse{}, status.Error(codes.Internal, "failed to retrieve events from database")
	}

	protoEvents := []*proto.TriggerEvent{}
	for _, event := range events {
		protoEvents = append(protoEvents, event.ToProto())
	}

	return &proto.ListTriggerEventsResponse{
		Events: protoEvents,
	}, nil
}

func (api *API) BatchGetTriggerEvents(ctx context.Context, request *proto.BatchGetTriggerEventsRequest) (*proto.BatchGetTriggerEventsResponse, error) {
	if request.NamespaceId == "" {
		return &proto.BatchGetTriggerEventsResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.PipelineId == "" {
		return &proto.BatchGetTriggerEventsResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.PipelineTriggerLabel == "" {
		return &proto.BatchGetTriggerEventsResponse{}, status.Error(codes.FailedPrecondition, "subscription id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if len(request.Ids) == 0 {
		return &proto.BatchGetTriggerEventsResponse{}, status.Error(codes.FailedPrecondition, "at least one ID required")
	}

	events := []*proto.TriggerEvent{}

	for _, id := range request.Ids {
		event, err := api.storage.GetTriggerEvent(storage.GetTriggerEventRequest{
			NamespaceID:          request.NamespaceId,
			PipelineID:           request.PipelineId,
			PipelineTriggerLabel: request.PipelineTriggerLabel,
			ID:                   id,
		})
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return &proto.BatchGetTriggerEventsResponse{}, status.Errorf(codes.FailedPrecondition, "event %q not found", id)
			}
			log.Error().Err(err).Int64("event", id).Msg("could not get event")
			return &proto.BatchGetTriggerEventsResponse{}, status.Errorf(codes.Internal, "failed to retrieve event %q from database", id)
		}

		events = append(events, event.ToProto())
	}

	return &proto.BatchGetTriggerEventsResponse{Events: events}, nil
}

func (api *API) GetTriggerEvent(ctx context.Context, request *proto.GetTriggerEventRequest) (*proto.GetTriggerEventResponse, error) {
	if request.PipelineId == "" {
		return &proto.GetTriggerEventResponse{}, status.Error(codes.FailedPrecondition, "pipeline id required")
	}

	if request.PipelineTriggerLabel == "" {
		return &proto.GetTriggerEventResponse{}, status.Error(codes.FailedPrecondition, "subscription id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if request.Id == 0 {
		return &proto.GetTriggerEventResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	event, err := api.storage.GetTriggerEvent(storage.GetTriggerEventRequest{
		NamespaceID:          request.NamespaceId,
		PipelineID:           request.PipelineId,
		PipelineTriggerLabel: request.PipelineTriggerLabel,
		ID:                   request.Id,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetTriggerEventResponse{}, status.Error(codes.FailedPrecondition, "event not found")
		}
		log.Error().Err(err).Msg("could not get event")
		return &proto.GetTriggerEventResponse{}, status.Error(codes.Internal, "failed to retrieve event from database")
	}

	return &proto.GetTriggerEventResponse{Event: event.ToProto()}, nil
}
