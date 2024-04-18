# Docker <small>scheduler</small>

The docker scheduler uses the machine's local docker engine to run containers. This is great for small or development workloads and very simple to implement. Simply download docker and go!

```toml
[scheduler]
engine = "docker"

[scheduler.docker]
prune = true
prune_interval = 604800
timeout = 300
```

## Configuration

Docker needs to be installed and the Gofer process needs to have the required permissions to run containers upon it.

Other than that the docker scheduler just needs to know how to clean up after itself.

| Parameter      | Type | Default | Description                                                                                                                                                                                                     |
| -------------- | ---- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| prune          | bool | true    | Whether or not to periodically clean up containers that are no longer in use. If prune is not turned on eventually the disk of the host machine will fill up with different containers that have run over time. |
| prune_interval | int  | 604800  | How often to run the prune job. Depending on how many containers you run per day this value could easily be set to monthly.                                                                                     |
| timeout        | int  | 300     | The timeout for the request to the docker service. Should be the same or more than the task_execution_stop_timeout                                                                                              |
