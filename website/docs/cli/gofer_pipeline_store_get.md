## gofer pipeline store get

Read an object from the pipeline store

```
gofer pipeline store get <pipeline_id> <key> [flags]
```

### Examples

```
$ gofer pipeline store get simple_test_pipeline my_key
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

- [gofer pipeline store](gofer_pipeline_store.md) - Store pipeline specific values
