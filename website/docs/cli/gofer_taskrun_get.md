## gofer taskrun get

Get details on a specific task run

```
gofer taskrun get <pipeline> <run> <id> [flags]
```

### Examples

```
$ gofer taskrun get simple_test_pipeline 23 example_run
```

### Options

```
  -h, --help   help for get
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
