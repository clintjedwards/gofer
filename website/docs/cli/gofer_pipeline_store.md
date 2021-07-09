## gofer pipeline store

Store pipeline specific values

### Synopsis

Store pipeline specific values.

Gofer has two ways to temporarily store objects that might be useful.

This command allows users to store objects at the "pipeline" level in a key-object fashion. Pipeline level objects are
great for storing things that need to be cached over many runs and don't change very often.

Pipeline objects are kept forever until the limit of number of pipeline objects is reached(this may be different depending on configuration).
Once this limit is reached the _oldest_ object will be removed to make space for the new object.

This "oldest is evicted" rule does not apply to objects which are being overwritten. So replacing an already populated key with
a newer object would not cause any object deletions even at the object limit.

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

- [gofer pipeline](gofer_pipeline.md) - Manage pipelines
- [gofer pipeline store get](gofer_pipeline_store_get.md) - Read an object from the pipeline store
- [gofer pipeline store put](gofer_pipeline_store_put.md) - Write an object into the pipeline store
