## gofer run start

Start a new run

```
gofer run start <pipeline_id> [flags]
```

### Examples

```
$ gofer run start simple_test_pipeline
```

### Options

```
  -h, --help           help for start
  -o, --only strings   Run only theses tasks
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

- [gofer run](gofer_run.md) - Manage runs
