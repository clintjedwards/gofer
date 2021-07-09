## gofer service toggle-event-ingress

Allows the operator to control run ingress

### Synopsis

Allows the operator to control whether it is possible to start new runs on the Gofer service or not

```
gofer service toggle-event-ingress [flags]
```

### Examples

```
$ gofer service toggle-event-ingress
```

### Options

```
  -h, --help   help for toggle-event-ingress
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

- [gofer service](gofer_service.md) - Manages service related commands for Gofer.
