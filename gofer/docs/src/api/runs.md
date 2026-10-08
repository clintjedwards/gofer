# Runs

A run is a specific execution of a pipeline at a specific point in time. A run is made up of multiple tasks that all execute according to their dependency on each other.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs`](#list-all-runs) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs`](#start-a-run-of-a-particular-pipeline) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}`](#get-run-by-id) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}`](#cancel-a-run-by-id) |

---

## List all runs

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs</code></div>

Returns a list of all runs by pipeline id.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `limit` | integer | no |  |
| `offset` | integer | no |  |
| `reverse` | boolean | no |  |
| `since` | integer | no | Only return runs started at or after this time, in epoch milliseconds. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListRunsResponse](./schemas.md#listrunsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Start a run of a particular pipeline

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Request body** (required): [StartRunRequest](./schemas.md#startrunrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [StartRunResponse](./schemas.md#startrunresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get run by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetRunResponse](./schemas.md#getrunresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Cancel a run by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

