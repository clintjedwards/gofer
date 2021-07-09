## gofer service registry

Manage docker registry authentication

### Synopsis

Gofer manages access to private docker registries through pre-registering credentials for that registry using
these tools. Once registered any downstream pipelines using images from these registries are passed along the credentials
mentioned here.

### Options

```
  -h, --help   help for registry
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
- [gofer service registry add](gofer_service_registry_add.md) - create a new docker registry auth
- [gofer service registry list](gofer_service_registry_list.md) - List all Docker registry auths
- [gofer service registry remove](gofer_service_registry_remove.md) - Remove Docker registry auth
