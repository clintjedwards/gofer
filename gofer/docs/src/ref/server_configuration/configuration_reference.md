# Configuration Reference

Gofer reads its configuration from `/etc/gofer/gofer_web.toml`, or the file passed to
`gofer service start --config <path>`. Any key can also be set through an environment variable that starts with
`GOFER_WEB_`, using a double underscore between the block and the key. For example `api.log_level` becomes
`GOFER_WEB_API__LOG_LEVEL`. Environment variables win over the file.

Every key has a default, so you only need to set the ones you want to change. The defaults are set up for running
Gofer locally; see the [bare minimum production file](./index.html#bare-minimum-production-file) for what to change
before running it for real.

The defaults come from
[default_api_config.toml](https://github.com/clintjedwards/gofer/blob/main/gofer/src/conf/default_api_config.toml) and
the keys themselves are defined in
[conf/api.rs](https://github.com/clintjedwards/gofer/blob/main/gofer/src/conf/api.rs). If this page and those files
ever disagree, the files are right.

Durations are all whole numbers of seconds.

## API

| name                           | type   | default  | description                                                                                                                                                                                                           |
| ------------------------------ | ------ | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| allow_task_attach              | bool   | true     | Lets users open a shell inside a running task's container with `gofer task attach`. Anyone with write access to a pipeline's task executions can run any command in its containers, so you may want to turn this off. |
| event_log_retention            | int    | 15768000 | How long, in seconds, Gofer holds onto events before discarding them (6 months by default). This is an important factor in disk space; rough math on a 5,000 pipeline Gofer instance with 6 months of retention puts it at about 9GB. |
| event_prune_interval           | int    | 604800   | How often, in seconds, Gofer checks for and removes events older than `event_log_retention`.                                                                                                                          |
| global_run_concurrency_limit   | int    | 2000     | How many runs can be in progress across all of Gofer at once.                                                                                                                                                        |
| log_level                      | string | info     | The log level for the service and its extensions.                                                                                                                                                                     |
| pipeline_run_concurrency_limit | int    | 200      | How many runs a single pipeline can have in progress at once. Pipelines can set a lower limit with `Parallelism`, but not a higher one. 0 is unlimited. |
| task_execution_log_retention   | int    | 50       | How many of each pipeline's runs keep their task logs. Once a pipeline has more runs than this, the logs of its oldest run are deleted. |
| task_execution_logs_dir        | string | /tmp     | The directory task execution logs are stored in. Each one is a text file on the server.                                                                                                                              |
| task_execution_stop_timeout    | int    | 300      | How long, in seconds, Gofer waits for a task's container to stop gracefully before killing it. 0 kills containers immediately.                                                                                        |

```toml
[api]
pipeline_run_concurrency_limit = 200
global_run_concurrency_limit = 2000
event_log_retention = 15768000     # 6 months
event_prune_interval = 604800      # 1 week
log_level = "info"
task_execution_log_retention = 50  # total runs
task_execution_logs_dir = "/tmp"
task_execution_stop_timeout = 300  # 5 mins
allow_task_attach = true
```

## Server

| name              | type   | default          | description                                                                                                                                                                                                    |
| ----------------- | ------ | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| bind_address      | string | 0.0.0.0:8080     | The address and port the API and web UI listen on.                                                                                                                                                             |
| extension_address | string | 172.17.0.1:8080  | The address extension containers use to reach Gofer. Extensions usually sit on a different network than your users, like the docker bridge network, so they need their own address. The default is the docker bridge's host address. |
| storage_path      | string | /tmp/gofer.db    | Where Gofer keeps its sqlite database.                                                                                                                                                                         |
| use_tls           | bool   | false            | Serve the API over TLS. Requires `tls_cert_path` and `tls_key_path`, unless `development.use_included_certs` is on.                                                                                           |
| tls_cert_path     | string |                  | Path to the TLS certificate.                                                                                                                                                                                   |
| tls_key_path      | string |                  | Path to the TLS certificate's key.                                                                                                                                                                             |

```toml
[server]
bind_address = "0.0.0.0:8080"
extension_address = "172.17.0.1:8080"
storage_path = "/tmp/gofer.db"
use_tls = false
```

## Development

Feature flags that make running Gofer locally easier. **The defaults are all on**, which is what lets
`gofer service start` work with no configuration. Turn them all off in production.

| name               | type | default | description                                                                                                                       |
| ------------------ | ---- | ------- | --------------------------------------------------------------------------------------------------------------------------------- |
| bypass_auth        | bool | true    | Skip authentication for all routes. Anyone who can reach Gofer can do anything.                                                    |
| pretty_logging     | bool | true    | Human readable logs instead of JSON.                                                                                              |
| use_included_certs | bool | true    | When TLS is turned on, use the localhost certificates built into Gofer instead of the paths you give it. They're public, so they don't protect anything. |

```toml
[development]
pretty_logging = true
bypass_auth = true
use_included_certs = true
```

## External Events

The external events service is a separate HTTP server that takes webhooks and passes them on to extensions. It runs
on its own port so you can expose it to services like Github without exposing the rest of Gofer. See
[External Events](./external_events.md) for how it works.

| name          | type   | default      | description                                                                                                  |
| ------------- | ------ | ------------ | ------------------------------------------------------------------------------------------------------------ |
| enable        | bool   | true         | Start the external events service.                                                                           |
| bind_address  | string | 0.0.0.0:8081 | The address and port the external events service listens on.                                                 |
| use_tls       | bool   | false        | Serve external events over TLS. Requires `tls_cert_path` and `tls_key_path`, unless `development.use_included_certs` is on. |
| tls_cert_path | string |              | Path to the TLS certificate.                                                                                 |
| tls_key_path  | string |              | Path to the TLS certificate's key.                                                                           |

```toml
[external_events]
enable = true
bind_address = "0.0.0.0:8081"
use_tls = false
```

## Object Store

The object store holds values that tasks share with each other or keep between runs.
[More about the object store.](../object_store/index.html)

| name                  | type   | default    | description                                                                                                                                                                                                                         |
| --------------------- | ------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| engine                | string | filesystem | The engine Gofer uses to store objects. The only accepted value is `filesystem`.                                                                                                                                                     |
| pipeline_object_limit | int    | 50         | How many objects each pipeline can store. Pipeline objects are kept until the limit is reached, then the oldest one is deleted to make room. Overwriting an object counts it as the newest. 0 is unlimited.                          |
| run_object_expiry     | int    | 2          | How many runs keep their run objects. An object stored on run #5 with an expiry of 2 is deleted when run #7 starts, regardless of how the runs went. Tokens from `InjectAPIToken` are deleted at the same time.                     |

### Filesystem

| name | type   | default            | description                                    |
| ---- | ------ | ------------------ | ---------------------------------------------- |
| path | string | /tmp/gofer_objects | The directory that object files are stored in. |

```toml
[object_store]
engine = "filesystem"
pipeline_object_limit = 50
run_object_expiry = 2

[object_store.filesystem]
path = "/tmp/gofer_objects"
```

## Secret Store

The secret store holds secrets that pipelines and extensions pull in at runtime.
[More about the secret store.](../secret_store/index.html)

| name   | type   | default | description                                                         |
| ------ | ------ | ------- | ------------------------------------------------------------------- |
| engine | string | sqlite  | The engine Gofer uses to store secrets. The only accepted value is `sqlite`. |

### Sqlite

| name           | type   | default                          | description                                                                                                                                                                       |
| -------------- | ------ | -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| path           | string | /tmp/gofer_secrets.db            | The path of the sqlite file. Gofer creates it if it doesn't exist.                                                                                                               |
| encryption_key | string | changemechangemechangemechangeme | The key secrets are encrypted with. It must be exactly 32 bytes (32 ASCII characters; `openssl rand -hex 16` makes one). Don't change it once it's set; every existing secret becomes unreadable. |

```toml
[secret_store]
engine = "sqlite"

[secret_store.sqlite]
path = "/tmp/gofer_secrets.db"
encryption_key = "changemechangemechangemechangeme"
```

## Scheduler

The scheduler is the container orchestrator Gofer runs tasks and extensions on.
[More about schedulers.](../scheduler/index.html)

| name   | type   | default | description                                                              |
| ------ | ------ | ------- | ------------------------------------------------------------------------ |
| engine | string | docker  | The scheduler Gofer uses. The only accepted value is `docker`.           |

### Docker

| name           | type | default | description                                                                                                          |
| -------------- | ---- | ------- | -------------------------------------------------------------------------------------------------------------------- |
| prune          | bool | true    | Periodically remove stopped containers. Without this the host's disk eventually fills with old containers.            |
| prune_interval | int  | 604800  | How often, in seconds, the prune job runs.                                                                           |
| timeout        | int  | 300     | How long, in seconds, a request to docker can take. Should be at least as long as `api.task_execution_stop_timeout`. |

```toml
[scheduler]
engine = "docker"

[scheduler.docker]
prune = true
prune_interval = 604800
timeout = 300
```

## Extensions

Controls Gofer's extension system. [More about extensions.](../extensions/index.html)

| name          | type   | default | description                                                                                                                                                                                                                                       |
| ------------- | ------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| install       | list   | empty   | The extensions Gofer should run, one `[[extensions.install]]` entry each. The `cron` and `interval` extensions are always included unless an entry with their id sets `enabled = false`. See [installing and configuring extensions](../extensions/index.html#installing-and-configuring-extensions) for every field. |
| stop_timeout  | int    | 300     | How long, in seconds, Gofer waits for an extension container to stop before killing it.                                                                                                                                                          |
| use_tls       | bool   | false   | Have extensions serve over TLS. Gofer hands each extension the certificate below, unless `development.use_included_certs` is on.                                                                                                                 |
| tls_cert_path | string |         | Path to the TLS certificate Gofer gives extensions.                                                                                                                                                                                              |
| tls_key_path  | string |         | Path to the TLS certificate's key.                                                                                                                                                                                                               |
| verify_certs  | bool   | false   | Check the extension's certificate when Gofer talks to it. Only matters when `use_tls` is on.                                                                                                                                                     |

`gofer extension reload` applies changes to the `install` entries without restarting Gofer. Everything else in this
block needs a restart.

```toml
[extensions]
stop_timeout = 300            # 5 mins
use_tls = false
verify_certs = false

[[extensions.install]]
id = "github"
manifest = "https://raw.githubusercontent.com/clintjedwards/gofer/v<gofer version>/containers/extensions/github/manifest.toml"
[extensions.install.settings]
app_id = "112348"
app_installation = "99560091"
app_key = "global_secret{{github-app-key}}"
app_webhook_secret = "global_secret{{github-app-webhook-secret}}"

[[extensions.install]]
id = "cron"
enabled = false
```
