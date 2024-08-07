# Configuration Reference

## This page might be outdated.

Gofer has a variety of parameters that can be specified via environment variables or the configuration file.

To view a list of all possible environment variables simply type: `gofer service start -h`.

The most up to date config file values can be found by
[reading the code](https://github.com/clintjedwards/gofer/blob/main/gofer/src/scheduler/mod.rs) or running the
command above, but a best effort key and description list is given below.

If examples of these values are needed you can find a sample file by using `gofer service init-config`.

## Values

### API

| name                        | type              | default | description                                                                                                                                                                                                                                                                     |
| --------------------------- | ----------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| event_log_retention         | string (duration) | 4380h   | Controls how long Gofer will hold onto events before discarding them. This is important factor in disk space and memory footprint. Example: Rough math on a 5,000 pipeline Gofer instance with a full 6 months of retention puts the memory and storage footprint at about 9GB. |
| event_prune_interval        | string            | 3h      | How often to check for old events and remove them from the database. Will only remove events older than the value in event_log_retention.                                                                                                                                       |
| ignore_pipeline_run_events  | boolean           | false   | Controls the ability for the Gofer service to execute jobs on startup. If this is set to false you can set it to true manually using the CLI command `gofer service toggle-event-ingress`.                                                                                      |
| log_level                   | string            | debug   | The logging level that is output. It is common to start with `info`.                                                                                                                                                                                                            |
| run_parallelism_limit       | int               | N/A     | The limit automatically imposed if the pipeline does not define a limit. 0 is unlimited.                                                                                                                                                                                        |
| task_execution_logs_dir     | string            | /tmp    | The path of the directory to store task execution logs. Task execution logs are stored as a text file on the server.                                                                                                                                                            |
| task_execution_log_expiry   | int               | 20      | The total amount of runs before logs of the oldest run will be deleted.                                                                                                                                                                                                         |
| task_execution_stop_timeout | string            | 5m      | The amount of time Gofer will wait for a container to gracefully stop before sending it a SIGKILL.                                                                                                                                                                              |
| external_events_api         | block             | N/A     | The external events API controls webhook type interactions with extensions. HTTP requests go through the events endpoint and Gofer routes them to the proper extension for handling.                                                                                            |
| object_store                | block             | N/A     | The settings for the Gofer object store. The object store assists Gofer with storing values between tasks since Gofer is by nature distributed. This helps jobs avoid having to download the same objects over and over or simply just allows tasks to share certain values.    |
| secret_store                | block             | N/A     | The settings for the Gofer secret store. The secret store allows users to securely populate their pipeline configuration with secrets that are used by their tasks, extension configuration, or scheduler.                                                                      |
| scheduler                   | block             | N/A     | The settings for the container orchestrator that Gofer will use to schedule workloads.                                                                                                                                                                                          |
| server                      | block             | N/A     | Controls the settings for the Gofer API service properties.                                                                                                                                                                                                                     |
| extensions                  | block             | N/A     | Controls settings for Gofer's extension system. Extensions are different workflows for running pipelines usually based on some other event (like the passing of time).                                                                                                          |


#### Example

```toml
[api]
ignore_pipeline_run_events = false
run_parallelism_limit = 200
pipeline_version_retention = 10
event_log_retention = 15768000     # 6 months
event_prune_interval = 604800      # 1 week
log_level = "info"
task_execution_log_retention = 50  # total runs
task_execution_logs_dir = "/tmp"
task_execution_stop_timeout = 300  # 5 mins
admin_key = "test"
```

### Development (block)

Special feature flags to make development easier

| name               | type    | default | description                                                                |
| ------------------ | ------- | ------- | -------------------------------------------------------------------------- |
| bypass_auth        | boolean | false   | Skip authentication for all routes.                                        |
| default_encryption | boolean | false   | Use default encryption key to avoid prompting for a unique one.            |
| pretty_logging     | boolean | false   | Turn on human readable logging instead of JSON.                            |
| use_localhost_tls  | boolean | false   | Use embedded localhost certs instead of prompting the user to provide one. |

#### Example

```toml
[development]
pretty_logging = true     # Tells the logging package to use human readable output.
bypass_auth = true        # Turns off auth.
use_included_certs = true # Automatically loads localhost certs for development.
```

### External Events API (block)

The external events API controls webhook type interactions with extensions. HTTP requests go through the events
endpoint and Gofer routes them to the proper extension for handling.

| name   | type    | default        | description                                                                               |
| ------ | ------- | -------------- | ----------------------------------------------------------------------------------------- |
| enable | boolean | true           | Enable the events api. If this is turned off the events http service will not be started. |
| host   | string  | localhost:8081 | The address and port to bind the events service to.                                       |

#### Example

```toml
[external_events]
enable = true
bind_address = "0.0.0.0:8081"
use_tls = false
````

### Object Store (block)

The settings for the Gofer object store. The object store assists Gofer with storing values between tasks since Gofer is by nature distributed. This helps jobs avoid having to download the same objects over and over or simply just allows tasks to share certain values.

You can find [more information on the object store block here.](../object_store/index.html)

| name                  | type   | default | description                                                                                                                                                                                                                                                                                                          |
| --------------------- | ------ | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| engine                | string | sqlite  | The engine Gofer will use to store state. The accepted values here are "sqlite".                                                                                                                                                                                                                                     |
| pipeline_object_limit | int    | 50      | The limit to the amount of objects that can be stored at the pipeline level. Objects stored at the pipeline level are kept permanently, but once the object limit is reach the oldest object will be deleted.                                                                                                        |
| run_object_expiry     | int    | 50      | Objects stored at the run level are unlimited in number, but only last for a certain number of runs. The number below controls how many runs until the run objects for the oldest run will be deleted. Ex. an object stored on run number #5 with an expiry of 2 will be deleted on run #7 regardless of run health. |

#### Sqlite (block)

The sqlite store is a built-in, easy to use object store. It is meant for development and small deployments.

| name   | type   | default              | description                                                                                  |
| ------ | ------ | -------------------- | -------------------------------------------------------------------------------------------- |
| path   | string | /tmp/gofer-object.db | The path of the file that sqlite will use. If this file does not exist Gofer will create it. |
| sqlite | block  | N/A                  | The sqlite storage engine.                                                                   |

```toml
[object_store]
engine = "sqlite"
pipeline_object_limit = 50
run_object_expiry = 50

[object_store.sqlite]
path = "/tmp/gofer_objects.db"
```

### Secret Store (block)

The settings for the Gofer secret store. The secret store allows users to securely populate their pipeline configuration with secrets that are used by their tasks, extension configuration, or scheduler.

You can find [more information on the secret store block here.](../secret_store/index.html)

| name   | type   | default | description                                                                      |
| ------ | ------ | ------- | -------------------------------------------------------------------------------- |
| engine | string | sqlite  | The engine Gofer will use to store state. The accepted values here are "sqlite". |
| sqlite | block  | N/A     | The sqlite storage engine.                                                       |

#### Sqlite (block)

The sqlite store is a built-in, easy to use object store. It is meant for development and small deployments.

| name           | type   | default                            | description                                                                                                                                                                                                            |
| -------------- | ------ | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| path           | string | /tmp/gofer-secret.db               | The path of the file that sqlite will use. If this file does not exist Gofer will create it.                                                                                                                           |
| encryption_key | string | "changemechangemechangemechangeme" | Key used to encrypt keys to keep them safe. This encryption key is responsible for facilitating that. It MUST be 32 characters long and cannot be changed for any reason once it is set or else all data will be lost. |

```toml
[secret_store]
engine = "sqlite"

[secret_store.sqlite]
path = "/tmp/gofer_secrets.db"
encryption_key = "changemechangemechangemechangeme"
```

### Scheduler (block)

The settings for the container orchestrator that Gofer will use to schedule workloads.

You can find [more information on the scheduler block here.](../scheduler/index.html)

| name   | type   | default | description                                                                                                                                               |
| ------ | ------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| engine | string | sqlite  | The engine Gofer will use as a container orchestrator. The accepted values here are "docker".                                                             |
| docker | block  | N/A     | [Docker](https://www.docker.com/why-docker) is the default container orchestrator and leverages the machine's local docker engine to schedule containers. |

#### Docker (block)

[Docker](https://www.docker.com/why-docker) is the default container orchestrator and leverages the machine's local docker engine to schedule containers.

| name           | type    | default | description                                                                   |
| -------------- | ------- | ------- | ----------------------------------------------------------------------------- |
| prune          | boolean | false   | Controls if the docker scheduler should periodically clean up old containers. |
| prune_interval | string  | 24h     | Controls how often the prune container job should run.                        |

```toml
[scheduler]
engine = "docker"

[scheduler.docker]
prune = true
prune_interval = 604800
timeout = 300           # Should be the same or more than the task_execution_stop_timeout
```

### Server (block)

Controls the settings for the Gofer service's server properties.

| name                  | type   | default        | description                                                                             |
| --------------------- | ------ | -------------- | --------------------------------------------------------------------------------------- |
| host                  | string | localhost:8080 | The address and port for the service to bind to.                                        |
| shutdown_timeout      | string | 15s            | The time Gofer will wait for all connections to drain before exiting.                   |
| tls_cert_path         | string | <Required>     | The TLS certificate Gofer will use for the main service endpoint. This is required.     |
| tls_key_path          | string | <Required>     | The TLS certificate key Gofer will use for the main service endpoint. This is required. |
| storage_path          | string | /tmp/gofer.db  | Where to put Gofer's sqlite database.                                                   |
| storage_results_limit | int    | 200            | The amount of results Gofer's database is allowed to return on one query.               |

```toml
[server]
url = "http://localhost:8080"
bind_address = "0.0.0.0:8080"
extension_address = "172.17.0.1:8080"
shutdown_timeout = 15
storage_path = "/tmp/gofer.db"
storage_results_limit = 200
use_tls = false
```

### Extensions (block)

Controls settings for Gofer's extension system. Extensions are different workflows for running pipelines usually based on some other event (like the passing of time).

You can find [more information on the extension block here.](../extensions/index.html)

| name                    | type    | default    | description                                                                                                                                      |
| ----------------------- | ------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| install_base_extensions | boolean | true       | Attempts to automatically install the `cron` and `interval` extensions on first startup.                                                         |
| stop_timeout            | string  | 5m         | The amount of time Gofer will wait until extension containers have stopped before sending a SIGKILL.                                             |
| tls_cert_path           | string  | <Required> | The TLS certificate path Gofer will use for the extensions. This should be a certificate that the main Gofer service will be able to access.     |
| tls_key_path            | string  | <Required> | The TLS certificate path key Gofer will use for the extensions. This should be a certificate that the main Gofer service will be able to access. |

```toml
[extensions]
install_std_extensions = true
stop_timeout = 300            # 5 mins
use_tls = false
```
