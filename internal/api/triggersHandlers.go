package api

import (
	"context"

	"github.com/clintjedwards/gofer/proto"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetTrigger(ctx context.Context, request *proto.GetTriggerRequest) (*proto.GetTriggerResponse, error) {
	if request.Name == "" {
		return &proto.GetTriggerResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	trigger, exists := api.triggers[request.Name]
	if !exists {
		return &proto.GetTriggerResponse{}, status.Error(codes.NotFound, "could not find trigger")
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
