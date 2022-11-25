package models

import (
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"
)

type ObjectStoreKey struct {
	Key     string `json:"key"`
	Created int64  `json:"created"`
}

func NewObjectStoreKey(key string) *ObjectStoreKey {
	return &ObjectStoreKey{
		Key:     key,
		Created: time.Now().UnixMilli(),
	}
}

func (s *ObjectStoreKey) ToProto() *proto.ObjectStoreKey {
	return &proto.ObjectStoreKey{
		Key:     s.Key,
		Created: s.Created,
	}
}
