## gofer pipeline

Manage pipelines

### Synopsis

Manage pipelines.

A "pipeline" is a directed acyclic graph of tasks that run together. A single execution of a pipeline is called a
"run".

### Options

```
  -h, --help   help for pipeline
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
- [gofer pipeline abandon](gofer_pipeline_abandon.md) - Abandon pipeline
- [gofer pipeline create](gofer_pipeline_create.md) - Create a new pipeline
- [gofer pipeline disable](gofer_pipeline_disable.md) - Disable pipeline
- [gofer pipeline enable](gofer_pipeline_enable.md) - Enable pipeline
- [gofer pipeline get](gofer_pipeline_get.md) - Get details on a specific pipeline
- [gofer pipeline list](gofer_pipeline_list.md) - List all pipelines
- [gofer pipeline store](gofer_pipeline_store.md) - Store pipeline specific values
- [gofer pipeline update](gofer_pipeline_update.md) - Update pipeline
