## gofer trigger get

Get a specific trigger by name.

```
gofer trigger get <name> [flags]
```

### Examples

```
$ gofer trigger get cron
```

### Options

```
  -h, --help   help for get
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

- [gofer trigger](gofer_trigger.md) - Get details about Gofer triggers
