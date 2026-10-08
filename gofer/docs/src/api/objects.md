# Objects

The object store is a temporary key-vale storage mechanism for pipelines and runs. It allows the user to cache objects for the lifetime of multiple runs or for the lifetime of a single run.There are two separate types of objects, each useful for its own use case. Visit the documentation for more details on the associated lifetimes of pipeline specific and run specific objects

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/objects`](#list-all-extension-objects) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/objects/{key}`](#get-extension-object-by-key) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/{extension_id}/objects/{key}`](#insert-a-new-object-into-the-extension-object-store) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/extensions/{extension_id}/objects/{key}`](#delete-extension-object-by-key) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects`](#list-all-pipeline-objects) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}`](#get-pipeline-object-by-key) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}`](#insert-a-new-object-into-the-pipeline-object-store) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}`](#delete-pipeline-object-by-key) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects`](#list-all-run-objects) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}`](#get-run-object-by-key) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}`](#insert-a-new-object-into-the-run-object-store) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}`](#delete-run-object-by-key) |

---

## List all extension objects

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}/objects</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListExtensionObjectsResponse](./schemas.md#listextensionobjectsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get extension object by key

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |
| `key` | string | yes | The unique identifier for the target object. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | raw bytes | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Insert a new object into the extension object store

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/extensions/{extension_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |
| `key` | string | yes | The unique identifier for the target object. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `force` | boolean | yes | Overwrite a value of a object if it already exists. |

**Request body** (required): raw bytes (`application/octet-stream`)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [PutExtensionObjectResponse](./schemas.md#putextensionobjectresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete extension object by key

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/extensions/{extension_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |
| `key` | string | yes | The unique identifier for the target object. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## List all pipeline objects

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListPipelineObjectsResponse](./schemas.md#listpipelineobjectsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get pipeline object by key

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target object. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | raw bytes | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Insert a new object into the pipeline object store

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target object. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `force` | boolean | yes | Overwrite a value of a object if it already exists. |

**Request body** (required): raw bytes (`application/octet-stream`)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [PutPipelineObjectResponse](./schemas.md#putpipelineobjectresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete pipeline object by key

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target object. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## List all run objects

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListRunObjectsResponse](./schemas.md#listrunobjectsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get run object by key

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target object. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | raw bytes | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Insert a new object into the run object store

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}</code></div>

Overwrites can be performed by passing the `force` query param.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target object. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `force` | boolean | yes | Overwrite a value of a object if it already exists. |

**Request body** (required): raw bytes (`application/octet-stream`)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [PutRunObjectResponse](./schemas.md#putrunobjectresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete run object by key

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target object. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `run_id` | integer | yes | The unique identifier for the target run. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

