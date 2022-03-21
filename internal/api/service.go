package api

import (
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
)

var appVersion = "0.0.dev_000000"

func parseVersion(versionString string) (version, commit string) {
	version, commit, err := strings.Cut(versionString, "_")
	if !err {
		return "", ""
	}

	return
}

func (api *API) createNewAPIToken(kind models.TokenKind, namespaces []string, metadata map[string]string) (key string, token *models.Token, err error) {
	key, hash := api.generateNewAPIToken()

	newToken := models.NewToken(hash, kind, namespaces, metadata)

	err = api.storage.AddToken(storage.AddTokenRequest{
		Token: newToken,
	})
	if err != nil {
		return "", nil, fmt.Errorf("could not save token to storage: %v", err)
	}

	return key, newToken, nil
}
