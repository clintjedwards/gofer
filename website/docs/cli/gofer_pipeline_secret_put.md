## gofer pipeline secret put

Write a secret into the pipeline secret store

### Synopsis

You can store both regular text values or read in entire files using the '@' prefix.

```
gofer pipeline secret put <pipeline_id> <key>=<object> [flags]
```

### Examples

```
$ gofer secret put simple_test_pipeline my_key=my_value
$ gofer secret put simple_test_pipeline my_key=@/test/folder/file_path
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

- [gofer pipeline secret](gofer_pipeline_secret.md) - Secret pipeline specific values
