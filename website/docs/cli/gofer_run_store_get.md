## gofer run store get

Read an object from the run specific store

```
gofer run store get <pipeline_id> <run_id> <key> [flags]
```

### Examples

```
$ gofer run store get simple_test_pipeline 5 my_key
```

### Options

```
  -h, --help        help for get
  -s, --stringify   Attempt to print the object as a string
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

- [gofer run store](gofer_run_store.md) - Store run specific values
