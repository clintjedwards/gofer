package models

import "time"

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
