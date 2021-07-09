## gofer config fmt

Format pipeline configuration files

### Synopsis

Format pipeline configuration files.

A basic HCL formatter. Rewrites the file in place.

```
gofer config fmt <...path> [flags]
```

### Examples

```
$ gofer config fmt myPipeline.hcl
$ gofer config fmt myPipeline.hcl anotherFile.hcl
$ gofer config fmt somedir/*
```

### Options

```
  -h, --help   help for fmt
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
