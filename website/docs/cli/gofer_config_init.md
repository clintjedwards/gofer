## gofer config init

Create example pipeline config file

### Synopsis

Create example pipeline configuration file.

This file can be used as a example starting point and be customized further.

The default filename is example.gofer.hcl, but can be renamed via flags.

```
gofer config init [flags]
```

### Examples

```
$ gofer config init
$ gofer config -f myPipeline.hcl
```

### Options

```
  -f, --filepath string   path to file (default "./example.gofer.hcl")
  -h, --help              help for init
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

- [gofer config](gofer_config.md) - Manage pipeline configuration files
