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

// GetSystemInfo returns system information and health
func (api *API) GetSystemInfo(_ context.Context, _ *proto.GetSystemInfoRequest) (*proto.GetSystemInfoResponse, error) {
	version, commit := parseVersion(appVersion)

	return &proto.GetSystemInfoResponse{
		Commit: commit,
		Semver: version,
	}, nil
}

func (api *API) ToggleEventIngress(ctx context.Context, _ *proto.ToggleEventIngressRequest) (*proto.ToggleEventIngressResponse, error) {
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

	if request.Expires == "" {
		return &proto.CreateTokenResponse{}, status.Error(codes.FailedPrecondition, "requires expiration duration")
	}

	expires, err := time.ParseDuration(request.Expires)
	if err != nil {
		return &proto.CreateTokenResponse{}, status.Errorf(codes.FailedPrecondition, "could not parse duration: %v", err)
	}

	token, hash := api.createNewAPIToken()

	for _, namespace := range request.Namespaces {
		_, err := api.db.GetNamespace(api.db, namespace)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return &proto.CreateTokenResponse{},
					status.Errorf(codes.NotFound, "namespace %q not found", namespace)
			}
			return &proto.CreateTokenResponse{},
				status.Errorf(codes.Internal, "could not create token: %v", err)
		}
	}

	kind := models.TokenKindClient

	if strings.EqualFold(request.Kind.String(), "management") {
		kind = models.TokenKindManagement
	}

	newToken := models.NewToken(hash, kind, request.Namespaces, request.Metadata, expires)

	_, err = api.db.InsertToken(api.db, newToken.ToStorage())
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.CreateTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.CreateTokenResponse{
		Details: newToken.ToProto(),
		Token:   token,
	}, nil
}

func contains(s []string, e string) bool {
	for _, a := range s {
		if a == e {
			return true
		}
	}
	return false
}

func (api *API) ListTokens(ctx context.Context, request *proto.ListTokensRequest) (*proto.ListTokensResponse, error) {
	if request.Namespace == "" {
		request.Namespace = determineNamespace(ctx)
	}

	tokenList := []*proto.Token{}

	tokensRaw, err := api.db.ListTokens(api.db, 0, 0)
	if err != nil {
		log.Error().Err(err).Msg("could not get token")
		return &proto.ListTokensResponse{}, status.Error(codes.Internal, "failed to retrieve token from database")
	}

	var tokens []models.Token
	for _, tokenRaw := range tokensRaw {
		var token models.Token
		token.FromStorage(&tokenRaw)
		tokens = append(tokens, token)
	}

	for _, token := range tokens {
		// If the token has namespaces AND the token does not contain the targeted namespace skip it.
		if len(token.Namespaces) != 0 && !contains(token.Namespaces, request.Namespace) {
			continue
		}

		// If the token is a management token, but the request is not made by a management key, skip it.
		if !isManagementUser(ctx) && token.Kind == models.TokenKindManagement {
			continue
		}

		// Otherwise just add the token.
		tokenList = append(tokenList, token.ToProto())
	}

	return &proto.ListTokensResponse{
		Tokens: tokenList,
	}, nil
}

func (api *API) GetToken(_ context.Context, request *proto.GetTokenRequest) (*proto.GetTokenResponse, error) {
	if request.Token == "" {
		return &proto.GetTokenResponse{}, status.Error(codes.FailedPrecondition, "token required")
	}

	hash := getHash(request.Token)
	tokenRaw, err := api.db.GetTokenByHash(api.db, hash)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.GetTokenResponse{}, status.Error(codes.FailedPrecondition, "token not found")
		}
		log.Error().Err(err).Msg("could not get token")
		return &proto.GetTokenResponse{}, status.Error(codes.Internal, "failed to retrieve token from database")
	}

	var token models.Token
	token.FromStorage(&tokenRaw)

	return &proto.GetTokenResponse{
		Details: token.ToProto(),
	}, nil
}

func (api *API) EnableToken(ctx context.Context, request *proto.EnableTokenRequest) (*proto.EnableTokenResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.EnableTokenResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Token == "" {
		return &proto.EnableTokenResponse{}, status.Error(codes.FailedPrecondition, "token required")
	}

	hash := getHash(request.Token)
	err := api.db.EnableToken(api.db, hash)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.EnableTokenResponse{}, status.Error(codes.NotFound, "token not found")
		}
		log.Error().Err(err).Msg("could not get token from storage")
		return &proto.EnableTokenResponse{}, status.Error(codes.Internal, "could not get token")
	}

	return &proto.EnableTokenResponse{}, nil
}

func (api *API) DisableToken(ctx context.Context, request *proto.DisableTokenRequest) (*proto.DisableTokenResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.DisableTokenResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Token == "" {
		return &proto.DisableTokenResponse{}, status.Error(codes.FailedPrecondition, "token required")
	}

	hash := getHash(request.Token)
	err := api.db.DisableToken(api.db, hash)
	if err != nil {
		if errors.Is(err, storage.ErrEntityNotFound) {
			return &proto.DisableTokenResponse{}, status.Error(codes.NotFound, "token not found")
		}
		log.Error().Err(err).Msg("could not get token from storage")
		return &proto.DisableTokenResponse{}, status.Error(codes.Internal, "could not get token")
	}

	return &proto.DisableTokenResponse{}, nil
}

func (api *API) DeleteToken(ctx context.Context, request *proto.DeleteTokenRequest) (*proto.DeleteTokenResponse, error) {
	if !isManagementUser(ctx) {
		return &proto.DeleteTokenResponse{}, status.Error(codes.PermissionDenied, "management token required for this action")
	}

	if request.Token == "" {
		return &proto.DeleteTokenResponse{}, status.Error(codes.FailedPrecondition, "token required")
	}

	hash := getHash(request.Token)
	err := api.db.DeleteTokenByHash(api.db, hash)
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.DeleteTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.DeleteTokenResponse{}, nil
}

func (api *API) BootstrapToken(_ context.Context, _ *proto.BootstrapTokenRequest) (*proto.BootstrapTokenResponse, error) {
	tokens, err := api.db.ListTokens(api.db, 0, 0)
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.BootstrapTokenResponse{}, status.Errorf(codes.Internal, "could not create bootstrap token: %v", err)
	}

	// TODO(): This is hacky, it either needs a new token kind or needs a separate table so that we can identify, when
	// a bootstrap token has been created.
	//
	// Get rid of extension tokens when attempting to determine if a bootstrap token has already been created.
	prunedTokens := []storage.Token{}

	for _, token := range tokens {
		if !strings.Contains(token.Metadata, "extension_token") {
			prunedTokens = append(prunedTokens, token)
		}
	}

	if len(prunedTokens) != 0 {
		return &proto.BootstrapTokenResponse{}, status.Error(codes.FailedPrecondition, "bootstrap token already created")
	}

	token, hash := api.createNewAPIToken()
	newToken := models.NewToken(hash, models.TokenKindManagement, []string{}, map[string]string{
		"bootstrap_token": "true",
	}, time.Hour*876600)

	_, err = api.db.InsertToken(api.db, newToken.ToStorage())
	if err != nil {
		log.Error().Err(err).Msg("could not save token to storage")
		return &proto.BootstrapTokenResponse{}, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
	}

	return &proto.BootstrapTokenResponse{
		Details: newToken.ToProto(),
		Token:   token,
	}, nil
}
