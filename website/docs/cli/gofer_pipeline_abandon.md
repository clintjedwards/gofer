## gofer pipeline abandon

Abandon pipeline

### Synopsis

Abandon a pipeline.

Abandoning a pipeline marks it for deletion and removes it from all lists. The pipeline may still be readable for a
short time.

```
gofer pipeline abandon <id> [flags]
```

### Examples

```
$ gofer pipeline abandon simple_test_pipeline
```

### Options

```
  -h, --help   help for abandon
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
