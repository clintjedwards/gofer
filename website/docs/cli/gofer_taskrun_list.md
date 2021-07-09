## gofer taskrun list

List all taskruns

### Synopsis

List all taskruns.

A short listing of all task runs for a specific run.

```
gofer taskrun list <pipeline> <run> [flags]
```

### Examples

```
$ gofer taskrun list simple_test_pipeline 15
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

- [gofer taskrun](gofer_taskrun.md) - Manage taskruns
