package storage

import (
	"github.com/clintjedwards/gofer/internal/models"
)

// These are groupings of all request structs used for the storage interface. These are purely so that its easier
// and clearer when calling storage functions. The drawback is that they require validation since we cannot force the
// caller to fill out the struct entirely.

// Namespace

type GetAllNamespacesRequest struct {
	Offset int
	Limit  int
}

type AddNamespaceRequest struct {
	Namespace *models.Namespace
}

type GetNamespaceRequest struct {
	ID string
}

type UpdateNamespaceRequest struct {
	Namespace *models.Namespace
}

type GetAllPipelinesRequest struct {
	Offset int
	Limit  int

	NamespaceID string
}

type GetPipelineRequest struct {
	NamespaceID string
	ID          string
}

type AddPipelineRequest struct {
	Pipeline *models.Pipeline
}

type UpdatePipelineRequest struct {
	Pipeline *models.Pipeline
}

// Runs

type GetAllRunsRequest struct {
	Offset int
	Limit  int

	NamespaceID string
	PipelineID  string
}

type GetRunRequest struct {
	NamespaceID string
	PipelineID  string
	ID          int64
}

type AddRunRequest struct {
	Run *models.Run
}

type UpdateRunRequest struct {
	Run *models.Run
}

type DeleteRunRequest struct {
	NamespaceID string
	PipelineID  string
	ID          int64
}

// Task Runs

type GetAllTaskRunsRequest struct {
	NamespaceID string
	PipelineID  string
	RunID       int64
}

type GetTaskRunRequest struct {
	NamespaceID string
	PipelineID  string
	RunID       int64
	ID          string
}

type AddTaskRunRequest struct {
	TaskRun *models.TaskRun
}

type UpdateTaskRunRequest struct {
	TaskRun *models.TaskRun
}

// events

type GetAllEventsRequest struct {
	Offset  int
	Limit   int
	Reverse bool
}

type GetEventRequest struct {
	ID int64
}

type AddEventRequest struct {
	Event models.Event
}

type DeleteEventRequest struct {
	ID int64
}

type GetAllTokensRequest struct {
	Offset     int
	Limit      int
	Namespaces []string
}

type AddTokenRequest struct {
	Token *models.Token
}

type GetTokenRequest struct {
	Hash string
}

type DeleteTokenRequest struct {
	Hash string
}
