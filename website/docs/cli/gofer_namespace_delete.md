## gofer namespace delete

Delete namespace

### Synopsis

Delete namespace.

Namespaces can only be deleted if all pipelines are abandoned.

```
gofer namespace delete <id> [flags]
```

### Examples

```
$ gofer namespace delete my_namespace
```

### Options

```
  -h, --help   help for delete
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
