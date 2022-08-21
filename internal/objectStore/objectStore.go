package objectStore

import "errors"

type EngineType string

const (
	// EngineBolt uses the boltDB.
	EngineBolt EngineType = "bolt"
)

var (
	// ErrEntityNotFound is returned when a certain entity could not be located.
	ErrEntityNotFound = errors.New("objectstore: entity not found")

	// ErrEntityExists is returned when a certain entity was located but not meant to be.
	ErrEntityExists = errors.New("objectstore: entity already exists")

	// ErrPreconditionFailure is returned when there was a validation error with the parameters passed.
	ErrPreconditionFailure = errors.New("objectstore: parameters did not pass validation")
)

type Engine interface {
	GetObject(key string) ([]byte, error)
	PutObject(key string, content []byte, force bool) error
	ListObjectKeys(prefix string) ([]string, error)
	DeleteObject(key string) error
}
