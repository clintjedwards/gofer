# Authentication and Authorization

Gofer's authentication and authorization systems are designed to be lightweight and unobtrusive,
allowing you to focus on your tasks with minimal interference. The permissioning system is based on
Role-Based Access Control (RBAC) and is primarily used to prevent certain token holders, such as extensions,
from accessing excessive parts of the system.

For most users, this means the ability to categorize users into distinct groups, typically at the namespace or pipeline level.

Authorization in Gofer is token-based, a principle that extends seamlessly to the frontend as well.

## Authorization

Gofer organizes grants into roles, which are then assigned to specific tokens. A token can do whatever its roles'
grants allow.

### Roles

A role is a collection of grants. The most significant role is the bootstrap role, which is a special role.
The bootstrap role is equivalent to a root role and is assigned to the first token you receive within Gofer.

Gofer also includes 'system roles' which are special roles that cannot be modified or removed. These roles are
generally used by other components of Gofer for specific purposes or are there for your convenience.

You can view a list of these roles by using the `gofer role list` command and identifying the roles
where `system_role` is marked as `true`.

### Grants

A role's grants are split into three groups, based on what they apply to:

```json
{
  "id": "devops",
  "description": "Full access to devops namespaces and can follow events",
  "grants": {
    "namespaces": [
      {
        "namespace": "devops.*",
        "pipeline": ".*",
        "resources": ["pipelines", "configs", "deployments", "runs", "task_executions", "objects", "secrets", "subscriptions"],
        "actions": ["read", "write", "delete"]
      }
    ],
    "extensions": [
      { "extension": "github", "resources": ["objects"], "actions": ["read"] }
    ],
    "global": [
      { "resources": ["events"], "actions": ["read"] }
    ]
  }
}
```

**Each grant stands on its own.** A request is allowed only if a single grant matches what the request targets and
includes both the resource and the action. Gofer never combines separate grants to satisfy a request. For example:

```json
"namespaces": [
  { "namespace": "frontend", "pipeline": "website", "resources": ["runs"], "actions": ["read"] },
  { "namespace": "backend", "pipeline": "billing", "resources": ["secrets"], "actions": ["read"] }
]
```

This allows reading runs for `frontend/website` and secrets for `backend/billing`, but not runs for
`frontend/billing` or secrets for `frontend/website`.

A token can have several roles. The grants from all of its roles are pooled together, and the request is allowed if any
one of them covers it.

#### Namespace grants

Namespace grants cover everything that lives under a pipeline.

| Field       | Description                                                                          |
| ----------- | ------------------------------------------------------------------------------------ |
| `namespace` | Required. A regex matched against the namespace id.                                  |
| `pipeline`  | Optional. A regex matched against the pipeline id. Leaving it out matches every pipeline. |
| `resources` | One or more of the resources below.                                                  |
| `actions`   | One or more of `read`, `write`, `delete`.                                            |

| Resource          | Covers                                                    |
| ----------------- | --------------------------------------------------------- |
| `pipelines`       | Pipeline details; listing, reading, updating and deleting pipelines. |
| `configs`         | Pipeline configurations, including deploying them.        |
| `deployments`     | Pipeline deployments.                                     |
| `runs`            | Starting, reading and cancelling runs.                    |
| `task_executions` | Task executions, their logs, and attaching to them.       |
| `objects`         | The pipeline and run object stores.                       |
| `secrets`         | The pipeline secret store.                                |
| `subscriptions`   | The pipeline's extension subscriptions.                   |

Any namespace grant also lets the token read the namespace itself.

#### Extension grants

| Field       | Description                                      |
| ----------- | ------------------------------------------------ |
| `extension` | Required. A regex matched against the extension id. |
| `resources` | One or more of the resources below.              |
| `actions`   | One or more of `read`, `write`, `delete`.        |

| Resource        | Covers                                        |
| --------------- | --------------------------------------------- |
| `objects`       | The extension's object store.                 |
| `subscriptions` | Listing the pipelines subscribed to the extension. |
| `logs`          | The extension's logs.                         |

Any extension grant also lets the token read the extension's details.

#### Global grants

Global grants cover things that don't belong to a namespace or extension.

| Resource  | Covers                                 |
| --------- | -------------------------------------- |
| `events`  | The event stream and individual events. |
| `tokens`  | API tokens.                            |
| `roles`   | Roles.                                 |
| `secrets` | The global secret store.               |
| `system`  | System settings.                       |

#### Targets

Targets are regexes that always match the entire id. `"namespace": "default"` matches `default` but not
`not-default`; use `default.*` for a prefix match. Writing `^` and `$` yourself is fine but not needed. Use `.*` to
match everything.

Gofer rejects roles with empty or invalid targets, grants without any resources or actions, and resources that don't
belong to the grant's group.

When a route lists things, like listing namespaces, the token only needs a matching grant of the right kind. The
results are then filtered down to the items the token is allowed to read.

#### Actions

There are three actions: `read`, `write` and `delete`. Each route belongs to one of them, which generally follows the
HTTP method: `GET` is `read`, `POST` and `PATCH` are `write`, and `DELETE` is `delete`. Cancelling a run or task
execution is a `delete`, and attaching to a task execution is a `write`.

#### Admin only routes

Some routes can only be used by tokens with the `admin` or `bootstrap` role, regardless of what grants a token has.
These include managing namespaces, tokens, roles, extensions, global secrets, and system settings.

### Creating roles

`gofer role create <id> <description> --file role.json` creates a role from a JSON file containing the `grants`
object. Without `--file`, the CLI prompts for each grant. `gofer role update <id> --file role.json` replaces a role's
grants.

### System roles

| Role        | Access                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------ |
| `bootstrap` | Everything. Given to the first token created; it can't be assigned to any other token.                 |
| `admin`     | Everything.                                                                                            |
| `user`      | All namespace resources with read, write, and delete in the `default` namespace. Can also follow events for the whole system. |
| `anonymous` | Read only access to pipelines and runs in the `default` namespace. Used for requests without a token on routes that allow it. |

Gofer also creates a few roles automatically:

- `extension_<extension_id>`: given to each extension. It can use its own object store and read its own
  subscriptions, start runs in any pipeline, read most pipeline information, and follow events.
- `inject_api_token_<namespace_id>_<pipeline_id>`: given to tokens created by a pipeline's `inject_api_token` option.
  It can read and write its own pipeline's runs, objects, and configs, read pipelines in the `default` namespace and
  its own namespace, and follow events.

### Errors

- `401 Unauthorized` means the request didn't include a valid token: it's missing, unknown, disabled, or expired.
- `403 Forbidden` means the token is valid but doesn't have permission for the route. The response explains what the
  route needs.

## Authentication

Before you can start using Gofer, you need to obtain an API token. You can retrieve the
bootstrap token using the `gofer token bootstrap` command or by making a request to the `/api/tokens/bootstrap` route.

Once a bootstrap token is collected it can no longer be collected.

### How to auth via the API

Gofer requires two headers for successful authentication:

- `Authorization: Bearer <token>`
- `gofer-api-version: v<version_number>`

### How to auth via the CLI

The Gofer CLI accepts [multiple methods for setting a token once you have one.](../../cli/configuration.md)
