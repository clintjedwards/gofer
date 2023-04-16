package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/events"
	"github.com/clintjedwards/gofer/internal/eventbus"
	"github.com/clintjedwards/gofer/internal/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetEvent(ctx context.Context, request *proto.GetEventRequest) (*proto.GetEventResponse, error) {
	if request.Id == 0 {
		return &proto.GetEventResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	eventRaw, err := api.events.Get(request.Id)
	if err != nil {
		if errors.Is(err, eventbus.ErrEventNotFound) {
			return &proto.GetEventResponse{}, status.Error(codes.FailedPrecondition, "event not found")
		}
		log.Error().Err(err).Msg("could not get event")
		return &proto.GetEventResponse{}, status.Error(codes.Internal, "failed to retrieve event from database")
	}

	event := models.FromEvent(&eventRaw)
	protoEvent, err := event.ToProto()
	if err != nil {
		log.Error().Err(err).Msg("could not encode proto event")
		return &proto.GetEventResponse{}, status.Error(codes.Internal, "failed to retrieve event from database")
	}

	return &proto.GetEventResponse{
		Event: protoEvent,
	}, nil
}

func (api *API) ListEvents(request *proto.ListEventsRequest, stream proto.Gofer_ListEventsServer) error {
	historicalEvents := api.events.GetAll(request.Reverse)

	subscription, err := api.events.Subscribe(events.EventTypeAny)
	if err != nil {
		return status.Errorf(codes.Internal, "could not subscribe to event stream: %v", err)
	}
	defer api.events.Unsubscribe(subscription)

	// We do two separate switch statements because we want the historical events to drain out first typically.
historicalLoop:
	for {
		select {
		case <-stream.Context().Done():
			return nil
		case <-api.context.ctx.Done():
			return nil
		case eventRaw, more := <-historicalEvents:
			if !more {
				break historicalLoop
			}

			event := models.FromEvent(&eventRaw)
			protoEvent, err := event.ToProto()
			if err != nil {
				if status.Code(err) == codes.Unavailable {
					return nil
				}
				log.Error().Err(err).Msg("could not send event")
				return status.Errorf(codes.Internal, "could not send event: %v", err)
			}

			err = stream.Send(&proto.ListEventsResponse{Event: protoEvent})
			if err != nil {
				if status.Code(err) == codes.Unavailable {
					return nil
				}
				log.Error().Err(err).Msg("could not send event")
				return status.Errorf(codes.Internal, "could not send event: %v", err)
			}
		}
	}

	// If the user wants the events in reverse order there is no need to wait for incoming events
	// so once we finish the historical events we just exit.
	if request.Reverse {
		return nil
	}

	for {
		select {
		case <-stream.Context().Done():
			return nil
		case <-api.context.ctx.Done():
			return nil
		case eventRaw, more := <-subscription.Events:
			if !more {
				return nil
			}

			event := models.FromEvent(&eventRaw)
			protoEvent, err := event.ToProto()
			if err != nil {
				if status.Code(err) == codes.Unavailable {
					return nil
				}
				log.Error().Err(err).Msg("could not send event")
				return status.Errorf(codes.Internal, "could not send event: %v", err)
			}

			err = stream.Send(&proto.ListEventsResponse{Event: protoEvent})
			if err != nil {
				if status.Code(err) == codes.Unavailable {
					return nil
				}
				log.Error().Err(err).Msg("could not send event")
				return status.Errorf(codes.Internal, "could not send event: %v", err)
			}
		default:
			if !request.Follow {
				return nil
			}
		}
	}
}
