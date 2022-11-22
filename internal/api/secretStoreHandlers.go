package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/secretStore"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetPipelineSecret(ctx context.Context, request *proto.GetPipelineSecretRequest) (*proto.GetPipelineSecretResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.GetPipelineSecretResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if request.Key == "" {
		return nil, status.Error(codes.FailedPrecondition, "key cannot be empty")
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.GetPipelineSecretResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	metadata, err := api.db.GetSecretStorePipelineKey(request.NamespaceId, request.PipelineId, request.Key)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetPipelineSecretResponse{}, status.Error(codes.FailedPrecondition, "key not found")
		}
		return &proto.GetPipelineSecretResponse{}, status.Error(codes.Internal, "failed to retrieve key from database")
	}

	var secret string

	if request.IncludeSecret {
		secret, err = api.secretStore.GetSecret(pipelineSecretKey(request.NamespaceId, request.PipelineId, request.Key))
		if err != nil {
			if errors.Is(err, secretStore.ErrEntityNotFound) {
				return &proto.GetPipelineSecretResponse{}, status.Error(codes.FailedPrecondition, "key not found")
			}
			return &proto.GetPipelineSecretResponse{}, status.Error(codes.Internal, "failed to retrieve key from database")
		}
	}

	return &proto.GetPipelineSecretResponse{
		Metadata: metadata.ToProto(),
		Secret:   secret,
	}, nil
}

func (api *API) ListPipelineSecrets(ctx context.Context, request *proto.ListPipelineSecretsRequest) (*proto.ListPipelineSecretsResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.ListPipelineSecretsResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.ListPipelineSecretsResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	keys, err := api.db.ListSecretStorePipelineKeys(request.NamespaceId, request.PipelineId)
	if err != nil {
		return &proto.ListPipelineSecretsResponse{}, err
	}

	var protoKeys []*proto.SecretStoreKey
	for _, key := range keys {
		protoKeys = append(protoKeys, key.ToProto())
	}

	return &proto.ListPipelineSecretsResponse{
		Keys: protoKeys,
	}, nil
}

func (api *API) PutPipelineSecret(ctx context.Context, request *proto.PutPipelineSecretRequest) (*proto.PutPipelineSecretResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.PutPipelineSecretResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if request.Key == "" {
		return nil, status.Error(codes.FailedPrecondition, "key cannot be empty")
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.PutPipelineSecretResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	newSecretKey := models.NewSecretStoreKey(request.Key)

	err = api.db.InsertSecretStorePipelineKey(request.NamespaceId, request.PipelineId, newSecretKey, request.Force)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.PutPipelineSecretResponse{},
				status.Error(codes.AlreadyExists, "key already exists")
		}

		return &proto.PutPipelineSecretResponse{},
			status.Error(codes.Internal, "could not insert key")
	}

	err = api.secretStore.PutSecret(pipelineSecretKey(request.NamespaceId, request.PipelineId, request.Key), request.Content, request.Force)
	if err != nil {
		if errors.Is(err, secretStore.ErrEntityExists) {
			return &proto.PutPipelineSecretResponse{},
				status.Error(codes.AlreadyExists, "key already exists")
		}

		return &proto.PutPipelineSecretResponse{},
			status.Error(codes.Internal, "could not insert key")
	}

	return &proto.PutPipelineSecretResponse{
		Bytes: int64(len(request.Content)),
	}, nil
}

func (api *API) DeletePipelineSecret(ctx context.Context, request *proto.DeletePipelineSecretRequest) (*proto.DeletePipelineSecretResponse, error) {
	namespace, err := api.resolveNamespace(ctx, request.NamespaceId)
	if err != nil {
		return &proto.DeletePipelineSecretResponse{},
			status.Errorf(codes.FailedPrecondition, "error retrieving namespace %q; %v", request.NamespaceId, err.Error())
	}

	request.NamespaceId = namespace

	if request.Key == "" {
		return nil, status.Error(codes.FailedPrecondition, "key cannot be empty")
	}

	if !hasAccess(ctx, request.NamespaceId) {
		return &proto.DeletePipelineSecretResponse{}, status.Error(codes.PermissionDenied, "access denied")
	}

	err = api.db.DeleteSecretStorePipelineKey(request.NamespaceId, request.PipelineId, request.Key)
	if err != nil {
		return &proto.DeletePipelineSecretResponse{}, err
	}

	err = api.secretStore.DeleteSecret(pipelineSecretKey(request.NamespaceId, request.PipelineId, request.Key))
	if err != nil {
		return &proto.DeletePipelineSecretResponse{}, err
	}

	return &proto.DeletePipelineSecretResponse{}, nil
}

