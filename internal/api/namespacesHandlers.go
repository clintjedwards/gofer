package api

import (
	"context"
	"errors"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (api *API) GetNamespace(ctx context.Context, request *proto.GetNamespaceRequest) (*proto.GetNamespaceResponse, error) {
	if request.Id == "" {
		return &proto.GetNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	namespaceRaw, err := api.db.GetNamespace(api.db, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetNamespaceResponse{}, status.Error(codes.FailedPrecondition, "namespace not found")
		}
		log.Error().Err(err).Msg("could not get namespace")
		return &proto.GetNamespaceResponse{}, status.Error(codes.Internal, "failed to retrieve namespace from database")
	}

	var namespace models.Namespace
	namespace.FromStorage(&namespaceRaw)

	return &proto.GetNamespaceResponse{Namespace: namespace.ToProto()}, nil
}

func (api *API) ListNamespaces(ctx context.Context, request *proto.ListNamespacesRequest) (*proto.ListNamespacesResponse, error) {
	namespaces, err := api.db.ListNamespaces(api.db, int(request.Offset), int(request.Limit))
	if err != nil {
		log.Error().Err(err).Msg("could not get namespaces")
		return &proto.ListNamespacesResponse{}, status.Error(codes.Internal, "failed to retrieve namespaces from database")
	}

	protoNamespaces := []*proto.Namespace{}
	for _, namespaceRaw := range namespaces {
		var namespace models.Namespace
		namespace.FromStorage(&namespaceRaw)
		protoNamespaces = append(protoNamespaces, namespace.ToProto())
	}

	return &proto.ListNamespacesResponse{
		Namespaces: protoNamespaces,
	}, nil
}

func (api *API) CreateNamespace(ctx context.Context, request *proto.CreateNamespaceRequest) (*proto.CreateNamespaceResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.CreateNamespaceResponse{},
			status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Id == "" {
		return &proto.CreateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	if request.Name == "" {
		return &proto.CreateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "name required")
	}

	// Prevent users from getting global secrets by naming their pipelines particular ways.
	if strings.EqualFold(request.Id, "global_secret") {
		return &proto.CreateNamespaceResponse{},
			status.Error(codes.FailedPrecondition, "namespace cannot be named global_secret")
	}

	newNamespace := models.NewNamespace(request.Id, request.Name, request.Description)

	err := api.db.InsertNamespace(api.db, newNamespace.ToStorage())
	if err != nil {
		if errors.Is(err, storage.ErrEntityExists) {
			return &proto.CreateNamespaceResponse{},
				status.Error(codes.AlreadyExists, "namespace already exists")
		}

		return &proto.CreateNamespaceResponse{},
			status.Error(codes.Internal, "could not insert namespace")
	}

	go api.events.Publish(models.EventCreatedNamespace{
		NamespaceID: newNamespace.ID,
	})

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

	err := api.db.UpdateNamespace(api.db, request.Id, storage.UpdatableNamespaceFields{
		Name:        &request.Name,
		Description: &request.Description,
		Modified:    ptr(time.Now().UnixMilli()),
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.UpdateNamespaceResponse{}, status.Error(codes.FailedPrecondition, "could not find namespace")
		}
		return &proto.UpdateNamespaceResponse{}, err
	}

	log.Info().Interface("namespace", request.Id).Msg("updated namespace")
	return &proto.UpdateNamespaceResponse{}, nil
}

func (api *API) DeleteNamespace(ctx context.Context, request *proto.DeleteNamespaceRequest) (*proto.DeleteNamespaceResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.DeleteNamespaceResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Id == "" {
		return &proto.DeleteNamespaceResponse{}, status.Error(codes.FailedPrecondition, "id required")
	}

	err := api.db.DeleteNamespace(api.db, request.Id)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DeleteNamespaceResponse{}, status.Error(codes.FailedPrecondition, "could not find namespace")
		}
		return &proto.DeleteNamespaceResponse{}, err
	}

	go api.events.Publish(models.EventDeletedNamespace{
		NamespaceID: request.Id,
	})

	return &proto.DeleteNamespaceResponse{}, nil
}
