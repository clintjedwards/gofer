# Pipelines

A pipeline is a graph of containers that accomplish some goal. Pipelines are created via a Pipeline configuration file and can be set to be run automatically via attached extensions

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines`](#list-all-pipelines) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}`](#get-pipeline-by-id) |
| <span class="api-method api-method-patch">PATCH</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}`](#update-a-pipelines-state) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}`](#delete-pipeline-by-id) |

---

## List all pipelines

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines</code></div>

Returns the metadata for all pipelines. If you want a more complete picture of the pipeline details combine this endpoint with the configs endpoint to grab the metadata AND the user's pipeline configuration.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListPipelinesResponse](./schemas.md#listpipelinesresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get pipeline by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetPipelineResponse](./schemas.md#getpipelineresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Update a pipeline's state

<div class="api-endpoint"><span class="api-method api-method-patch">PATCH</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Request body** (required): [UpdatePipelineRequest](./schemas.md#updatepipelinerequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | resource updated |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete pipeline by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}</code></div>

IMPORTANT: Deleting a pipeline is set to cascade. All downstream objects to the pipeline (configs, secrets, runs, tasks) will be removed as well.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

