package api

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/danielgtaylor/huma/v2"

	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type CreateTokenRequest struct {
	Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	Body struct {
		TokenType  models.TokenType  `json:"token_type" example:"CLIENT" doc:"The type of token to be created. Can be \"management\" or \"client\""`
		Namespaces []string          `json:"namespaces" example:"[\"default\", \"other_group\"]" doc:"The namespaces this token applies to; will be unauthorized at all other namespaces. This field can contain simple regexs"`
		Metadata   map[string]string `json:"metadata" example:"{\"created_by\": \"me\"}" doc:"Various other bits of data you can attach to tokens"`
		Expires    string            `json:"expires" example:"1h" doc:"The amount of time the token is valid for. Uses golang time duration strings: https://pkg.go.dev/time#ParseDuration"`
	}
}

type CreateTokenResponse struct {
	Body struct {
		TokenMetadata *models.Token `json:"token" doc:"Details about the created token"`
		Secret        string        `json:"secret" example:"secret_value" doc:"The secret value for the created token"`
	}
}

func (apictx *APIContext) registerCreateToken(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "CreateToken",
		Method:        http.MethodPost,
		Path:          "/api/tokens",
		Summary:       "Create new API token",
		DefaultStatus: http.StatusCreated,
		Description: "This endpoint is responsible for generating new API tokens. Tokens are essential for interacting " +
			"with Gofer's APIs, providing the necessary authentication for various operations. Management tokens can use " +
			"all admin routes and have no restrictions. Client tokens can only operate within their namespace and cannot " +
			"access admin routes.",
		Tags: []string{"Tokens"},
		// Handler //
	}, func(ctx context.Context, request *CreateTokenRequest) (*CreateTokenResponse, error) {
		if !isManagementUser(ctx) {
			return &CreateTokenResponse{}, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		if request.Body.Expires == "" {
			return &CreateTokenResponse{}, huma.NewError(http.StatusBadRequest, "requires expiration duration")
		}

		expires, err := time.ParseDuration(request.Body.Expires)
		if err != nil {
			return &CreateTokenResponse{}, huma.NewError(http.StatusBadRequest, "could not parse duration", err)
		}

		token, hash := apictx.createNewAPIToken()

		for _, namespace := range request.Body.Namespaces {
			_, err := apictx.db.GetNamespace(apictx.db, namespace)
			if err != nil {
				if errors.Is(err, storage.ErrEntityNotFound) {
					return &CreateTokenResponse{},
						huma.NewError(http.StatusNotFound, fmt.Sprintf("namespace %q not found", namespace))
				}
				return &CreateTokenResponse{},
					huma.NewError(http.StatusInternalServerError, "could not create token", err)
			}
		}

		kind := models.TokenTypeClient

		if strings.EqualFold(string(request.Body.TokenType), "management") {
			kind = models.TokenTypeManagement
		}

		newToken := models.NewToken(hash, kind, request.Body.Namespaces, request.Body.Metadata, expires)

		err = apictx.db.InsertToken(apictx.db, newToken.ToStorage())
		if err != nil {
			log.Error().Err(err).Msg("could not save token to storage")
			return &CreateTokenResponse{}, huma.NewError(http.StatusInternalServerError, "could not save token to storage", err)
		}
		resp := &CreateTokenResponse{}
		resp.Body.TokenMetadata = newToken
		resp.Body.Secret = token

		return resp, nil
	})
}

type ListTokensRequest struct {
	Auth      string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	Namespace string `query:"namespace" example:"my_namespace" default:"default" doc:"The unique identifier for the namespace to list the tokens for."`
}
type ListTokensResponse struct {
	Body struct {
		Tokens []*models.Token `json:"tokens" doc:"A list of tokens within this namespace"`
	}
}

func (apictx *APIContext) registerListTokens(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "ListTokens",
		Method:      http.MethodGet,
		Path:        "/api/tokens",
		Summary:     "List available API tokens",
		Description: "This endpoint lists all API tokens that are available within a specified namespace. ",
		Tags:        []string{"Tokens"},
		// Handler //
	}, func(ctx context.Context, request *ListTokensRequest) (*ListTokensResponse, error) {
		if !isManagementUser(ctx) {
			return nil, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		tokenList := []*models.Token{}

		tokensRaw, err := apictx.db.ListTokens(apictx.db, 0, 0)
		if err != nil {
			log.Error().Err(err).Msg("could not get token")
			return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve token from database")
		}

		for _, tokenRaw := range tokensRaw {
			token := models.Token{}
			token.FromStorage(&tokenRaw)

			// If the token has namespaces AND the token does not contain the targeted namespace skip it.
			if len(token.Namespaces) != 0 && !contains(token.Namespaces, request.Namespace) {
				continue
			}

			// If the token is a management token, but the request is not made by a management key, skip it.
			if !isManagementUser(ctx) && token.TokenType == models.TokenTypeManagement {
				continue
			}

			// Otherwise, prepare the token for the response.
			tokenMetadata := &token
			tokenList = append(tokenList, tokenMetadata)
		}

		resp := &ListTokensResponse{}
		resp.Body.Tokens = tokenList

		return resp, nil
	})
}

