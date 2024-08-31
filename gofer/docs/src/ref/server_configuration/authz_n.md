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

Permissions in Gofer are composed of two key components: "Resources" and "Actions."

#### Resources

Resources refer to specific groups of objects or collections within Gofer. Below is an example list of Gofer resources:

```rust
pub enum Resource {
    Configs,
    Deployments,
    Events,
    Extensions(String),
    Namespaces(String),
    Objects,
    Permissions,
    Pipelines(String),
    Runs,
    Secrets,
    Subscriptions,
    System,
    TaskExecutions,
    Tokens,
}
```

Some resources may include what Gofer refers to as "targets." Targets allow the creator of the permission
to specify particular resources or a set of resources. The true power of targets lies in their ability to leverage
regular expressions (regex).

For example, you might want to grant access to all namespaces that begin with a specific prefix.

To achieve this, you would create a role similar to the following:

```json
# Create a new role with permissions limited to the devops namespaces.
POST https://gofer.clintjedwards.com/api/roles
gofer-api-version: v0
Content-Type: application/json
Authorization: Bearer {{secret}}
{
  "id": "devops",
  "description": "Access only to namespaces that start with devops",
  "permissions": [
    {
      "actions": [
        "read",
        "write",
        "delete"
      ],
      "resources": [
        { "resource": "namespaces", "target": "^devops.*" },
        { "resource": "pipelines", "target": ".*" }
      ]
    }
  ]
}
HTTP 201
```
This grants full read, write, and delete permissions for any namespace that begins with "devops"

#### Actions

Actions are straightforward. Each route in Gofer is associated with an action, which typically corresponds to the
HTTP method used (e.g., GET, POST, DELETE). Only tokens with the correct combination of resource and
action are permitted to proceed.

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
