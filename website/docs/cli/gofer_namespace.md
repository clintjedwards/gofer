## gofer namespace

Manage namespaces

### Synopsis

Manage namespaces.

A namespace is a divider between sets of pipelines. It's usually common to divide namespaces based on
team or environment or some combination of both.

### Options

```
  -h, --help   help for namespace
```

### Options inherited from parent commands

```
      --config string      configuration file path
      --detail             show extra detail for some commands (ex. Exact time instead of humanized)
      --format string      output format; accepted values are 'pretty', 'json', 'silent'
      --host string        specify the URL of the server to communicate to
      --namespace string   specify which namespace the command should be run against
      --no-color           disable color output
```

### SEE ALSO

- [gofer](gofer.md) - Gofer is a distributed, continuous thing do-er.
- [gofer namespace create](gofer_namespace_create.md) - Create a new namespace
- [gofer namespace delete](gofer_namespace_delete.md) - Delete namespace
- [gofer namespace get](gofer_namespace_get.md) - Get details on a specific namespace
- [gofer namespace list](gofer_namespace_list.md) - List all namespaces
- [gofer namespace update](gofer_namespace_update.md) - Update details on a specific namespace
