package models

import (
	"time"
)

type ObjectStoreKey struct {
	Key     string `json:"key" example:"example_key" doc:"The unique key for the object"`
	Created uint64 `json:"created" example:"1712433802634" doc:"Time object was created in epoch milliseconds"`
}

func NewObjectStoreKey(key string) *ObjectStoreKey {
	return &ObjectStoreKey{
		Key:     key,
		Created: uint64(time.Now().UnixMilli()),
	}
}
