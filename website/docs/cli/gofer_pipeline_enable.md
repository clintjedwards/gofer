## gofer pipeline enable

Enable pipeline

### Synopsis

Enable pipeline.

This restores a previously disabled pipeline.

```
gofer pipeline enable <id> [flags]
```

### Examples

```
$ gofer pipeline enable simple_test_pipeline
```

### Options

```
  -h, --help   help for enable
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
