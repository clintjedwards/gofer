package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/eventbus"
	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetEvent(ctx context.Context, request *proto.GetEventRequest) (*proto.GetEventResponse, error) {
	if request.Id == 0 {
		return &proto.GetEventResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	event, err := api.events.Get(request.Id)
	if err != nil {
		if errors.Is(err, eventbus.ErrEventNotFound) {
			return &proto.GetEventResponse{}, status.Error(codes.FailedPrecondition, "event not found")
		}
		log.Error().Err(err).Msg("could not get event")
		return &proto.GetEventResponse{}, status.Error(codes.Internal, "failed to retrieve event from database")
	}

	switch evt := event.(type) {
	case *models.EventCreatedNamespace:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_CreatedNamespaceEvent{
				CreatedNamespaceEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventDisabledPipeline:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_DisabledPipelineEvent{
				DisabledPipelineEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventEnabledPipeline:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_EnabledPipelineEvent{
				EnabledPipelineEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventCreatedPipeline:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_CreatedPipelineEvent{
				CreatedPipelineEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventAbandonedPipeline:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_AbandonedPipelineEvent{
				AbandonedPipelineEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventStartedRun:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_StartedRunEvent{
				StartedRunEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventCompletedRun:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_CompletedRunEvent{
				CompletedRunEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventStartedTaskRun:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_StartedTaskRunEvent{
				StartedTaskRunEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventScheduledTaskRun:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_ScheduledTaskRunEvent{
				ScheduledTaskRunEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventCompletedTaskRun:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_CompletedTaskRunEvent{
				CompletedTaskRunEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventFiredTrigger:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_FiredTriggerEvent{
				FiredTriggerEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventProcessedTrigger:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_ProcessedTriggerEvent{
				ProcessedTriggerEvent: evt.ToProto(),
			},
		}, nil
	case *models.EventResolvedTrigger:
		return &proto.GetEventResponse{
			Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
			Event: &proto.GetEventResponse_ResolvedTriggerEvent{
				ResolvedTriggerEvent: evt.ToProto(),
			},
		}, nil
	default:
		log.Error().Int64("id", evt.GetID()).Str("kind", string(evt.GetKind())).Msg("could not serialize event")
		return nil, status.Error(codes.Internal, "could not serialize event")
	}
}

func (api *API) ListEvents(request *proto.ListEventsRequest, stream proto.Gofer_ListEventsServer) error {
	historicalEvents := api.events.GetAll(request.Reverse)

	subscription, err := api.events.Subscribe(models.AnyEvent)
	if err != nil {
		return status.Errorf(codes.Internal, "could not subscribe to event stream: %v", err)
	}
	defer api.events.Unsubscribe(subscription)

	// God help me. Huge switch statements like this seem to be the only way to do this.
	// We do two separate switch statements because we want the historical events to drain out first typically.
historicalLoop:
	for {
		select {
		case <-stream.Context().Done():
			return nil
		case <-api.context.ctx.Done():
			return nil
		case event := <-historicalEvents:
			if event == nil {
				break historicalLoop
			}

			switch evt := event.(type) {
			case *models.EventCreatedNamespace:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CreatedNamespaceEvent{
						CreatedNamespaceEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventDisabledPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_DisabledPipelineEvent{
						DisabledPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventEnabledPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_EnabledPipelineEvent{
						EnabledPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventCreatedPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CreatedPipelineEvent{
						CreatedPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventAbandonedPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_AbandonedPipelineEvent{
						AbandonedPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventStartedRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_StartedRunEvent{
						StartedRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventCompletedRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CompletedRunEvent{
						CompletedRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventStartedTaskRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_StartedTaskRunEvent{
						StartedTaskRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventScheduledTaskRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_ScheduledTaskRunEvent{
						ScheduledTaskRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventCompletedTaskRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CompletedTaskRunEvent{
						CompletedTaskRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventFiredTrigger:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_FiredTriggerEvent{
						FiredTriggerEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventProcessedTrigger:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_ProcessedTriggerEvent{
						ProcessedTriggerEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
			case *models.EventResolvedTrigger:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_ResolvedTriggerEvent{
						ResolvedTriggerEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
				continue
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
		case event := <-subscription.Events:
			if event == nil {
				return nil
			}

			switch evt := event.(type) {
			case *models.EventCreatedNamespace:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CreatedNamespaceEvent{
						CreatedNamespaceEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventDisabledPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_DisabledPipelineEvent{
						DisabledPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventEnabledPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_EnabledPipelineEvent{
						EnabledPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventCreatedPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CreatedPipelineEvent{
						CreatedPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventAbandonedPipeline:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_AbandonedPipelineEvent{
						AbandonedPipelineEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventStartedRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_StartedRunEvent{
						StartedRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventCompletedRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CompletedRunEvent{
						CompletedRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventStartedTaskRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_StartedTaskRunEvent{
						StartedTaskRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventScheduledTaskRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_ScheduledTaskRunEvent{
						ScheduledTaskRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventCompletedTaskRun:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_CompletedTaskRunEvent{
						CompletedTaskRunEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventFiredTrigger:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_FiredTriggerEvent{
						FiredTriggerEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventProcessedTrigger:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_ProcessedTriggerEvent{
						ProcessedTriggerEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			case *models.EventResolvedTrigger:
				err := stream.Send(&proto.ListEventsResponse{
					Kind: proto.EventType(proto.EventType_value[string(evt.GetKind())]),
					Event: &proto.ListEventsResponse_ResolvedTriggerEvent{
						ResolvedTriggerEvent: evt.ToProto(),
					},
				})
				if err != nil {
					if status.Code(err) == codes.Unavailable {
						return nil
					}
					log.Error().Err(err).Msg("could not send event")
					return status.Errorf(codes.Internal, "could not send event: %v", err)
				}
			}
		default:
			if !request.Follow {
				return nil
			}
		}
	}
}
