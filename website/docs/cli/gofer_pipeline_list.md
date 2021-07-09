## gofer pipeline list

List all pipelines

### Synopsis

List all pipelines.

A short listing of all currently registered pipelines.

Health shows a quick glimpse into how the last 5 builds performed.

- Unstable = There is a failure in the last 5 builds.
- Poor = Past 5 builds have all failed.
- Good = Past 5 builds have all passed.

```
gofer pipeline list [flags]
```

### Examples

```
$ gofer pipeline list
```

### Options

```
  -h, --help   help for list
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
