## gofer pipeline secrets

Store pipeline secrets

### Synopsis

Store pipeline secrets.

Gofer allows you to store pipeline secrets. These secrets are then used to populate the pipeline
configuration file.

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
- [gofer pipeline store get](gofer_pipeline_secret_get.md) - Read a secret from the pipeline secret store
- [gofer pipeline store put](gofer_pipeline_secret_put.md) - Write a secret into the pipeline secret store
