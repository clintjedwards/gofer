package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetDeployment(ctx context.Context, request *proto.GetDeploymentRequest) (*proto.GetDeploymentResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetDeploymentResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	deploymentRaw, err := api.db.GetPipelineDeployment(api.db, request.NamespaceId, request.PipelineId, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetDeploymentResponse{}, status.Error(codes.FailedPrecondition, "deployment not found")
		}
		log.Error().Err(err).Int64("Deployment", request.Id).Msg("could not get deployment")
		return &proto.GetDeploymentResponse{}, status.Error(codes.Internal, "failed to retrieve deployment from database")
	}

	var deployment models.Deployment
	deployment.FromStorage(&deploymentRaw)

	return &proto.GetDeploymentResponse{Deployment: deployment.ToProto()}, nil
}

func (api *API) ListDeployments(ctx context.Context, request *proto.ListDeploymentsRequest) (*proto.ListDeploymentsResponse, error) {
	if request.PipelineId == "" {
		return &proto.ListDeploymentsResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListDeploymentsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	deployments, err := api.db.ListPipelineDeployments(api.db, int(request.Offset), int(request.Limit), request.NamespaceId, request.PipelineId)
	if err != nil {
		log.Error().Err(err).Msg("could not get deployments")
		return &proto.ListDeploymentsResponse{}, status.Error(codes.Internal, "failed to retrieve deployments from database")
	}

	protoDeployments := []*proto.Deployment{}
	for _, deploymentRaw := range deployments {
		var deployment models.Deployment
		deployment.FromStorage(&deploymentRaw)
		protoDeployments = append(protoDeployments, deployment.ToProto())
	}

	return &proto.ListDeploymentsResponse{
		Deployments: protoDeployments,
	}, nil
}
