# Tasks

A task is the lowest unit of execution for a pipeline. A task execution is the tracking of a task, which is to say a task execution is simply the tracking of the container that is in the act of being executed.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks`](#list-all-task-executions) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}`](#get-task-execution-by-id) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}`](#cancel-a-task-execution-by-id) |
| <span class="api-method api-method-ws">WS</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/attach`](#run-command-on-a-running-task-execution-container) |
| <span class="api-method api-method-ws">WS</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs`](#retrieves-logs-from-a-task-execution) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs`](#removes-a-task-executions-associated-log-object) |

---

## List all task executions

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks</code></div>

Returns a list of all task executions by run.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | no | Return credentials in plaintext instead of redacted. Admin only. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListTaskExecutionsResponse](./schemas.md#listtaskexecutionsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get task execution by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |
| `task_id` | string | yes | The unique identifier for the target task execution. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | no | Return credentials in plaintext instead of redacted. Admin only. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetTaskExecutionResponse](./schemas.md#gettaskexecutionresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Cancel a task execution by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |
| `task_id` | string | yes | The unique identifier for the target task execution. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `wait_for` | integer | yes | Period of time to wait the task before forcing it to cancel. 0 means send SIGKILL instantly. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Run command on a running task execution container

<div class="api-endpoint"><span class="api-method api-method-ws">WS</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/attach</code></div>

This allows you to run a command on a task execution container and connect to the stdin and stdout/err for said container.

Useful for debugging.

This endpoint upgrades the connection to a websocket: after the `101` response the server streams data over the socket instead of returning a body.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |
| `task_id` | string | yes | The unique identifier for the target task execution. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `command` | string | yes |  |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `101` | none | Negotiating protocol upgrade from HTTP/1.1 to WebSocket |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Retrieves logs from a task execution

<div class="api-endpoint"><span class="api-method api-method-ws">WS</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs</code></div>

This endpoint upgrades the connection to a websocket: after the `101` response the server streams data over the socket instead of returning a body.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |
| `task_id` | string | yes | The unique identifier for the target task execution. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `101` | none | Negotiating protocol upgrade from HTTP/1.1 to WebSocket |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Removes a task execution's associated log object

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs</code></div>

This is useful for if logs mistakenly contain sensitive data.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |
| `task_id` | string | yes | The unique identifier for the target task execution. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

