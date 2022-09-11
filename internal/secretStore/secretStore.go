package secretStore

import "errors"

type EngineType string

const (
	// EngineSqlite uses the sqlite DB.
	EngineSqlite EngineType = "sqlite"
)

var (
	// ErrInternal is returned when the db runs into an unexpected error.
	ErrInternal = errors.New("secretstore: internal error occurred")

	// ErrEntityNotFound is returned when a certain entity could not be located.
	ErrEntityNotFound = errors.New("secretStore: entity not found")

	// ErrEntityExists is returned when a certain entity was located but not meant to be.
	ErrEntityExists = errors.New("secretStore: entity already exists")

	// ErrPreconditionFailure is returned when there was a validation error with the parameters passed.
	ErrPreconditionFailure = errors.New("secretStore: parameters did not pass validation")
)

type Engine interface {
	GetSecret(key string) (string, error)
	PutSecret(key string, content string, force bool) error
	ListSecretKeys(prefix string) ([]string, error)
	DeleteSecret(key string) error
}
