package api

import (
	"context"

	"github.com/clintjedwards/gofer/proto"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetNotifier(ctx context.Context, request *proto.GetNotifierRequest) (*proto.GetNotifierResponse, error) {
	if request.Name == "" {
		return &proto.GetNotifierResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	notifier, exists := api.notifiers[request.Name]
	if !exists {
		return &proto.GetNotifierResponse{}, status.Error(codes.NotFound, "could not find notifier")
	}

	return &proto.GetNotifierResponse{Notifier: notifier.ToProto()}, nil
}

func (api *API) ListNotifiers(ctx context.Context, request *proto.ListNotifiersRequest) (*proto.ListNotifiersResponse, error) {
	protoNotifiers := []*proto.Notifier{}
	for _, notifier := range api.notifiers {
		protoNotifiers = append(protoNotifiers, notifier.ToProto())
	}

	return &proto.ListNotifiersResponse{
		Notifiers: protoNotifiers,
	}, nil
}
