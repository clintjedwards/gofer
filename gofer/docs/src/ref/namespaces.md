# Namespaces

Namespaces divide pipelines into groups, usually one per team or project. Every pipeline lives in exactly one
namespace, and pipeline ids only have to be unique within their namespace.

Gofer starts with a single namespace called `default`, which is where everything goes unless you say otherwise. If
you're the only one using Gofer, you may never need another.

## Creating a namespace

Only admins can create, update, or delete namespaces:

```bash
gofer namespace create frontend "Frontend Team" "Pipelines for the website and design system"
gofer namespace list
```

Like pipeline ids, namespace ids can only contain letters, numbers, and hyphens, and have to be 3 to 32 characters
long.

## Working in a namespace

CLI commands work in your default namespace, which is `default` unless you change it. There are two ways to use a
different one:

- Pass `--namespace` to a single command: `gofer up --namespace frontend ./website`.
- Change your default by setting `namespace` in your [CLI configuration](../cli/configuration.md), or the
  `GOFER_NAMESPACE` environment variable.

## What namespaces control

- **Permissions.** Roles grant access by namespace, using a regex like `frontend` or `ops-.*`. This is how you give
  a team full access to its own pipelines without access to anyone else's. See
  [Authentication and Authorization](./server_configuration/authz_n.md). The built in `user` role only covers the
  `default` namespace, so tokens for other namespaces need a role of their own.
- **Global secrets.** Each global secret lists which namespaces can use it, so a team's secrets can be shared
  across its pipelines without being visible to other teams. See [Secret Store](./secret_store/index.html).
