// Package storage contains the interface in which the storage layer adheres to.
// It has a fairly tight coupling with the main application(for example providing autogenerated/sequential ids)
// so it's flexibility with introducing other storage options heavily requires the writer to
// look into what the original Bolt package is doing.
package storage

import (
	"errors"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/gofer/internal/models"
)

// EngineType type represents the different possible storage engines available
type EngineType string

const (
	// StorageEngineBoltDB represents a boltDB storage engine.
	// A file based key-value store.(https://pkg.go.dev/go.etcd.io/bbolt) used through (https://github.com/asdine/storm)
	StorageEngineBoltDB EngineType = "bolt"
)

var (
	// ErrEntityNotFound is returned when a certain entity could not be located.
	ErrEntityNotFound = errors.New("storage: entity not found")

	// ErrEntityExists is returned when a certain entity was located but not meant to be.
	ErrEntityExists = errors.New("storage: entity already exists")

	// ErrPreconditionFailure is returned when there was a validation error with the parameters passed.
	ErrPreconditionFailure = errors.New("storage: parameters did not pass validation")
)

// Engine represents backend storage implementations where items can be persisted.
type Engine interface {
	GetAllNamespaces(r GetAllNamespacesRequest) ([]*models.Namespace, error)
	AddNamespace(r AddNamespaceRequest) error
	GetNamespace(r GetNamespaceRequest) (*models.Namespace, error)
	UpdateNamespace(r UpdateNamespaceRequest) error

	GetAllTokens(r GetAllTokensRequest) ([]*models.Token, error)
	AddToken(r AddTokenRequest) error
	GetToken(r GetTokenRequest) (*models.Token, error)
	DeleteToken(r DeleteTokenRequest) error

	GetAllPipelines(r GetAllPipelinesRequest) ([]*models.Pipeline, error)
	GetPipeline(r GetPipelineRequest) (*models.Pipeline, error)
	AddPipeline(r AddPipelineRequest) error
	UpdatePipeline(r UpdatePipelineRequest) error

	GetAllRuns(r GetAllRunsRequest) ([]*models.Run, error)
	GetRun(r GetRunRequest) (*models.Run, error)
	AddRun(r AddRunRequest) error
	UpdateRun(r UpdateRunRequest) error

	GetAllTaskRuns(r GetAllTaskRunsRequest) ([]*models.TaskRun, error)
	GetTaskRun(r GetTaskRunRequest) (*models.TaskRun, error)
	AddTaskRun(r AddTaskRunRequest) error
	UpdateTaskRun(r UpdateTaskRunRequest) error

	GetAllEvents(r GetAllEventsRequest) ([]models.Event, error)
	GetEvent(r GetEventRequest) (models.Event, error)
	AddEvent(r AddEventRequest) error
	DeleteEvent(r DeleteEventRequest) error

	GetAllTriggers(r GetAllTriggersRequest) ([]*config.Trigger, error)
	GetTrigger(r GetTriggerRequest) (*config.Trigger, error)
	AddTrigger(r AddTriggerRequest) error
	DeleteTrigger(r DeleteTriggerRequest) error

	GetAllNotifiers(r GetAllNotifiersRequest) ([]*config.Notifier, error)
	GetNotifier(r GetNotifierRequest) (*config.Notifier, error)
	AddNotifier(r AddNotifierRequest) error
	DeleteNotifier(r DeleteNotifierRequest) error
}
