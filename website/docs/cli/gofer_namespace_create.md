## gofer namespace create

Create a new namespace

### Synopsis

Create a new namespace.

Namespaces act as divider lines between different sets of pipelines.

```
gofer namespace create <id> <name> [flags]
```

### Examples

```
$ gofer namespace create new_namespace "New Namespace"
$ gofer namespace create new_namespace "New Namespace" --description="my new namespace"

```

### Options

```
  -d, --description string   Description on use for namespace
  -h, --help                 help for create
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

- [gofer namespace](gofer_namespace.md) - Manage namespaces
