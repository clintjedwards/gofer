package api

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"fmt"
	"net/http"
	"regexp"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/danielgtaylor/huma/v2"
	"github.com/rs/zerolog/log"
)

// We create custom types because context is a mess: https://www.calhoun.io/pitfalls-of-context-values-and-how-to-avoid-or-mitigate-them/
// Specifically the types are to prevent cross-contamination between context values from other upstream sources.
// For instance if something like GRPC introduces a new context value that conflicts with out context value then there
// would be no way to tell those two types apart other than using type inferences. Causing weird bugs.
type goferContextKey string

var (
	contextUserNamespaces = goferContextKey("namespaces")
	contextUserKind       = goferContextKey("kind")
)

var authlessEndpoints = []string{
	"/api/system/summary",
	"/api/system/info",
	"/api/tokens/bootstrap",
}

func generateToken(length int) string {
	b := make([]byte, length)
	_, _ = rand.Read(b)
	return fmt.Sprintf("%x", b)
}

// createNewAPIToken creates returns the new token and its hash.
func (apictx *APIContext) createNewAPIToken() (token string, hash string) {
	token = generateToken(32)

	hasher := sha256.New()
	hasher.Write([]byte(token))
	hash = fmt.Sprintf("%x", hasher.Sum(nil))

	return
}

func (apictx *APIContext) getAPIToken(token string) (*models.Token, error) {
	hash := getHash(token)
	tokenDetailsRaw, err := apictx.db.GetTokenByHash(apictx.db, hash)
	if err != nil {
		return nil, err
	}

	var tokenDetails models.Token
	tokenDetails.FromStorage(&tokenDetailsRaw)

	return &tokenDetails, nil
}

func getHash(token string) string {
	hasher := sha256.New()
	hasher.Write([]byte(token))
	return fmt.Sprintf("%x", hasher.Sum(nil))
}

func authMiddleware(apictx *APIContext, api huma.API) func(ctx huma.Context, next func(huma.Context)) {
	return func(ctx huma.Context, next func(huma.Context)) {
		currentEndpoint := ctx.Operation().Path

		// Exclude routes that don't need authentication
		for _, endpoint := range authlessEndpoints {
			if currentEndpoint == endpoint {
				next(ctx)
				return
			}
		}

		// If server is in DevMode give context fake admin values
		if apictx.config.Development.BypassAuth {
			ctx = huma.WithValue(ctx, contextUserNamespaces, []string{})
			ctx = huma.WithValue(ctx, contextUserKind, string(models.TokenTypeManagement))

			next(ctx)
			return
		}

		token := strings.TrimPrefix(ctx.Header("Authorization"), "Bearer ")
		if len(token) == 0 {
			err := huma.WriteErr(api, ctx, http.StatusUnauthorized, "Unauthorized; Token missing")
			if err != nil {
				log.Error().Err(err).Msg("Could not properly write error")
			}
			return
		}

		storedToken, err := apictx.getAPIToken(token)
		if err != nil {
			err = huma.WriteErr(api, ctx, http.StatusUnauthorized, "Unauthorized")
			if err != nil {
				log.Error().Err(err).Msg("Could not properly write error")
			}
			return
		}

		now := time.Now().UnixMilli()
		if uint64(now) >= storedToken.Expires {
			err = huma.WriteErr(api, ctx, http.StatusUnauthorized, "Unauthorized; Token expired")
			if err != nil {
				log.Error().Err(err).Msg("Could not properly write error")
			}
			return
		}

		if storedToken.Disabled {
			err = huma.WriteErr(api, ctx, http.StatusUnauthorized, "Unauthorized; Token disabled")
			if err != nil {
				log.Error().Err(err).Msg("Could not properly write error")
			}
			return
		}

		ctx = huma.WithValue(ctx, contextUserNamespaces, storedToken.Namespaces)
		ctx = huma.WithValue(ctx, contextUserKind, string(storedToken.TokenType))

		next(ctx)
	}
}

// hasAccess is a convenience function for common routes that checks first for management key and then
// if the namespace is valid.
func hasAccess(ctx context.Context, namespace string) bool {
	if isManagementUser(ctx) {
		return true
	}

	return hasNamespaceAccess(ctx, namespace)
}

// Attempts to match on the namespaces that the user's token has access to if they have access to the provided
// namespace.
func hasNamespaceAccess(ctx context.Context, namespace string) bool {
	namespaces, present := ctx.Value(contextUserNamespaces).([]string)
	if !present {
		log.Error().Msg("namespace field missing from context in request")
		return false
	}

	for _, namespaceFilter := range namespaces {
		// Check if the string is a valid regex
		isRegex := false
		_, err := regexp.Compile(namespaceFilter)
		if err == nil {
			isRegex = true
		}

		if isRegex {
			matched, err := regexp.MatchString(namespaceFilter, namespace)
			if err != nil {
				log.Err(err).Msg("Could not match regex during check auth namespaces")
				return false
			}

			if !matched {
				return false
			}

			return true
		}

		if namespaceFilter == namespace {
			return true
		}
	}

	return false
}

func isManagementUser(ctx context.Context) bool {
	kind, present := ctx.Value(contextUserKind).(string)
	if !present {
		log.Error().Msg("kind field missing from context in request")
		return false
	}

	return strings.EqualFold(kind, string(models.TokenTypeManagement))
}
