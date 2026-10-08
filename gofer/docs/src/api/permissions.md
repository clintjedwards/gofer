# Permissions

Gofer has an RBAC system which can be utilized to give different tokens/users permissions.

| Method | Endpoint |
| ------ | -------- |
| <span class="api-method api-method-get">GET</span> | [`/api/roles`](#list-all-roles) |
| <span class="api-method api-method-post">POST</span> | [`/api/roles`](#create-a-new-role) |
| <span class="api-method api-method-get">GET</span> | [`/api/roles/{role_id}`](#get-api-role-by-id) |
| <span class="api-method api-method-patch">PATCH</span> | [`/api/roles/{role_id}`](#update-a-roles-details) |
| <span class="api-method api-method-delete">DELETE</span> | [`/api/roles/{role_id}`](#delete-api-role-by-id) |

---

## List all roles

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/roles</code></div>

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [ListRolesResponse](./schemas.md#listrolesresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Create a new role

<div class="api-endpoint"><span class="api-method api-method-post">POST</span><code>/api/roles</code></div>

This route is only accessible for admin tokens.

**Request body** (required): [CreateRoleRequest](./schemas.md#createrolerequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `201` | [CreateRoleResponse](./schemas.md#createroleresponse) | successful creation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Get api role by id

<div class="api-endpoint"><span class="api-method api-method-get">GET</span><code>/api/roles/{role_id}</code></div>

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `role_id` | string | yes | The unique identifier for the target role. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [GetRoleResponse](./schemas.md#getroleresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Update a role's details

<div class="api-endpoint"><span class="api-method api-method-patch">PATCH</span><code>/api/roles/{role_id}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `role_id` | string | yes | The unique identifier for the target role. |

**Request body** (required): [UpdateRoleRequest](./schemas.md#updaterolerequest)

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `200` | [UpdateRoleResponse](./schemas.md#updateroleresponse) | successful operation |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |


---

## Delete api role by id

<div class="api-endpoint"><span class="api-method api-method-delete">DELETE</span><code>/api/roles/{role_id}</code></div>

This route is only accessible for admin tokens.

**Path parameters**

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| `role_id` | string | yes | The unique identifier for the target role. |

**Responses**

| Status | Body | Description |
| ------ | ---- | ----------- |
| `204` | none | successful deletion |
| `4XX` | [Error](./schemas.md#error) | Error |
| `5XX` | [Error](./schemas.md#error) | Error |

