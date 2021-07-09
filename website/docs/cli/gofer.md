## gofer

Gofer is a distributed, continuous thing do-er.

### Synopsis

Gofer is a distributed, continuous thing do-er.

It uses a similar model to concourse(https://concourse-ci.org/), leveraging the docker container as a key mechanism
to run short-lived workloads. The benefits of this is simplicity. No foreign agents, no cluster setup, just run
containers.

Read more at https://clintjedwards.com/gofer

### Global Options

```
      --config string      configuration file path
      --detail             show extra detail for some commands (ex. Exact time instead of humanized)
      --format string      output format; accepted values are 'pretty', 'json', 'silent'
  -h, --help               help for gofer
      --host string        specify the URL of the server to communicate to
      --namespace string   specify which namespace the command should be run against
      --no-color           disable color output
```

### SEE ALSO

- [gofer config](gofer_config.md) - Manage pipeline configuration files
- [gofer namespace](gofer_namespace.md) - Manage namespaces
- [gofer pipeline](gofer_pipeline.md) - Manage pipelines
- [gofer run](gofer_run.md) - Manage runs
- [gofer service](gofer_service.md) - Manages service related commands for Gofer.
- [gofer taskrun](gofer_taskrun.md) - Manage taskruns
- [gofer trigger](gofer_trigger.md) - Get details about Gofer triggers
