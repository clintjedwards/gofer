package api

import (
	"context"

	"github.com/rs/zerolog/log"
)

const (
	namespaceDefaultID   string = "default"
	namespaceDefaultName string = "Default"
)

// determineNamespace determines a default namespace from the user's token. This is used if the user hasn't specifically
// specified a namespace in the request.
//
// If the user's token has a single namespace it returns that namespace.
// If the user's token has no namespaces it returns the default namespace.
// If the user's token has multiple namespaces it returns the first namespace that isn't the default namespace.
func determineNamespace(ctx context.Context) string {
	namespaces, present := ctx.Value(contextUserNamespaces).([]string)
	if !present {
		log.Error().Msg("namespace field missing from context in request")
		return namespaceDefaultID
	}

	if len(namespaces) == 0 {
		return namespaceDefaultID
	}

	if len(namespaces) == 1 {
		return namespaces[0]
	}

	for _, namespace := range namespaces {
		if namespace != namespaceDefaultID {
			return namespace
		}
	}

	return namespaceDefaultID
}
