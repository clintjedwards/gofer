# Tokens

Gofer API Token

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/tokens`](#list-all-gofer-api-tokens) |
| <span class="api-method api-method-post">POST</span> | [`/api/tokens`](#create-a-new-token) |
| <span class="api-method api-method-post">POST</span> | [`/api/tokens/bootstrap`](#create-root-admin-token) |
| <span class="api-method api-method-post">POST</span> | [`/api/tokens/web-login`](#start-a-browser-sign-in) |
| <span class="api-method api-method-post">POST</span> | [`/api/tokens/web-login/exchange`](#finish-a-browser-sign-in) |
| <span class="api-method api-method-get">GET</span> | [`/api/tokens/whoami`](#get-api-token-who-made-the-request) |
| <span class="api-method api-method-get">GET</span> | [`/api/tokens/{id}`](#get-api-token-by-id) |
| <span class="api-method api-method-patch">PATCH</span> | [`/api/tokens/{id}`](#update-a-tokens-state) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/tokens/{id}`](#delete-api-token-by-id) |

---

## List all Gofer API tokens

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/tokens</code></div>

This endpoint is restricted to admin tokens only.

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListTokensResponse](./schemas.md#listtokensresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Create a new token

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/tokens</code></div>

This endpoint is restricted to admin tokens only.

**Request body** (required): [CreateTokenRequest](./schemas.md#createtokenrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [CreateTokenResponse](./schemas.md#createtokenresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Create root admin token

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/tokens/bootstrap</code></div>

This endpoint can only be hit once and will create the root admin token, from which all other tokens can be created.

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [CreateTokenResponse](./schemas.md#createtokenresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Start a browser sign in

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/tokens/web-login</code></div>

Returns a short lived, single use code that a browser can trade for the calling token, so the CLI can open the web UI already signed in without ever putting the token itself in a URL.

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [CreateWebLoginResponse](./schemas.md#createwebloginresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Finish a browser sign in

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/tokens/web-login/exchange</code></div>

Trades a code from `create_web_login` for the token that created it. Each code works once. The code is sent in the body rather than the path so it stays out of request logs.

**Request body** (required): [ExchangeWebLoginRequest](./schemas.md#exchangewebloginrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ExchangeWebLoginResponse](./schemas.md#exchangewebloginresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get api token who made the request

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/tokens/whoami</code></div>

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [WhoAmIResponse](./schemas.md#whoamiresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get api token by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/tokens/{id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `id` | string | yes | The unique identifier for the target namespace. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetTokenByIDResponse](./schemas.md#gettokenbyidresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Update a token's state

<div class="api-endpoint"><span class="api-method api-method-patch">PATCH</span><code>/api/tokens/{id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `id` | string | yes | The unique identifier for the target namespace. |

**Request body** (required): [UpdateTokenRequest](./schemas.md#updatetokenrequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | resource updated |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete api token by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/tokens/{id}</code></div>

This endpoint is restricted to admin tokens only.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `id` | string | yes | The unique identifier for the target namespace. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

