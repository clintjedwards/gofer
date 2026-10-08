# Secrets

Gofer allows user to enter secrets on both a global and pipeline scope. This is useful for workloads that need access to secret values and want a quick, convenient way to access those secrets. Global secrets are managed by admins and can grant pipelines access to secrets shared amongst many namespaces. Pipeline secrets on the other hand are only accessible from within that specific pipeline

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets`](#list-all-pipeline-secrets) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets`](#insert-a-new-secret-into-the-pipeline-secret-store) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}`](#get-pipeline-secret-by-key) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}`](#delete-pipeline-secret-by-key) |
| <span class="api-method api-method-get">GET</span> | [`/api/secrets/global`](#list-all-global-secrets) |
| <span class="api-method api-method-post">POST</span> | [`/api/secrets/global`](#insert-a-new-secret-into-the-global-secret-store) |
| <span class="api-method api-method-get">GET</span> | [`/api/secrets/global/{key}`](#get-global-secret-by-key) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/secrets/global/{key}`](#delete-global-secret-by-key) |

---

## List all pipeline secrets

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListPipelineSecretsResponse](./schemas.md#listpipelinesecretsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Insert a new secret into the pipeline secret store

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Request body** (required): [PutPipelineSecretRequest](./schemas.md#putpipelinesecretrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [PutPipelineSecretResponse](./schemas.md#putpipelinesecretresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get pipeline secret by key

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target secret. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | yes | Includes the actual plaintext secret in the response. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetPipelineSecretResponse](./schemas.md#getpipelinesecretresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete pipeline secret by key

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target secret. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## List all global secrets

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/secrets/global</code></div>

Admin tokens required.

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListGlobalSecretsResponse](./schemas.md#listglobalsecretsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Insert a new secret into the global secret store

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/secrets/global</code></div>

This route is only accessible for admin tokens.

**Request body** (required): [PutGlobalSecretRequest](./schemas.md#putglobalsecretrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [PutGlobalSecretResponse](./schemas.md#putglobalsecretresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get global secret by key

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/secrets/global/{key}</code></div>

Admin token required.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target secret. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | yes | Includes the actual plaintext secret in the response. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetGlobalSecretResponse](./schemas.md#getglobalsecretresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete global secret by key

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/secrets/global/{key}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `key` | string | yes | The unique identifier for the target secret. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

