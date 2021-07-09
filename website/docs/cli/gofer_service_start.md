## gofer service start

Start the Gofer GRPC/HTTP combined server

### Synopsis

Start the Gofer GRPC/HTTP combined server.

Gofer runs as a GRPC backend combined with GRPC-WEB/HTTP. Running this command attempts to start the long
running service. This command will block and only gracefully stop on SIGINT or SIGTERM signals

```
gofer service start [flags]
```

### Options

```
  -h, --help   help for start
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
