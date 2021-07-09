## gofer run

Manage runs

### Synopsis

Manage runs.

A "run" is a single instance of a pipeline's execution. It consists of a collection of tasks that can be
all run in parallel or depend on the execution of others.

### Options

```
  -h, --help   help for run
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

- [gofer](gofer.md) - Gofer is a distributed, continuous thing do-er.
- [gofer run cancel](gofer_run_cancel.md) - Cancel a run in progress
- [gofer run cancel-all](gofer_run_cancel-all.md) - CancelAll cancels all run for a given pipeline
- [gofer run get](gofer_run_get.md) - Get details on a specific run
- [gofer run list](gofer_run_list.md) - List all runs
- [gofer run start](gofer_run_start.md) - Start a new run
- [gofer run store](gofer_run_store.md) - Store run specific values
