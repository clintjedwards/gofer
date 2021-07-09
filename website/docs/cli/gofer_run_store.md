## gofer run store

Store run specific values

### Synopsis

Store run specific values.

Gofer has two ways to temporarily store objects that might be useful.

This command allows users to store objects at the "run" level in a key-object fashion. Run level objects are
great for storing things that need to be cached only for the communication between tasks.

Run objects are kept individual to each run and removed after a certain run limit. This means that after a certain
amount of runs for a particular pipeline a run's objects will be discarded. The limit of amount of objects you can
store per run is of a much higher limit.

### Options

```
  -h, --help   help for store
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
- [gofer run store get](gofer_run_store_get.md) - Read an object from the run specific store
- [gofer run store put](gofer_run_store_put.md) - Write an object into the run store
