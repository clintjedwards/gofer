package sdk

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
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
func NewClientWithHeaders(host, token string, useTLS bool, apiVersion GoferAPIVersion) (*Client, error) {
	scheme := "http://"

	if useTLS {
		scheme = "https://"
	}

	auth, err := securityprovider.NewSecurityProviderBearerToken(token)
	if err != nil {
		return nil, fmt.Errorf("could not establish Gofer client while attempting to create auth header: %w", err)
	}

	client, err := NewClient(scheme+host, WithRequestEditorFn(auth.Intercept),
		WithRequestEditorFn(apiVersion.goferAPIVersionInterceptor))
	if err != nil {
		return nil, fmt.Errorf("could not establish Gofer client: %w", err)
	}

	return client, nil
}

// List extension specific subscriptions.
func ListExtensionSubscriptions(extension_id, goferHost, secret string, useTLS bool, apiVersion GoferAPIVersion) ([]Subscription, error) {
	client, err := NewClientWithHeaders(goferHost, secret, useTLS, apiVersion)
	if err != nil {
		return nil, fmt.Errorf("could not establish Gofer client: %w", err)
	}

	resp, err := client.ListExtensionSubscriptions(context.Background(), extension_id)
	if err != nil {
		return nil, fmt.Errorf("could not query Gofer for extension subscriptions")
	}

	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode > 299 {
		return nil, fmt.Errorf("could not query Gofer for extension subscriptions; status_code: %d;", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("could not read response body while attempting to query for extension subscriptions")
	}

	listExtensionSubscriptionsResponse := ListExtensionSubscriptionsResponse{}
	if err := json.Unmarshal(body, &listExtensionSubscriptionsResponse); err != nil {
		return nil, fmt.Errorf("could not parse response body while attempting to query for extension subscriptions")
	}

	return listExtensionSubscriptionsResponse.Subscriptions, nil
}
