## gofer service token create

Create new API token

```
gofer service token create <management|client> [flags]
```

### Options

```
  -h, --help                 help for create
  -m, --metadata strings     metadata about the token, useful for attaching a name, team, and other details. Format = key:value
  -n, --namespaces strings   namespaces this key will have access to. If not specified namespace is default (default [default])
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

- [gofer service token](gofer_service_token.md) - Manage api tokens
