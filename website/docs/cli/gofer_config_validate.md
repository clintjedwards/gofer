## gofer config validate

Validate pipeline configuration files

```
gofer config validate <...path> [flags]
```

### Examples

```
$ gofer config validate myPipeline.hcl
$ gofer config validate myPipeline.hcl anotherFile.hcl
$ gofer config validate somedir/*
```

### Options

```
  -h, --help   help for validate
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
