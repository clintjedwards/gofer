# Deployments

A deployment represents a transition between pipeline versions

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments`](#list-all-deployments) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments/{deployment_id}`](#get-api-deployment-by-id) |

---

## List all deployments

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListDeploymentsResponse](./schemas.md#listdeploymentsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get api deployment by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments/{deployment_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `deployment_id` | integer | yes | The unique identifier for the target deployment. |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |
| `pipeline_id` | string | yes | The unique identifier for the target pipeline. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetDeploymentResponse](./schemas.md#getdeploymentresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

