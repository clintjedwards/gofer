package api

import (
	"context"
	"errors"
	"fmt"

	"github.com/clintjedwards/gofer/internal/models"
	objectstore "github.com/clintjedwards/gofer/internal/objectStore"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) ListPipelineObjects(ctx context.Context, request *proto.ListPipelineObjectsRequest) (*proto.ListPipelineObjectsResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListPipelineObjectsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.ListPipelineObjectsResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	keys, err := api.db.ListObjectStorePipelineKeys(api.db, request.NamespaceId, request.PipelineId)
	if err != nil {
		return &proto.ListPipelineObjectsResponse{}, err
	}

	var protoKeys []*proto.ObjectStoreKey
	for _, keyRaw := range keys {
		var key models.ObjectStoreKey
		key.Key = keyRaw.Key
		key.Created = keyRaw.Created
		protoKeys = append(protoKeys, key.ToProto())
	}

	return &proto.ListPipelineObjectsResponse{
		Keys: protoKeys,
	}, nil
}

func (api *API) GetPipelineObject(ctx context.Context, request *proto.GetPipelineObjectRequest) (*proto.GetPipelineObjectResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetPipelineObjectResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

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
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.PutPipelineObjectResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.PutPipelineObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	evictedObject, err := api.addPipelineObject(request.NamespaceId, request.PipelineId, request.Key,
		request.Content, request.Force)
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
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeletePipelineObjectResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.objectStore.DeleteObject(pipelineObjectKey(request.NamespaceId, request.PipelineId, request.Key))
	if err != nil {
		return &proto.DeletePipelineObjectResponse{}, status.Error(codes.Internal, fmt.Sprintf("could not delete object %q; %v", request.Key, err))
	}

	return &proto.DeletePipelineObjectResponse{}, nil
}

func (api *API) ListRunObjects(ctx context.Context, request *proto.ListRunObjectsRequest) (*proto.ListRunObjectsResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListRunObjectsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.ListRunObjectsResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	keys, err := api.db.ListObjectStoreRunKeys(api.db, request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		return &proto.ListRunObjectsResponse{}, err
	}

	var protoKeys []*proto.ObjectStoreKey
	for _, keyRaw := range keys {
		var key models.ObjectStoreKey
		key.Key = keyRaw.Key
		key.Created = keyRaw.Created
		protoKeys = append(protoKeys, key.ToProto())
	}

	return &proto.ListRunObjectsResponse{
		Keys: protoKeys,
	}, nil
}

func (api *API) GetRunObject(ctx context.Context, request *proto.GetRunObjectRequest) (*proto.GetRunObjectResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetRunObjectResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

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
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.PutRunObjectResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.PutRunObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	newObjectKey := models.NewObjectStoreKey(request.Key)

	err = api.db.InsertObjectStoreRunKey(api.db, &storage.ObjectStoreRunKey{
		Namespace: namespace,
		Pipeline:  request.PipelineId,
		Run:       request.RunId,
		Key:       newObjectKey.Key,
		Created:   newObjectKey.Created,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.PutRunObjectResponse{},
				status.Error(codes.AlreadyExists, "key already exists")
		}

		return &proto.PutRunObjectResponse{},
			status.Error(codes.Internal, "could not insert key")
	}

	err = api.objectStore.PutObject(runObjectKey(request.NamespaceId, request.PipelineId, request.RunId, request.Key), request.Content, request.Force)
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
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeleteRunObjectResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeleteRunObjectResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.objectStore.DeleteObject(runObjectKey(request.NamespaceId, request.PipelineId, request.RunId, request.Key))
	if err != nil {
		return nil, err
	}

	return &proto.DeleteRunObjectResponse{}, nil
}
