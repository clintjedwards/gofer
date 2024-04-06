package models

import (
	"encoding/json"
	"fmt"
	"regexp"
	"strconv"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	"github.com/rs/zerolog/log"
)

type SecretStoreKey struct {
	Key        string   `json:"key"`
	Namespaces []string `json:"namespaces"`
	Created    uint64   `json:"created"`
}

func NewSecretStoreKey(key string, namespaces []string) *SecretStoreKey {
	return &SecretStoreKey{
		Key:        key,
		Namespaces: namespaces,
		Created:    uint64(time.Now().UnixMilli()),
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

func (s *SecretStoreKey) FromGlobalSecretKeyStorage(sn *storage.SecretStoreGlobalKey) {
	namespaces := []string{}
	err := json.Unmarshal([]byte(sn.Namespaces), &namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	created, err := strconv.ParseUint(sn.Created, 10, 64)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	s.Key = sn.Key
	s.Namespaces = namespaces
	s.Created = created
}

func (s *SecretStoreKey) ToGlobalSecretKeyStorage() *storage.SecretStoreGlobalKey {
	namespacesRaw, err := json.Marshal(s.Namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.SecretStoreGlobalKey{
		Key:        s.Key,
		Namespaces: string(namespacesRaw),
		Created:    fmt.Sprint(s.Created),
	}
}
