package sdk

import (
	"context"
	"fmt"
	"net/http"

	"github.com/oapi-codegen/oapi-codegen/v2/pkg/securityprovider"
)

type GoferAPIVersion string

const (
	GoferAPIVersion0 GoferAPIVersion = "v0"
)

func (g GoferAPIVersion) goferAPIVersionInterceptor(_ context.Context, req *http.Request) error {
	req.Header.Add("gofer-api-version", string(g))

	return nil
}

// Creates a new client for the main Gofer API with all the required headers.
func NewClientWithHeaders(host, token string, apiVersion GoferAPIVersion) (*Client, error) {
	auth, err := securityprovider.NewSecurityProviderBearerToken(token)
	if err != nil {
		return nil, fmt.Errorf("could not establish Gofer client while attempting to create auth header: %w", err)
	}

	client, err := NewClient(host, WithRequestEditorFn(auth.Intercept),
		WithRequestEditorFn(apiVersion.goferAPIVersionInterceptor))
	if err != nil {
		return nil, fmt.Errorf("could not establish Gofer client: %w", err)
	}

	return client, nil
}
