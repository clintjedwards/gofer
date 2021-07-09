## gofer namespace update

Update details on a specific namespace

```
gofer namespace update <id> [flags]
```

### Examples

```
$ gofer namespace update old_namespace --name="New name"
```

### Options

```
  -d, --description string   Description on use for namespace
  -h, --help                 help for update
  -n, --name string          Human readable name for namespace
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
