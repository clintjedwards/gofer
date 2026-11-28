# Extensions

An extension is a way to give pipelines more functionality. This might include automatically running your pipeline or printing the results of a run to Slack or more. Pipelines can subscribe to one or more extensions (usually with some individual configuration) and those extensions perform actions on behalf of the pipeline.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions`](#list-all-extensions-currently-registered) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/images`](#list-the-extension-images-stored-in-gofer) |
| <span class="api-method api-method-put">PUT</span> | [`/api/extensions/images/blobs/{digest}`](#upload-an-extension-image-blob) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/images/missing`](#list-which-extension-image-blobs-need-to-be-uploaded) |
| <span class="api-method api-method-put">PUT</span> | [`/api/extensions/images/{name}/{tag}`](#store-an-extension-image) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/extensions/images/{name}/{tag}`](#delete-an-extension-image) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/plan`](#compare-gofers-config-with-the-extensions-currently-running) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}`](#returns-details-about-a-specific-extension) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/extensions/{extension_id}`](#permanently-delete-an-extension-along-with-its-api-token-pipeline-subscriptions-and-stored-objects) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/{extension_id}/apply`](#bring-one-extension-in-line-with-gofers-config) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/debug`](#dump-extension-debug-information) |
| <span class="api-method api-method-ws">WS</span> | [`/api/extensions/{extension_id}/logs`](#retrieves-logs-from-the-extension-container) |
| <span class="api-method api-method-post">POST</span> | [`/api/extensions/{extension_id}/revert`](#go-back-to-the-version-of-an-extension-that-was-running-before-the-last-apply) |
| <span class="api-method api-method-get">GET</span> | [`/api/extensions/{extension_id}/subscriptions`](#list-an-extensions-subscriptions) |

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

## List the extension images stored in Gofer

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/images</code></div>

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListExtensionImagesResponse](./schemas.md#listextensionimagesresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Upload an extension image blob

<div class="api-endpoint"><span class="api-method api-method-put">PUT</span><code>/api/extensions/images/blobs/{digest}</code></div>

Uploads one part of an extension image: its manifest, its config, or one of its layers. The contents must match the digest in the path. Uploading the same blob again is fine.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `digest` | string | yes | The sha256 digest of the blob's contents. Ex: `sha256:2cf24dba5fb0...` |

**Request body** (required): raw bytes (`application/octet-stream`)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | resource updated |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## List which extension image blobs need to be uploaded

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/extensions/images/missing</code></div>

Given the digests of the blobs that make up an extension image, returns the ones Gofer doesn't have yet. Extension images can only use blobs uploaded for extension images, so blobs only a pipeline has uploaded are returned too.

**Request body** (required): [ListMissingImageBlobsRequest](./schemas.md#listmissingimageblobsrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListMissingImageBlobsResponse](./schemas.md#listmissingimageblobsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Store an extension image

<div class="api-endpoint"><span class="api-method api-method-put">PUT</span><code>/api/extensions/images/{name}/{tag}</code></div>

Stores an image whose parts were uploaded, so manifests can name it as `gofer://<name>:<tag>`. The image has to be one Gofer can run. A name and tag that's already stored is refused unless `force` is set, and the names of Gofer's own extensions are kept for them.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `name` | string | yes | The image's name, as in `gofer://<name>:<tag>`. |
| `tag` | string | yes | The image's tag, as in `gofer://<name>:<tag>`. |

**Request body** (required): [PutExtensionImageRequest](./schemas.md#putextensionimagerequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [PutExtensionImageResponse](./schemas.md#putextensionimageresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete an extension image

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/extensions/images/{name}/{tag}</code></div>

Refused while any extension's manifest names it, including extensions removed from the config but not yet purged, and for Gofer's own extensions. Its blobs are removed once nothing else uses them.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `name` | string | yes | The image's name, as in `gofer://<name>:<tag>`. |
| `tag` | string | yes | The image's tag, as in `gofer://<name>:<tag>`. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
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

## Permanently delete an extension along with its API token, pipeline subscriptions, and stored objects

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

## List an extension's subscriptions

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/extensions/{extension_id}/subscriptions</code></div>

Leaves out disabled subscriptions unless `include_disabled` is set.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `extension_id` | string | yes | The unique identifier for the target extension. |

**Query parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `include_disabled` | boolean | no | Include subscriptions that were turned off. Extensions leave this off when they load their subscriptions on startup, so disabled ones never fire. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListExtensionSubscriptionsResponse](./schemas.md#listextensionsubscriptionsresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

