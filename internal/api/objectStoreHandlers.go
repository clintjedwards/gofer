package api

import (
	"context"
	"errors"
	"fmt"

	objectstore "github.com/clintjedwards/gofer/internal/objectStore"
	proto "github.com/clintjedwards/gofer/proto/go"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetPipelineObject(ctx context.Context, request *proto.GetPipelineObjectRequest) (*proto.GetPipelineObjectResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.GetPipelineObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	content, err := api.objectStore.GetObject(pipelineObjectKey(request.NamespaceId, request.PipelineId, request.Key))
	if err != nil {
		return &proto.GetPipelineObjectResponse{}, err
	}

	return &proto.GetPipelineObjectResponse{
		Content: content,
	}, nil
}

func (api *API) PutPipelineObject(ctx context.Context, request *proto.PutPipelineObjectRequest) (*proto.PutPipelineObjectResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.PutPipelineObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	evictedObject, err := api.addPipelineObject(request.NamespaceId,
		request.PipelineId, request.Key, request.Content, request.Force)
	if err != nil {
		if errors.Is(err, objectstore.ErrEntityExists) {
			return &proto.PutPipelineObjectResponse{}, status.Error(codes.FailedPrecondition,
				fmt.Sprintf("object already exists for key %q; try using the 'force' to overwrite", request.Key))
		}
		return &proto.PutPipelineObjectResponse{}, status.Error(codes.Internal, fmt.Sprintf("could not put object %q; %v", request.Key, err))
	}

	return &proto.PutPipelineObjectResponse{
		Bytes:         int64(len(request.Content)),
		ObjectLimit:   int64(api.config.ObjectStore.PipelineObjectLimit),
		ObjectEvicted: evictedObject,
	}, nil
}

func (api *API) DeletePipelineObject(ctx context.Context, request *proto.DeletePipelineObjectRequest) (*proto.DeletePipelineObjectResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err := api.objectStore.DeleteObject(pipelineObjectKey(request.NamespaceId, request.PipelineId, request.Key))
	if err != nil {
		return &proto.DeletePipelineObjectResponse{}, status.Error(codes.Internal, fmt.Sprintf("could not delete object %q; %v", request.Key, err))
	}

	return &proto.DeletePipelineObjectResponse{}, nil
}

func (api *API) GetRunObject(ctx context.Context, request *proto.GetRunObjectRequest) (*proto.GetRunObjectResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.GetRunObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	content, err := api.objectStore.GetObject(runObjectKey(request.NamespaceId, request.PipelineId, request.RunId, request.Key))
	if err != nil {
		return nil, err
	}

	return &proto.GetRunObjectResponse{
		Content: content,
	}, nil
}

func (api *API) PutRunObject(ctx context.Context, request *proto.PutRunObjectRequest) (*proto.PutRunObjectResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.PutRunObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err := api.objectStore.PutObject(runObjectKey(request.NamespaceId, request.PipelineId, request.RunId, request.Key), request.Content, request.Force)
	if err != nil {
		if errors.Is(err, objectstore.ErrEntityExists) {
			return &proto.PutRunObjectResponse{}, status.Error(codes.FailedPrecondition,
				fmt.Sprintf("object already exists for key %q; try using the '--force' flag to overwrite", request.Key))
		}
		return &proto.PutRunObjectResponse{}, status.Error(codes.Internal, fmt.Sprintf("could not put object %q; %v", request.Key, err))
	}

	return &proto.PutRunObjectResponse{
		Bytes: int64(len(request.Content)),
	}, nil
}

func (api *API) DeleteRunObject(ctx context.Context, request *proto.DeleteRunObjectRequest) (*proto.DeleteRunObjectResponse, error) {
	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeleteRunObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err := api.objectStore.DeleteObject(runObjectKey(request.NamespaceId, request.PipelineId, request.RunId, request.Key))
	if err != nil {
		return nil, err
	}

	return &proto.DeleteRunObjectResponse{}, nil
}
