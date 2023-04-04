package models

import (
	"encoding/json"
	"time"

	"github.com/clintjedwards/gofer/internal/storage"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/rs/zerolog/log"
)

type SecretStoreKey struct {
	Key            string   `json:"key"`
	Namespaces     []string `json:"namespaces"`
	ExtensionsOnly bool     `json:"extensions-only"`
	Created        int64    `json:"created"`
}

func NewSecretStoreKey(key string, namespaces []string, extensionsOnly bool) *SecretStoreKey {
	return &SecretStoreKey{
		Key:            key,
		Namespaces:     namespaces,
		ExtensionsOnly: extensionsOnly,
		Created:        time.Now().UnixMilli(),
	}
}

func (s *SecretStoreKey) ToProto() *proto.SecretStoreKey {
	return &proto.SecretStoreKey{
		Key:            s.Key,
		Namespaces:     s.Namespaces,
		ExtensionsOnly: s.ExtensionsOnly,
		Created:        s.Created,
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
	s.ExtensionsOnly = sn.ExtensionsOnly
	s.Created = sn.Created
}

func (s *SecretStoreKey) ToGlobalSecretKeyStorage() *storage.SecretStoreGlobalKey {
	namespacesRaw, err := json.Marshal(s.Namespaces)
	if err != nil {
		log.Fatal().Err(err).Msg("error in translating from storage")
	}

	return &storage.SecretStoreGlobalKey{
		Key:            s.Key,
		Namespaces:     string(namespacesRaw),
		ExtensionsOnly: s.ExtensionsOnly,
		Created:        s.Created,
	}
}
