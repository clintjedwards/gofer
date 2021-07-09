## gofer namespace list

List all namespaces

### Synopsis

List all namespaces.

Namespaces act as divider lines between different sets of pipelines.

```
gofer namespace list [flags]
```

### Examples

```
$ gofer namespace list
```

### Options

```
  -h, --help   help for list
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
