package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type SecretStoreKey struct {
	Key     string `json:"key"`
	Created int64  `json:"created"`
}

func NewSecretStoreKey(key string) *SecretStoreKey {
	return &SecretStoreKey{
		Key:     key,
		Created: time.Now().UnixMilli(),
	}
}

func (s *SecretStoreKey) ToProto() *proto.SecretStoreKey {
	return &proto.SecretStoreKey{
		Key:     s.Key,
		Created: s.Created,
	}
}
