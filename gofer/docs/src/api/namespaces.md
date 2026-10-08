# Namespaces

A namespace represents a grouping of pipelines. Normally it is used to divide teams or logically different sections of workloads. It is the highest level unit as it sits above pipelines in the hierarchy of Gofer

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces`](#list-all-namespaces) |
| <span class="api-method api-method-post">POST</span> | [`/api/namespaces`](#create-a-new-namespace) |
| <span class="api-method api-method-get">GET</span> | [`/api/namespaces/{namespace_id}`](#get-api-namespace-by-id) |
| <span class="api-method api-method-patch">PATCH</span> | [`/api/namespaces/{namespace_id}`](#update-a-namespaces-details) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/namespaces/{namespace_id}`](#delete-api-namespace-by-id) |

---

## List all namespaces

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces</code></div>

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListNamespacesResponse](./schemas.md#listnamespacesresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Create a new namespace

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/namespaces</code></div>

This route is only accessible for admin tokens.

**Request body** (required): [CreateNamespaceRequest](./schemas.md#createnamespacerequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [CreateNamespaceResponse](./schemas.md#createnamespaceresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get api namespace by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/namespaces/{namespace_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetNamespaceResponse](./schemas.md#getnamespaceresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Update a namespace's details

<div class="api-endpoint"><span class="api-method api-method-patch">PATCH</span><code>/api/namespaces/{namespace_id}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |

**Request body** (required): [UpdateNamespaceRequest](./schemas.md#updatenamespacerequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [UpdateNamespaceResponse](./schemas.md#updatenamespaceresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete api namespace by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/namespaces/{namespace_id}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `namespace_id` | string | yes | The unique identifier for the target namespace. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

