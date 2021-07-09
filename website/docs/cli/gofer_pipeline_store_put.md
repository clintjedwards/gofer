## gofer pipeline store put

Write an object into the pipeline store

### Synopsis

Write an object into the pipeline store.

The pipeline store allows storage of objects as key-values pairs that many runs might need to reference. These pipeline
level objects are kept forever until the limit of number of pipeline objects is reached(this may be different depending
on configuration). Once this limit is reached the _oldest_ object will be removed to make space for the new object.

You can store both regular text values or read in entire files using the '@' prefix.

```
gofer pipeline store put <pipeline_id> <key>=<object> [flags]
```

### Examples

```
$ gofer store put simple_test_pipeline my_key=my_value
$ gofer store put simple_test_pipeline my_key=@/test/folder/file_path
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

- [gofer pipeline store](gofer_pipeline_store.md) - Store pipeline specific values
