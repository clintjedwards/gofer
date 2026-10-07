# Authentication and Authorization

Gofer's authentication and authorization systems are designed to be lightweight and unobtrusive,
allowing you to focus on your tasks with minimal interference. The permissioning system is based on
Role-Based Access Control (RBAC) and is primarily used to prevent certain token holders, such as extensions,
from accessing excessive parts of the system.

For most users, this means the ability to categorize users into distinct groups, typically at the namespace or pipeline level.

Authorization in Gofer is token-based, a principle that extends seamlessly to the frontend as well.

## Authorization

Gofer organizes permissions into roles, which are then assigned to specific tokens. These tokens grant access based on
the permissions associated with their roles.

### Roles

Roles are collections of permissions. The most significant role is the bootstrap role, which is a special role.
The bootstrap role is equivalent to a root role and is assigned to the first token you receive within Gofer.

Gofer also includes 'system roles' which are special roles that cannot be modified or removed. These roles are
generally used by other components of Gofer for specific purposes or are there for your convenience.

You can view a list of these roles by using the `gofer role list` command and identifying the roles
where `system_role` is marked as `true`.

### Permissions

A permission is made up of two parts: a list of "resources" and a list of "actions". A role can contain many
permissions.

**Each permission is a standalone grant.** For a request to be allowed, a single permission has to cover every
resource the route needs, along with the route's action. Gofer never combines separate permissions to satisfy a
route. For example, take this role:

```json
"permissions": [
  { "actions": ["read"], "resources": ["namespaces:^frontend$", "pipelines:^website$"] },
  { "actions": ["read"], "resources": ["namespaces:^backend$", "pipelines:^billing$"] }
]
```

This allows reading `frontend/website` and `backend/billing`, but not `frontend/billing`. If you want one permission
to cover several things, put all of them in the same permission.

A token can have several roles. The permissions from all of its roles are pooled together, and the request is allowed
if any one of them covers the route.

#### Resources

Resources are groups of related routes within Gofer. They are written as plain strings:

| Resource               | Covers                                                                        |
| ---------------------- | ----------------------------------------------------------------------------- |
| `all`                  | Every resource.                                                               |
| `configs`              | Pipeline configurations.                                                      |
| `deployments`          | Pipeline deployments.                                                         |
| `events`               | The event stream and individual events.                                       |
| `extensions:<target>`  | Extensions, targeted by extension id.                                         |
| `namespaces:<target>`  | Namespaces, targeted by namespace id.                                         |
| `objects`              | Pipeline, run, and extension object stores.                                   |
| `permissions`          | Roles.                                                                        |
| `pipelines:<target>`   | Pipelines, targeted by pipeline id.                                           |
| `runs`                 | Runs.                                                                         |
| `secrets`              | Pipeline and global secret stores.                                            |
| `subscriptions`        | Pipeline extension subscriptions.                                             |
| `system`               | System settings.                                                              |
| `task_executions`      | Task executions, their logs, and attaching to them.                           |
| `tokens`               | API tokens.                                                                   |

Routes usually need more than one resource. Reading a pipeline's secrets, for instance, needs
`namespaces:<namespace>`, `pipelines:<pipeline>` and `secrets` in the same permission.

#### Targets

`extensions`, `namespaces` and `pipelines` require a target, which is a regex that the resource's id is matched
against. A few rules:

- Targets always match the entire id. `namespaces:default` matches `default` but not `not-default`; use
  `namespaces:default.*` if you want a prefix match. Writing `^` and `$` yourself is fine but not needed.
- A target is required. Use `.*` to match everything, e.g. `pipelines:.*`.
- The other resources don't take a target.
- Gofer rejects roles with unknown resources, missing targets, or targets that aren't valid regex.

When a route lists things, like listing namespaces, the token only needs a permission for that resource type. The
results are then filtered down to the items the token is allowed to read.

Here is a role that grants full access to every namespace starting with "devops":

```json
POST https://gofer.clintjedwards.com/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "devops",
  "description": "Access only to namespaces that start with devops",
  "permissions": [
    {
      "actions": ["read", "write", "delete"],
      "resources": [
        "namespaces:devops.*",
        "pipelines:.*",
        "configs",
        "deployments",
        "objects",
        "runs",
        "secrets",
        "subscriptions",
        "task_executions"
      ]
    }
  ]
}
```

#### Actions

There are three actions: `read`, `write` and `delete`. Each route belongs to one of them, which generally follows the
HTTP method: `GET` is `read`, `POST` and `PATCH` are `write`, and `DELETE` is `delete`. Cancelling a run or task
execution is a `delete`.

#### Admin only routes

Some routes can only be used by tokens with the `admin` or `bootstrap` role, regardless of what other permissions a
token has. These include managing namespaces, tokens, roles, extensions, global secrets, and system settings.

### System roles

| Role        | Access                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------ |
| `bootstrap` | Everything. Given to the first token created; it can't be assigned to any other token.                 |
| `admin`     | Everything.                                                                                            |
| `user`      | Read, write, and delete for pipelines in the `default` namespace and everything under them (configs, deployments, objects, runs, secrets, subscriptions, task executions). Can also follow events for the whole system. |
| `anonymous` | Read only access to pipelines and runs in the `default` namespace. Used for requests without a token on routes that allow it. |

Each extension also gets its own system role named `extension_<extension_id>`. It can use its own object store,
start runs in any pipeline, and read most pipeline information.

### Errors

- `401 Unauthorized` means the request didn't include a valid token: it's missing, unknown, disabled, or expired.
- `403 Forbidden` means the token is valid but doesn't have permission for the route. The response explains which
  resources and action the route needs.

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
