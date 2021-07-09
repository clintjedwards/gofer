---
id: overview
title: Overview
sidebar_position: 1
---

# Storage

Storage is simply the Gofer database aka how Gofer stores persistent information.

## Supported Storage

The only currently supported storage is the [boltdb object store](bolt/overview).

## How to add new Storage?

Databases are pluggable! Simply implement a new database by following [the given interface.](https://github.com/clintjedwards/gofer/blob/053ad33e30e9fdf21a005fffbe9ad849fe258ec1/internal/storage/storage.go#L30)

```go
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

	GetAllTriggerEvents(r GetAllTriggerEventsRequest) ([]*models.TriggerEvent, error)
	GetTriggerEvent(r GetTriggerEventRequest) (*models.TriggerEvent, error)
	AddTriggerEvent(r AddTriggerEventRequest) error
	UpdateTriggerEvent(r UpdateTriggerEventRequest) error

	GetAllRunRegistrations(r GetAllRunRegistrationsRequest) (map[RunRegistryKey]struct{}, error)
	RegisterRun(r RegisterRunRequest) error
	UnregisterRun(r UnregisterRunRequest) error
	RegistrationExists(r RegistrationExistsRequest) bool

	GetAllDockerRegistryAuths(r GetAllDockerRegistryAuthsRequest) ([]*models.DockerRegistryAuth, error)
	AddDockerRegistryAuth(r AddDockerRegistryAuthRequest) error
	RemoveDockerRegistryAuth(r RemoveDockerRegistryAuthRequest) error
}
```
