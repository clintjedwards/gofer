## gofer taskrun cancel

Cancel a specific task run

### Synopsis

Cancels a task run by requesting that the scheduler gracefully stops it. Usually this means the scheduler will
pass a SIGTERM to the container. If the container does not shut down within the API defined timeout or the user has passed
the force flag the scheduler will then kill the container immediately.

Cancelling a task run might mean that downstream/dependent task runs are skipped.

```
gofer taskrun cancel <pipeline> <run> <id> [flags]
```

### Examples

```
$ gofer taskrun cancel simple_test_pipeline 23 example_task
```

### Options

```
  -f, --force   Stop job immediately(sigkill/ungraceful shutdown)
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

- [gofer taskrun](gofer_taskrun.md) - Manage taskruns
