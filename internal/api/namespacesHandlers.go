package api

import (
	"context"
	"errors"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetNamespace(ctx context.Context, request *proto.GetNamespaceRequest) (*proto.GetNamespaceResponse, error) {
	if request.Id == "" {
		return &proto.GetNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespace, err := api.storage.GetNamespace(storage.GetNamespaceRequest{ID: request.Id})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetNamespaceResponse{}, status.Error(codes.FailedPrecondition, "namespace not found")
		}
		log.Error().Err(err).Msg("could not get namespace")
		return &proto.GetNamespaceResponse{}, status.Error(codes.Internal, "failed to retrieve namespace from database")
	}

	return &proto.GetNamespaceResponse{Namespace: namespace.ToProto()}, nil
}

func (api *API) ListNamespaces(ctx context.Context, request *proto.ListNamespacesRequest) (*proto.ListNamespacesResponse, error) {
	namespaces, err := api.storage.GetAllNamespaces(storage.GetAllNamespacesRequest{Offset: int(request.Offset), Limit: int(request.Limit)})
	if err != nil {
		log.Error().Err(err).Msg("could not get namespaces")
		return &proto.ListNamespacesResponse{}, status.Error(codes.Internal, "failed to retrieve namespaces from database")
	}

	protoNamespaces := []*proto.Namespace{}
	for _, namespace := range namespaces {
		protoNamespaces = append(protoNamespaces, namespace.ToProto())
	}

	return &proto.ListNamespacesResponse{
		Namespaces: protoNamespaces,
	}, nil
}

func (api *API) CreateNamespace(ctx context.Context, request *proto.CreateNamespaceRequest) (*proto.CreateNamespaceResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.CreateNamespaceResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Id == "" {
		return &proto.CreateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.Name == "" {
		return &proto.CreateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	newNamespace := models.NewNamespace(request.Id, request.Name, request.Description)

	err := api.storage.AddNamespace(storage.AddNamespaceRequest{
		Namespace: newNamespace,
	})
	if err != nil {
		return nil, err
	}

	log.Info().Interface("namespace", newNamespace).Msg("created new namespace")
	return &proto.CreateNamespaceResponse{
		Namespace: newNamespace.ToProto(),
	}, nil
}

func (api *API) UpdateNamespace(ctx context.Context, request *proto.UpdateNamespaceRequest) (*proto.UpdateNamespaceResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.UpdateNamespaceResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Id == "" {
		return &proto.UpdateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	// Get the old namespace first so that we can store the old values that we need before inserting
	// the new values from the content buffer.
	updatedNamespace, err := api.storage.GetNamespace(storage.GetNamespaceRequest{ID: request.Id})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.UpdateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "could not find namespace")
		}
		return &proto.UpdateNamespaceResponse{}, err
	}

	updatedNamespace.Name = request.Name
	updatedNamespace.Description = request.Description

	err = api.storage.UpdateNamespace(storage.UpdateNamespaceRequest{Namespace: updatedNamespace})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.UpdateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "could not find namespace")
		}
		return &proto.UpdateNamespaceResponse{}, err
	}

	log.Info().Interface("namespace", updatedNamespace).Msg("updated namespace")
	return &proto.UpdateNamespaceResponse{
		Namespace: updatedNamespace.ToProto(),
	}, nil
}

func (api *API) DeleteNamespace(ctx context.Context, request *proto.DeleteNamespaceRequest) (*proto.DeleteNamespaceResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.DeleteNamespaceResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Id == "" {
		return &proto.DeleteNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	updatedNamespace, err := api.storage.GetNamespace(storage.GetNamespaceRequest{ID: request.Id})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeleteNamespaceResponse{}, status.Error(codes.FailedPrecondition, "could not find namespace")
		}
		return &proto.DeleteNamespaceResponse{}, err
	}

	updatedNamespace.Deleted = time.Now().UnixMilli()

	err = api.storage.UpdateNamespace(storage.UpdateNamespaceRequest{Namespace: updatedNamespace})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeleteNamespaceResponse{}, status.Error(codes.FailedPrecondition, "could not find namespace")
		}
		return &proto.DeleteNamespaceResponse{}, err
	}

	log.Info().Interface("namespace", updatedNamespace).Msg("deleted namespace")
	return &proto.DeleteNamespaceResponse{}, nil
}
