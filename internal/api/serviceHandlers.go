package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/proto"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// GetSystemInfo returns system information and health
func (api *API) GetSystemInfo(context context.Context, request *proto.GetSystemInfoRequest) (*proto.GetSystemInfoResponse, error) {
	version, commit, buildTime := parseVersion(appVersion)

	return &proto.GetSystemInfoResponse{
		BuildTime:       buildTime,
		Commit:          commit,
		DevmodeEnabled:  api.config.Server.DevMode,
		FrontendEnabled: false,
		Version:         version,
		AcceptNewEvents: api.acceptNewEvents,
	}, nil
}

func (api *API) ToggleEventIngress(ctx context.Context, request *proto.ToggleEventIngressRequest) (*proto.ToggleEventIngressResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.ToggleEventIngressResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	api.acceptNewEvents = !api.acceptNewEvents
	log.Debug().Bool("accept_new_events", api.acceptNewEvents).Msg("toggled event ingress")
	return &proto.ToggleEventIngressResponse{
		Value: api.acceptNewEvents,
	}, nil
}

func (api *API) RepairOrphan(ctx context.Context, request *proto.RepairOrphanRequest) (*proto.RepairOrphanResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.RepairOrphanResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.PipelineId == "" {
		return &proto.RepairOrphanResponse{}, status.Error(codes.FailedPrecondition, "pipeline_id required")
	}

	if request.NamespaceId == "" {
		request.NamespaceId = determineNamespace(ctx)
	}

	err := api.repairOrphanRun(request.NamespaceId, request.PipelineId, request.RunId)
	if err != nil {
		return &proto.RepairOrphanResponse{}, status.Errorf(codes.Internal,
			"could not repair orphan run: %v", err)
	}

	return &proto.RepairOrphanResponse{}, nil
}

func (api *API) CreateToken(ctx context.Context, request *proto.CreateTokenRequest) (*proto.CreateTokenResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.CreateTokenResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	token, hash := api.createNewAPIToken()

	for _, namespace := range request.Namespaces {
		_, err := api.storage.GetNamespace(storage.GetNamespaceRequest{ID: namespace})
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return &proto.CreateTokenResponse{},
					status.Errorf(codes.NotFound, "namespace %q not found", namespace)
			}
			return &proto.CreateTokenResponse{},
				status.Errorf(codes.Internal, "could not create token: %v", err)
		}
	}

	newToken := models.NewToken(hash, models.TokenKind(request.Kind.String()), request.Namespaces, request.Metadata)

	err := api.storage.AddToken(storage.AddTokenRequest{
		Token: newToken,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.CreateTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.CreateTokenResponse{
		Details: newToken.ToProto(),
		Token:   token,
	}, nil
}

func (api *API) GetToken(ctx context.Context, request *proto.GetTokenRequest) (*proto.GetTokenResponse, error) {
	if request.Token == "" {
		return &proto.GetTokenResponse{}, status.Error(codes.FailedPrecondition, "token required")
	}

	hash := getHash(request.Token)
	token, err := api.storage.GetToken(storage.GetTokenRequest{
		Hash: hash,
	})
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetTokenResponse{}, status.Error(codes.FailedPrecondition, "token not found")
		}
		log.Error().Err(err).Msg("could not get token")
		return &proto.GetTokenResponse{}, status.Error(codes.Internal, "failed to retrieve token from database")
	}

	return &proto.GetTokenResponse{
		Details: token.ToProto(),
	}, nil
}

func (api *API) DeleteToken(ctx context.Context, request *proto.DeleteTokenRequest) (*proto.DeleteTokenResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.DeleteTokenResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Token == "" {
		return &proto.DeleteTokenResponse{}, status.Error(codes.FailedPrecondition, "token required")
	}

	hash := getHash(request.Token)
	err := api.storage.DeleteToken(storage.DeleteTokenRequest{
		Hash: hash,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.DeleteTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.DeleteTokenResponse{}, nil
}

func (api *API) BootstrapToken(ctx context.Context, request *proto.BootstrapTokenRequest) (*proto.BootstrapTokenResponse, error) {
	tokens, err := api.storage.GetAllTokens(storage.GetAllTokensRequest{
		Limit: 1,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.BootstrapTokenResponse{}, status.Errorf(codes.Internal, "could not create bootstrap token: %v", err)
	}

	if len(tokens) != 0 {
		return &proto.BootstrapTokenResponse{}, status.Error(codes.FailedPrecondition, "bootstrap token already created")
	}

	token, hash := api.createNewAPIToken()
	newToken := models.NewToken(hash, models.TokenKindManagement, []string{}, map[string]string{
		"bootstrap_token": "true",
	})

	err = api.storage.AddToken(storage.AddTokenRequest{
		Token: newToken,
	})
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.BootstrapTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.BootstrapTokenResponse{
		Details: newToken.ToProto(),
		Token:   token,
	}, nil
}
