## gofer pipeline disable

Disable pipeline

### Synopsis

Disable pipeline.

This will prevent the pipeline from running any more jobs and events passed to the pipeline
will be discarded.

```
gofer pipeline disable <id> [flags]
```

### Examples

```
$ gofer pipeline disable simple_test_pipeline
```

### Options

```
  -h, --help   help for disable
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

- [gofer pipeline](gofer_pipeline.md) - Manage pipelines
