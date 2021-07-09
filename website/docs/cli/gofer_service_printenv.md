## gofer service printenv

Print the list of environment variables the server looks for on startup.

### Synopsis

Print the list of environment variables the server looks for on startup.

This is helpful for setting variables for controlling how the server should work.

All configuration set by environment variable overrides default and config file read configuration.

```
gofer service printenv [flags]
```

### Options

```
  -h, --help   help for printenv
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

- [gofer service](gofer_service.md) - Manages service related commands for Gofer.
