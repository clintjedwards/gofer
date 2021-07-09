## gofer service registry add

create a new docker registry auth

```
gofer service registry add <registry> <user> [flags]
```

### Examples

```
$ gofer service ghcr.io/clintjedwards/gofer my-user
```

### Options

```
  -h, --help   help for add
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

- [gofer service registry](gofer_service_registry.md) - Manage docker registry authentication
