# Extensions

An extension is a way to give pipelines more functionality. This might include automatically running your pipeline or printing the results of a run to Slack or more. Pipelines can subscribe to one or more extensions (usually with some individual configuration) and those extensions perform actions on behalf of the pipeline.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions`](#list-all-extensions-currently-registered) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/plan`](#compare-gofers-config-with-the-extensions-currently-running) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}`](#returns-details-about-a-specific-extension) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/extensions/{extension_id}`](#permanently-delete-an-extension-along-with-its-pipeline-subscriptions-and-stored-objects) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/{extension_id}/apply`](#bring-one-extension-in-line-with-gofers-config) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/debug`](#dump-extension-debug-information) |
| <span class="api-method api-method-ws">WS</span> | [`/api/extensions/{extension_id}/logs`](#retrieves-logs-from-the-extension-container) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/{extension_id}/revert`](#go-back-to-the-version-of-an-extension-that-was-running-before-the-last-apply) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/subscriptions`](#list-all-extension-subscriptions) |

---

## List all extensions currently registered

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions</code></div>

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListExtensionsResponse](./schemas.md#listextensionsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Compare Gofer's config with the extensions currently running

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/extensions/plan</code></div>

Rereads Gofer's config file and reports, for each extension, what applying the config would change. Nothing is changed. This backs `gofer extension reload`. This route is only accessible for admin tokens.

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [PlanExtensionsResponse](./schemas.md#planextensionsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Returns details about a specific extension

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetExtensionResponse](./schemas.md#getextensionresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Permanently delete an extension along with its pipeline subscriptions and stored objects

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/extensions/{extension_id}</code></div>

Only extensions that have already been removed from Gofer's config can be purged; Gofer would just install a configured one again. This route is only accessible for admin tokens.

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

## Bring one extension in line with Gofer's config

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/extensions/{extension_id}/apply</code></div>

Rereads Gofer's config file and installs, updates, starts, or stops the extension to match. The request carries the `plan_hash` from the plan the operator reviewed; if anything changed since then (the config, the manifest, or a secret) the request is refused so the operator can look at a fresh plan. This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Request body** (required): [ApplyExtensionRequest](./schemas.md#applyextensionrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ApplyExtensionResponse](./schemas.md#applyextensionresponse) | successful operation |
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

## Go back to the version of an extension that was running before the last apply

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/extensions/{extension_id}/revert</code></div>

Meant for when a new version won't start. The extension stays different from Gofer's config until the config is fixed, so it'll show up in the next plan again. This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ApplyExtensionResponse](./schemas.md#applyextensionresponse) | successful operation |
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

