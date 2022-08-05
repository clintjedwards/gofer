package api

import (
	"context"
	"errors"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// GetSystemInfo returns system information and health
func (api *API) GetSystemInfo(context context.Context, request *proto.GetSystemInfoRequest) (*proto.GetSystemInfoResponse, error) {
	version, commit := parseVersion(appVersion)

	return &proto.GetSystemInfoResponse{
		Commit:         commit,
		DevModeEnabled: api.config.Server.DevMode,
		Semver:         version,
	}, nil
}

func (api *API) ToggleEventIngress(ctx context.Context, request *proto.ToggleEventIngressRequest) (*proto.ToggleEventIngressResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.ToggleEventIngressResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if !api.ignorePipelineRunEvents.CompareAndSwap(false, true) {
		api.ignorePipelineRunEvents.Store(false)
	}

	log.Info().Bool("ignore_pipeline_run_events", api.ignorePipelineRunEvents.Load()).Msg("toggled event ingress")
	return &proto.ToggleEventIngressResponse{
		Value: api.ignorePipelineRunEvents.Load(),
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
		_, err := api.db.GetNamespace(namespace)
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

	err := api.db.InsertToken(newToken)
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
	token, err := api.db.GetToken(hash)
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
	err := api.db.DeleteToken(hash)
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.DeleteTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.DeleteTokenResponse{}, nil
}

func (api *API) BootstrapToken(ctx context.Context, request *proto.BootstrapTokenRequest) (*proto.BootstrapTokenResponse, error) {
	tokens, err := api.db.ListTokens(0, 1)
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

	err = api.db.InsertToken(newToken)
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.BootstrapTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.BootstrapTokenResponse{
		Details: newToken.ToProto(),
		Token:   token,
	}, nil
}
