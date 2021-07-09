package api

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
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

var authlessMethods = []string{
	"/proto.Gofer/BootstrapToken",
	"/proto.Gofer/GetSystemInfo",
}

func generateToken(length int) string {
	b := make([]byte, length)
	_, _ = rand.Read(b)
	return fmt.Sprintf("%x", b)
}

// createNewAPIToken creates returns the new token and its hash.
func (api *API) createNewAPIToken() (token string, hash string) {
	token = generateToken(32)

	hasher := sha256.New()
	hasher.Write([]byte(token))
	hash = fmt.Sprintf("%x", hasher.Sum(nil))

	return
}

func (api *API) getAPIToken(token string) (*models.Token, error) {
	hash := getHash(token)
	tokenDetails, err := api.storage.GetToken(storage.GetTokenRequest{
		Hash: hash,
	})
	if err != nil {
		return nil, err
	}
	return tokenDetails, nil
}

func getHash(token string) string {
	hasher := sha256.New()
	hasher.Write([]byte(token))
	return fmt.Sprintf("%x", hasher.Sum(nil))
}

// authenticate is run on every call to verify if the user is allowed to access a given RPC
func (api *API) authenticate(ctx context.Context) (context.Context, error) {
	method, _ := grpc.Method(ctx)

	// Exclude routes that don't need authentication
	for _, route := range authlessMethods {
		if method == route {
			return ctx, nil
		}
	}

	// If server is in DevMode give context fake admin values
	if api.config.Server.DevMode {
		ctxNamespaces := context.WithValue(ctx, contextUserNamespaces, []string{})
		ctxKind := context.WithValue(ctxNamespaces, contextUserKind, string(models.TokenKindManagement))

		return ctxKind, nil
	}

	token, err := grpc_auth.AuthFromMD(ctx, "Bearer")
	if err != nil {
		return ctx, status.Error(codes.PermissionDenied, "malformed token fmt; should be in form: 'Bearer <token>'")
	}

	storedToken, err := api.getAPIToken(token)
	if err != nil {
		return ctx, status.Error(codes.PermissionDenied, "access denied")
	}

	ctxNamespaces := context.WithValue(ctx, contextUserNamespaces, storedToken.Namespaces)
	ctxKind := context.WithValue(ctxNamespaces, contextUserKind, string(storedToken.Kind))

	return ctxKind, nil
}

// hasAccess is a convenience function for common routes that checks first for management key and then
// if the namespace is valid.
func hasAccess(ctx context.Context, namespace string) bool {
	if isManagementUser(ctx) {
		return true
	}

	return hasNamespaceAccess(ctx, namespace)
}

func hasNamespaceAccess(ctx context.Context, namespace string) bool {
	namespaces, present := ctx.Value(contextUserNamespaces).([]string)
	if !present {
		log.Error().Msg("namespace field missing from context in request")
		return false
	}
	for _, space := range namespaces {
		if namespace == space {
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

	return strings.EqualFold(kind, string(models.TokenKindManagement))
}
