## gofer run store put

Write an object into the run store

### Synopsis

Write an object into the run store.

The run store allows storage of objects as key-values pairs that individual runs might need to reference. These run
level objects allow for more objects to be stored than their pipeline counterparts, but are kept on a much shorter
time scale. Run level objects are removed once their run limit is reached(this may be different depending on
configuration). This run limit is related to the number of runs in a pipeline.

For instance, after a run is 10 runs old, gofer may clean up its objects.

You can store both regular text values or read in entire files using the '@' prefix.

```
gofer run store put <pipeline_id> <run_id> <key>=<object> [flags]
```

### Examples

```
$ gofer store put simple_test_pipeline my_key=my_value
$ gofer store put simple_test_pipeline my_key=@file_path
```

### Options

```
  -f, --force   replace value if exists
  -h, --help    help for put
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
