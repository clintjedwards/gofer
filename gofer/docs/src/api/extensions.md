# Extensions

An extension is a way to give pipelines more functionality. This might include automatically running your pipeline or printing the results of a run to Slack or more. Pipelines can subscribe to one or more extensions (usually with some individual configuration) and those extensions perform actions on behalf of the pipeline.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions`](#list-all-extensions-currently-registered) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions`](#register-and-start-a-new-extension) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}`](#returns-details-about-a-specific-extension) |
| <span class="api-method api-method-patch">PATCH</span> | [`/api/extensions/{extension_id}`](#enable-or-disable-an-extension) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/extensions/{extension_id}`](#uninstall-a-registered-extension) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/debug`](#dump-extension-debug-information) |
| <span class="api-method api-method-ws">WS</span> | [`/api/extensions/{extension_id}/logs`](#retrieves-logs-from-the-extension-container) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/subscriptions`](#list-all-extension-subscriptions) |

---

## List all extensions currently registered

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions</code></div>

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | no | Return credentials in plaintext instead of redacted. Admin only. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListExtensionsResponse](./schemas.md#listextensionsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Register and start a new extension

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/extensions</code></div>

This route is only available to admin tokens.

**Request body** (required): [InstallExtensionRequest](./schemas.md#installextensionrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [InstallExtensionResponse](./schemas.md#installextensionresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Returns details about a specific extension

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_secret` | boolean | no | Return credentials in plaintext instead of redacted. Admin only. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetExtensionResponse](./schemas.md#getextensionresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Enable or disable an extension

<div class="api-endpoint"><span class="api-method api-method-patch">PATCH</span><code>/api/extensions/{extension_id}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Request body** (required): [UpdateExtensionRequest](./schemas.md#updateextensionrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | resource updated |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Uninstall a registered extension

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/extensions/{extension_id}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Dump extension debug information

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}/debug</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [DebugResponse](./schemas.md#debugresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Retrieves logs from the extension container

<div class="api-endpoint"><span class="api-method api-method-ws">WS</span><code>/api/extensions/{extension_id}/logs</code></div>

This endpoint upgrades the connection to a websocket: after the `101` response the server streams data over the socket instead of returning a body.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `101` | none | Negotiating protocol upgrade from HTTP/1.1 to WebSocket |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## List all extension subscriptions

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}/subscriptions</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListExtensionSubscriptionsResponse](./schemas.md#listextensionsubscriptionsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

