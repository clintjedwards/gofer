## gofer service

Manages service related commands for Gofer.

### Synopsis

Manages service related commands for the Gofer Service/API.

These commands help with managing and running the Gofer service.

### Options

```
  -h, --help   help for service
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
- [gofer service init-config](gofer_service_init-config.md) - Create example gofer config file.
- [gofer service printenv](gofer_service_printenv.md) - Print the list of environment variables the server looks for on startup.
- [gofer service start](gofer_service_start.md) - Start the Gofer GRPC/HTTP combined server
- [gofer service toggle-event-ingress](gofer_service_toggle-event-ingress.md) - Allows the operator to control run ingress
- [gofer service token](gofer_service_token.md) - Manage api tokens
