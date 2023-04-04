package models

import (
	"encoding/json"
	"regexp"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

type SecretStoreKey struct {
	Key        string   `json:"key"`
	Namespaces []string `json:"namespaces"`
	Created    int64    `json:"created"`
}

func NewSecretStoreKey(key string, namespaces []string) *SecretStoreKey {
	return &SecretStoreKey{
		Key:        key,
		Namespaces: namespaces,
		Created:    time.Now().UnixMilli(),
	}
}

// Checks a global secret key's namespace list to confirm it actually does match a given namespace.
// It loops through the namespaces list and tries to evaluate regexps when it can.
func (s *SecretStoreKey) IsAllowedNamespace(namespace string) bool {
	for _, namespaceFilter := range s.Namespaces {
		// Check if the string is a valid regex
		isRegex := false
		_, err := regexp.Compile(namespaceFilter)
		if err == nil {
			isRegex = true
		}

		if isRegex {
			matched, err := regexp.MatchString(namespaceFilter, namespace)
			if err != nil {
				log.Err(err).Msg("Could not match regex during check global secret namespaces")
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

func (s *SecretStoreKey) ToProto() *proto.SecretStoreKey {
	return &proto.SecretStoreKey{
		Key:        s.Key,
		Namespaces: s.Namespaces,
		Created:    s.Created,
	}
}

func (s *SecretStoreKey) FromGlobalSecretKeyStorage(sn *storage.SecretStoreGlobalKey) {
	namespaces := []string{}
	err := json.Unmarshal([]byte(sn.Namespaces), &namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	s.Key = sn.Key
	s.Namespaces = namespaces
	s.Created = sn.Created
}

func (s *SecretStoreKey) ToGlobalSecretKeyStorage() *storage.SecretStoreGlobalKey {
	namespacesRaw, err := json.Marshal(s.Namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.SecretStoreGlobalKey{
		Key:        s.Key,
		Namespaces: string(namespacesRaw),
		Created:    s.Created,
	}
}