func (api *API) GetGlobalSecret(ctx context.Context, request *proto.GetGlobalSecretRequest) (*proto.GetGlobalSecretResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Key == "" {
		return &proto.GetGlobalSecretResponse{}, status.Error(codes.FailedPrecondition, "key cannot be empty")
	}

	metadata, err := api.db.GetSecretStoreGlobalKey(request.Key)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetGlobalSecretResponse{}, status.Error(codes.FailedPrecondition, "key not found")
		}
		return &proto.GetGlobalSecretResponse{}, status.Error(codes.Internal, "failed to retrieve key from database")
	}

	var secret string

	if request.IncludeSecret {
		secret, err = api.secretStore.GetSecret(globalSecretKey(request.Key))
		if err != nil {
			if errors.Is(err, secretStore.ErrEntityNotFound) {
				return &proto.GetGlobalSecretResponse{}, status.Error(codes.FailedPrecondition, "key not found")
			}
			return &proto.GetGlobalSecretResponse{}, status.Error(codes.Internal, "failed to retrieve key from database")
		}
	}

	return &proto.GetGlobalSecretResponse{
		Metadata: metadata.ToProto(),
		Secret:   secret,
	}, nil
}

func (api *API) ListGlobalSecrets(ctx context.Context, request *proto.ListGlobalSecretsRequest) (*proto.ListGlobalSecretsResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	keys, err := api.db.ListSecretStoreGlobalKeys()
	if err != nil {
		return &proto.ListGlobalSecretsResponse{}, err
	}

	var protoKeys []*proto.SecretStoreKey
	for _, key := range keys {
		protoKeys = append(protoKeys, key.ToProto())
	}

	return &proto.ListGlobalSecretsResponse{
		Keys: protoKeys,
	}, nil
}

func (api *API) PutGlobalSecret(ctx context.Context, request *proto.PutGlobalSecretRequest) (*proto.PutGlobalSecretResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Key == "" {
		return nil, status.Error(codes.FailedPrecondition, "key cannot be empty")
	}

	newSecretKey := models.NewSecretStoreKey(request.Key)

	err := api.db.InsertSecretStoreGlobalKey(newSecretKey, request.Force)
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.PutGlobalSecretResponse{},
				status.Error(codes.AlreadyExists, "key already exists")
		}

		return &proto.PutGlobalSecretResponse{},
			status.Error(codes.Internal, "could not insert key")
	}

	err = api.secretStore.PutSecret(globalSecretKey(request.Key), request.Content, request.Force)
	if err != nil {
		if errors.Is(err, secretStore.ErrEntityExists) {
			return &proto.PutGlobalSecretResponse{},
				status.Error(codes.AlreadyExists, "key already exists")
		}

		return &proto.PutGlobalSecretResponse{},
			status.Error(codes.Internal, "could not insert key")
	}

	return &proto.PutGlobalSecretResponse{
		Bytes: int64(len(request.Content)),
	}, nil
}

func (api *API) DeleteGlobalSecret(ctx context.Context, request *proto.DeleteGlobalSecretRequest) (*proto.DeleteGlobalSecretResponse, error) {
	if !isManagementUser(ctx) {
		return nil, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Key == "" {
		return nil, status.Error(codes.FailedPrecondition, "key cannot be empty")
	}

	err := api.db.DeleteSecretStoreGlobalKey(request.Key)
	if err != nil {
		return &proto.DeleteGlobalSecretResponse{}, err
	}

	err = api.secretStore.DeleteSecret(globalSecretKey(request.Key))
	if err != nil {
		return &proto.DeleteGlobalSecretResponse{}, err
	}

	return &proto.DeleteGlobalSecretResponse{}, nil
}
