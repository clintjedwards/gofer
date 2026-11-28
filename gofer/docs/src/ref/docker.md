# Docker

Gofer runs every task and extension as a container on the machine's local Docker engine. Simply download docker and go!

```toml
[docker]
prune = true
prune_interval_seconds = 604800
timeout_seconds = 300
```

## Configuration

Docker needs to be installed and the Gofer process needs to have the required permissions to run containers upon it.

Task images come from Gofer, not from a registry. Before a task runs, Gofer loads its image into Docker from its own
[image store](./server_configuration/configuration_reference.md#images), named `gofer-image:<digest>`, and it
removes those once no version of any pipeline keeps them. Extensions whose image is stored in Gofer, which includes
Gofer's own, are loaded the same way. An extension whose manifest names a registry image is pulled from that registry;
see [where an extension's image comes from](./extensions/index.html#where-an-extensions-image-comes-from).

Each run gets a Docker volume of its own, which is the folder its tasks share at `/gofer/run`. Gofer removes it when
the run ends, along with the run's task containers, since Docker won't remove a volume while any container has it.
Every hour Gofer also looks for any that a crash left behind. The volume takes space in Docker's own storage, usually
under `/var/lib/docker`, and Docker can't limit its size.

Every task also gets [the gofer CLI](./pipeline_configuration/tasks.md#the-gofer-cli): Gofer mounts its own binary,
read only, at `/gofer/bin/gofer`. It's a bind mount of the file Gofer was started from.

`gofer up` uses Docker as well, on the machine it runs on, to build task images. It runs
[BuildKit](https://github.com/moby/buildkit) in a container called `gofer-buildkit`, with its cache in a volume, and
`gofer builder --help` shows how to empty or remove it.

Other than that Gofer just needs to know how to clean up after itself.

| Parameter              | Type | Default | Description                                                                                                                                                                                                     |
| ---------------------- | ---- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| prune                  | bool | true    | Whether or not to periodically clean up containers that are no longer in use. If prune is not turned on eventually the disk of the host machine will fill up with different containers that have run over time. |
| prune_interval_seconds | int  | 604800  | How often to run the prune job. Depending on how many containers you run per day this value could easily be set to monthly.                                                                                     |
| timeout_seconds        | int  | 300     | The timeout for the request to the docker service. Should be at least as long as `api.task_execution_stop_timeout_seconds`. |
