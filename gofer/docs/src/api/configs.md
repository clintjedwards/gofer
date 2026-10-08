# Configs

Pipeline configs are versioned configurations for a particular pipeline.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs`](#list-all-pipeline-configs) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs`](#register-a-new-pipeline-configuration) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}`](#get-a-specific-version-of-a-pipeline-configuration) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}`](#deploy-pipeline-config) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}`](#delete-pipeline-config-by-version) |

---

## List all pipeline configs

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs</code></div>

A pipeline's config is the small program you write to configure how you want your pipeline to run.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | no | Return credentials in plaintext instead of redacted. Admin only. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListPipelineConfigsResponse](./schemas.md#listpipelineconfigsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Register a new pipeline configuration

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs</code></div>

This creates both the pipeline metadata and the initial config object.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Request body** (required): [RegisterPipelineConfigRequest](./schemas.md#registerpipelineconfigrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [RegisterPipelineConfigResponse](./schemas.md#registerpipelineconfigresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get a specific version of a pipeline configuration

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}</code></div>

A version of 0 indicates to return the latest pipeline config.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `version` | integer | yes | The version of the configuration you want to target. 0 means return the latest. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | no | Return credentials in plaintext instead of redacted. Admin only. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetPipelineConfigResponse](./schemas.md#getpipelineconfigresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Deploy pipeline config

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `version` | integer | yes | The version of the configuration you want to target. 0 means return the latest. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [DeployPipelineConfigResponse](./schemas.md#deploypipelineconfigresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete pipeline config by version

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |
| `version` | integer | yes | The version of the configuration you want to target. 0 means return the latest. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