type DescribeTokenByIDRequest struct {
	Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	Body struct {
		TokenID string `json:"token_id" example:"token_id" doc:"The id of the token you want to look up."`
	}
}
type DescribeTokenByIDResponse struct {
	Body struct {
		TokenMetadata *models.Token `json:"token_metadata" doc:"Details about the token."`
	}
}

func (apictx *APIContext) registerDescribeTokenByID(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DescribeTokenByID",
		Method:      http.MethodGet,
		Path:        "/api/tokens/by-id",
		Summary:     "Describe a specific API token's details by it's ID",
		Description: "This endpoint fetches the details of a specific API token using its unique ID. ",
		Tags:        []string{"Tokens"},
		// Handler //
	}, func(_ context.Context, request *DescribeTokenByIDRequest) (*DescribeTokenByIDResponse, error) {
		if request.Body.TokenID == "" {
			return nil, huma.NewError(http.StatusBadRequest, "token id required")
		}

		tokenRaw, err := apictx.db.GetTokenByHash(apictx.db, request.Body.TokenID)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return nil, huma.NewError(http.StatusNotFound, "token not found")
			}
			log.Error().Err(err).Msg("could not get token")
			return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve token from database")
		}

		token := models.Token{}
		token.FromStorage(&tokenRaw)

		resp := &DescribeTokenByIDResponse{}
		resp.Body.TokenMetadata = &token

		return resp, nil
	})
}

type DescribeTokenByHashRequest struct {
	Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
	Body struct {
		TokenHash string `json:"token_hash" example:"some_token_hash" doc:"The hash of the token you want to look up."`
	}
}
type DescribeTokenByHashResponse struct {
	Body struct {
		TokenMetadata *models.Token `json:"token_metadata" doc:"Details about the token."`
	}
}

func (apictx *APIContext) registerDescribeTokenByHash(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DescribeTokenByHash",
		Method:      http.MethodGet,
		Path:        "/api/tokens/by-hash",
		Summary:     "Retrieve a specific API token's details",
		Description: "This endpoint fetches the details of a specific API token using its secret. ",
		Tags:        []string{"Tokens"},
		// Handler //
	}, func(_ context.Context, request *DescribeTokenByHashRequest) (*DescribeTokenByHashResponse, error) {
		if request.Body.TokenHash == "" {
			return nil, huma.NewError(http.StatusBadRequest, "token hash required")
		}

		hash := getHash(request.Body.TokenHash)
		tokenRaw, err := apictx.db.GetTokenByHash(apictx.db, hash)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return nil, huma.NewError(http.StatusNotFound, "token not found")
			}
			log.Error().Err(err).Msg("could not get token")
			return nil, huma.NewError(http.StatusInternalServerError, "failed to retrieve token from database")
		}

		token := models.Token{}
		token.FromStorage(&tokenRaw)

		resp := &DescribeTokenByHashResponse{}
		resp.Body.TokenMetadata = &token
		return resp, nil
	})
}

type (
	EnableTokenRequest struct {
		Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
		Body struct {
			TokenID string `json:"token_id" example:"a4e7L2" doc:"The unique identifier for the token you want to target"`
		}
	}
	EnableTokenResponse struct{}
)

func (apictx *APIContext) registerEnableToken(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "EnableToken",
		Method:      http.MethodPost,
		Path:        "/api/tokens/enable",
		Summary:     "Set disabled token to enabled",
		Description: "Tokens can be in two separate states either 'Enabled' or 'Disabled'. Disabled tokens cannot be used " +
			"within the Gofer API. This endpoint allows admins to take disabled tokens and make them enabled again.",
		Tags: []string{"Tokens"},
		// Handler //
	}, func(ctx context.Context, request *EnableTokenRequest) (*EnableTokenResponse, error) {
		if !isManagementUser(ctx) {
			return nil, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		if request.Body.TokenID == "" {
			return nil, huma.NewError(http.StatusBadRequest, "token ID required")
		}

		err := apictx.db.EnableToken(apictx.db, request.Body.TokenID)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return nil, huma.NewError(http.StatusNotFound, "token not found")
			}
			log.Error().Err(err).Msg("could not get token from storage")
			return nil, huma.NewError(http.StatusInternalServerError, "could not get token")
		}

		resp := &EnableTokenResponse{}

		return resp, nil
	})
}

