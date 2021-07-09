## gofer service init-config

Create example gofer config file.

### Synopsis

Create example gofer config file.

This file can be used as a example starting point and be customized further. This file should not
be used to run production versions of Gofer as it is inherently insecure.

The default filename is example.gofer.hcl, but can be renamed via flags.

```
gofer service init-config [flags]
```

### Examples

```
$ gofer service init-config
$ gofer service init-config -f myServer.hcl
```

### Options

```
  -f, --filepath string   path to file (default "./example.gofer.hcl")
  -h, --help              help for init-config
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
