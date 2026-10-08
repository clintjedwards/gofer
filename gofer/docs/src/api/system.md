# System

Routes focused on meta-information for the Gofer service

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/system`](#get-system-parameters) |
| <span class="api-method api-method-patch">PATCH</span> | [`/api/system`](#update-system-parameters) |
| <span class="api-method api-method-get">GET</span> | [`/api/system/metadata`](#describe-current-system-meta-information) |

---

## Get system parameters

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/system</code></div>

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetSystemPreferencesResponse](./schemas.md#getsystempreferencesresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Update system parameters

<div class="api-endpoint"><span class="api-method api-method-patch">PATCH</span><code>/api/system</code></div>

**Request body** (required): [UpdateSystemPreferencesRequest](./schemas.md#updatesystempreferencesrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [UpdateSystemPreferencesResponse](./schemas.md#updatesystempreferencesresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Describe current system meta-information

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/system/metadata</code></div>

Return a number of internal metadata about the Gofer service itself.

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetSystemMetadataResponse](./schemas.md#getsystemmetadataresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

