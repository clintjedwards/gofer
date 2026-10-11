# API Reference

Gofer exposes a REST API; every endpoint lives under `/api` and speaks JSON unless noted otherwise. These pages are generated from the [OpenAPI spec](../assets/openapi.json) (Gofer v0.12.0) by `make generate-api-docs`.

## Authentication

Every request needs a bearer token. See [Authentication and Authorization](../ref/server_configuration/authz_n.md) for how tokens are created and what they can do.

## Versioning

Every request must carry a `gofer-api-version` header. The only version right now is `v0`.

```bash
curl http://localhost:8080/api/system/metadata \
  -H "Authorization: Bearer $GOFER_TOKEN" \
  -H "gofer-api-version: v0"
```

## Websockets

A few endpoints (event streaming, log following, task attach) upgrade the connection to a websocket instead of returning a JSON body. They're marked with a <span class="api-method api-method-ws">WS</span> badge.

## Sections

| Section | Description |
| ------- | ----------- |
| [Configs](./configs.md) | Pipeline configs are versioned configurations for a particular pipeline. |
| [Deployments](./deployments.md) | A deployment represents a transition between pipeline versions |
| [Events](./events.md) | Gofer emits events for actions that happen within it's purview. You can use the event api to get a list of all events or request specific events. |
| [Extensions](./extensions.md) | An extension is a way to give pipelines more functionality. This might include automatically running your pipeline or printing the results of a run to Slack or more. Pipelines can subscribe to one or more extensions (usually with some individual configuration) and those extensions perform actions on behalf of the pipeline. |
| [Namespaces](./namespaces.md) | A namespace represents a grouping of pipelines. Normally it is used to divide teams or logically different sections of workloads. It is the highest level unit as it sits above pipelines in the hierarchy of Gofer |
| [Objects](./objects.md) | The object store is a temporary key-vale storage mechanism for pipelines and runs. It allows the user to cache objects for the lifetime of multiple runs or for the lifetime of a single run.There are two separate types of objects, each useful for its own use case. Visit the documentation for more details on the associated lifetimes of pipeline specific and run specific objects |
| [Permissions](./permissions.md) | Gofer has an RBAC system which can be utilized to give different tokens/users permissions. |
| [Pipelines](./pipelines.md) | A pipeline is a graph of containers that accomplish some goal. Pipelines are created via a Pipeline configuration file and can be set to be run automatically via attached extensions |
| [Runs](./runs.md) | A run is a specific execution of a pipeline at a specific point in time. A run is made up of multiple tasks that all execute according to their dependency on each other. |
| [Secrets](./secrets.md) | Gofer allows user to enter secrets on both a global and pipeline scope. This is useful for workloads that need access to secret values and want a quick, convenient way to access those secrets. Global secrets are managed by admins and can grant pipelines access to secrets shared amongst many namespaces. Pipeline secrets on the other hand are only accessible from within that specific pipeline |
| [Subscriptions](./subscriptions.md) | A subscription represents a pipeline's subscription to a extension. |
| [System](./system.md) | Routes focused on meta-information for the Gofer service |
| [Tasks](./tasks.md) | A task is the lowest unit of execution for a pipeline. A task execution is the tracking of a task, which is to say a task execution is simply the tracking of the container that is in the act of being executed. |
| [Tokens](./tokens.md) | Gofer API Token |
| [Schemas](./schemas.md) | Request and response object definitions. |
