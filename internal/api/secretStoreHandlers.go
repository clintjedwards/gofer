package api

import (
	"context"

	proto "github.com/clintjedwards/gofer/proto/go"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetSecret(ctx context.Context, request *proto.GetSecretRequest) (*proto.GetSecretResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.GetSecretResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	content, err := api.secretStore.GetSecret(secretKey(request.NamespaceId, request.PipelineId, request.Key))
	if err != nil {
		return &proto.GetSecretResponse{}, err
	}

	return &proto.GetSecretResponse{
		Content: content,
	}, nil
}

func (api *API) PutSecret(ctx context.Context, request *proto.PutSecretRequest) (*proto.PutSecretResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.PutSecretResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err := api.secretStore.PutSecret(secretKey(request.NamespaceId, request.PipelineId, request.Key), request.Content, request.Force)
	if err != nil {
		return &proto.PutSecretResponse{}, err
	}

	return &proto.PutSecretResponse{
		Bytes: int64(len(request.Content)),
	}, nil
}

func (api *API) DeleteSecret(ctx context.Context, request *proto.DeleteSecretRequest) (*proto.DeleteSecretResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeleteSecretResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err := api.secretStore.DeleteSecret(secretKey(request.NamespaceId, request.PipelineId, request.Key))
	if err != nil {
		return &proto.DeleteSecretResponse{}, err
	}

	return &proto.DeleteSecretResponse{}, nil
}
