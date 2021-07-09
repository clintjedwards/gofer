## gofer run cancel

Cancel a run in progress

```
gofer run cancel <pipeline> <id> [flags]
```

### Examples

```
$ gofer run cancel simple_test_pipeline 3
```

### Options

```
  -f, --force   Stop run and child taskrun containers immediately (SIGKILL)
  -h, --help    help for cancel
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