type (
	DisableTokenRequest struct {
		Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
		Body struct {
			TokenID string `json:"token_id" example:"a4e7L2" doc:"The unique identifier for the token you want to target"`
		}
	}
	DisableTokenResponse struct{}
)

func (apictx *APIContext) registerDisableToken(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DisableToken",
		Method:      http.MethodPost,
		Path:        "/api/tokens/disable",
		Summary:     "Set enabled token to disabled",
		Description: "Tokens can be in two separate states either 'Enabled' or 'Disabled'. Disabled tokens cannot be used " +
			"within the Gofer API. This endpoint allows admins to take enabled tokens and make them disabled.",
		Tags: []string{"Tokens"},
		// Handler //
	}, func(ctx context.Context, request *DisableTokenRequest) (*DisableTokenResponse, error) {
		if !isManagementUser(ctx) {
			return nil, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		if request.Body.TokenID == "" {
			return nil, huma.NewError(http.StatusBadRequest, "token required")
		}

		err := apictx.db.DisableToken(apictx.db, request.Body.TokenID)
		if err != nil {
			if errors.Is(err, storage.ErrEntityNotFound) {
				return nil, huma.NewError(http.StatusNotFound, "token not found")
			}
			log.Error().Err(err).Msg("could not get token from storage")
			return nil, huma.NewError(http.StatusInternalServerError, "could not get token")
		}

		resp := &DisableTokenResponse{}

		return resp, nil
	})
}

type (
	DeleteTokenRequest struct {
		Auth string `header:"Authorization" example:"Bearer <your_api_token>" required:"true"`
		Body struct {
			TokenID string `json:"token_id" example:"a4e7L2" doc:"The unique identifier for the token you want to target"`
		}
	}
	DeleteTokenResponse struct{}
)

func (apictx *APIContext) registerDeleteToken(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID: "DeleteToken",
		Method:      http.MethodDelete,
		Path:        "/api/tokens/delete",
		Summary:     "Delete a specific token",
		Description: "Remove a stored token",
		Tags:        []string{"Tokens"},
		// Handler //
	}, func(ctx context.Context, request *DeleteTokenRequest) (*DeleteTokenResponse, error) {
		if !isManagementUser(ctx) {
			return nil, huma.NewError(http.StatusUnauthorized, "management token required for this action")
		}

		if request.Body.TokenID == "" {
			return nil, huma.NewError(http.StatusBadRequest, "token ID required")
		}

		err := apictx.db.DeleteTokenByID(apictx.db, request.Body.TokenID)
		if err != nil {
			log.Error().Err(err).Msg("could not save token to storage")
			return nil, huma.NewError(http.StatusInsufficientStorage, "could not save token to storage", err)
		}

		resp := &DeleteTokenResponse{}

		return resp, nil
	})
}

type (
	CreateBootstrapTokenRequest  struct{}
	CreateBootstrapTokenResponse struct {
		Body struct {
			TokenMetadata *models.Token `json:"token" doc:"Details about the created token"`
			Secret        string        `json:"secret" example:"secret_value" doc:"The secret value for the created token"`
		}
	}
)

func (apictx *APIContext) registerCreateBootstrapToken(apiDesc huma.API) {
	// Description //
	huma.Register(apiDesc, huma.Operation{
		OperationID:   "CreateBootstrapToken",
		Method:        http.MethodPost,
		Path:          "/api/tokens/bootstrap",
		Summary:       "Create original management token",
		DefaultStatus: http.StatusCreated,
		Description: "This endpoint is meant to be called on the first run of the Gofer application. It provides the " +
			"original management token (also referred to as the root or init token) that can create all future tokens. " +
			"This route can only be used once.",
		Tags: []string{"Tokens"},
		// Handler //
	}, func(_ context.Context, _ *CreateBootstrapTokenRequest) (*CreateBootstrapTokenResponse, error) {
		tokens, err := apictx.db.ListTokens(apictx.db, 0, 0)
		if err != nil {
			log.Error().Err(err).Msg("could not save token to storage")
			return nil, huma.NewError(http.StatusInternalServerError, "could not create bootstrap token", err)
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
			return nil, huma.NewError(http.StatusBadRequest, "bootstrap token already created")
		}

		token, hash := apictx.createNewAPIToken()
		newToken := models.NewToken(hash, models.TokenTypeManagement, []string{}, map[string]string{
			"bootstrap_token": "true",
		}, time.Hour*876600)

		err = apictx.db.InsertToken(apictx.db, newToken.ToStorage())
		if err != nil {
			log.Error().Err(err).Msg("could not save token to storage")
			return nil, status.Errorf(codes.Internal, "could not save token to storage: %v", err)
		}

		resp := &CreateBootstrapTokenResponse{}
		resp.Body.TokenMetadata = newToken
		resp.Body.Secret = token

		return resp, nil
	})
}
