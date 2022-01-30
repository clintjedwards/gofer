---
id: configuration-values
title: Configuration Values
sidebar_position: 2
---

# Configuration Values

Gofer has a variety of parameters that can be specified via environment variables or the configuration file.

To view a listing of the possible environment variables use: `gofer service printenv`.

The most up to date config file values can be found by [reading the code](https://github.com/clintjedwards/gofer/blob/main/internal/config/api.go#L14), but a best effort key and description list is given below.

## Values

- #### `ignore_pipeline_run_events` (bool: _false_)

  Controls the ability for the Gofer service to execute jobs on startup. If this is set to false you can set it to true manually using the CLI command `gofer service toggle-event-ingress`.

- #### `event_log_retention` (string: _4380h_)

  The limit (in hours) for how long Gofer will store events. Larger values will lead to larger database footprints. The defalt retention period is 6 months.

- #### `prune_events_interval` (string: _3h_)

  The limit (in hours) for how long Gofer will store events. Larger values will lead to larger database footprints. The defalt retention period is 6 months.

- #### `host` (string: _localhost:8080_)

  The address and port for the service to bind to.

- #### `log_level` (string: _debug_)

  The logging level that is output. It is common to start with info.

- #### `run_log_expiry` (int: _20_)

  Each Gofer task run generates logs from the container being run. This controls the number of runs before Gofer cleans up the logs associated with that run.

- #### `task_run_logs_dir` (string: _/tmp_)

  The path of the directory to store task run logs. Task run logs are stored as a text file on the server.

- #### `task_run_stop_timeout` (string: _5m_)

  The amount of time Gofer will wait for a container to gracefully stop before sending it a SIGKILL.

- #### `external_events_api` (block)

  The external events API controls webhook type interactions with triggers. HTTP requests go through the events endpoint and Gofer routes them to the proper trigger for handling.

  - #### `enable` (bool: _true_)
    Enable the events api
  - #### `host` (string: _localhost:8081_)
    The address and port to bind the events service to.

  ```hcl
  external_events_api {
    enable = true
    host   = "0.0.0.0:8081"
  }
  ```

- #### `database` (block)

  The settings for the backend database Gofer will use to store state. Gofer's only database option is boltdb.

  - #### `engine` (string: _bolt_)
    The engine Gofer will use to store state. The accepted values here are "bolt".
  - #### `max_results_limit` (int: _100_)
    The maximum amount of results the database will return when not specified. This is useful for pagination defaults.
  - #### `boltdb` (block)
    [Bolt DB](https://dbdb.io/db/boltdb) is a key-value store. Its fast, lightweight, and can be run easily locally. It is the defacto development storage because of these properties.
    - #### `path` (string: _/tmp/gofer.db_)
      The path of the file that boltdb will use. If this file does not exist Gofer will create it.

  ```hcl
  database {
    engine            = "bolt"
    max_results_limit = 100
    boltdb {
      path = "/tmp/gofer.db"
    }
  }
  ```

- #### `object_store` (block)

  The settings for the Gofer object store. The object store assists Gofer with storing values between tasks since Gofer is by nature distributed. This helps jobs avoid having to download the same objects over and over or simply just allows tasks to share certain values.

  You can find [more information on the object store block here.](../object-stores/overview)

  - #### `engine` (string: _bolt_)
    The engine Gofer will use to store state. The accepted values here are "bolt".
  - #### `pipeline_object_limit` (int: _10_)
    The limit to the amount of objects that can be stored at the pipeline level. Objects stored at the pipeline level are kept permanently, but once the object limit is reach the oldest object will be deleted.
  - #### `run_object_expiry` (int: _20_)
    The number of runs before objects stored at the "run level" will be removed.
  - #### `boltdb` (block)
    [Bolt DB](https://dbdb.io/db/boltdb) is a key-value store. Its fast, lightweight, and can be run easily locally. It is the defacto development storage because of these properties.
    - #### `path` (string: _/tmp/gofer-os.db_)
      The path of the file that boltdb will use. If this file does not exist Gofer will create it.

  ```hcl
  object_store {
    engine = "bolt"
    boltdb {
      path = "/tmp/gofer-os.db"
    }
  }
  ```

- #### `secret_store` (block)

  The settings for the Gofer secret store. The secret store allows users to securely populate their pipeline configuration with secrets that are used by their tasks, trigger configuration, or scheduler.

  You can find [more information on the secret store block here.](../secret-stores/overview)

  - #### `engine` (string: _bolt_)
    The engine Gofer will use to store state. The accepted values here are "bolt".
  - #### `boltdb` (block)
    [Bolt DB](https://dbdb.io/db/boltdb) is a key-value store. Its fast, lightweight, and can be run easily locally. It is the defacto development storage because of these properties.
    - #### `path` (string: _/tmp/gofer-os.db_)
      The path of the file that boltdb will use. If this file does not exist Gofer will create it.
    - #### `encryption_key` (string: _default_)
      The key used to encrypt secrets into the secretStore. This must be a 32 character randomized value.

  ```hcl
  secret_store {
    engine = "bolt"
    boltdb {
      path = "/tmp/gofer-os.db"
      encryption_key = "changemechangemechangemechangeme"
    }
  }
  ```

- #### `scheduler` (block)

  The settings for the container orchestrator that Gofer will use to schedule workloads.

  You can find [more information on the scheduler block here.](../schedulers/overview)

  - #### `engine` (string: _docker_)
    The engine Gofer will use as a container orchestrator. The accepted values here are "docker".
  - #### `docker` (block)
    [Docker](https://www.docker.com/why-docker) is the default container orchestrator and leverages the machine's local docker engine to schedule containers.
    - #### `prune` (bool: _false_)
      Controls if the docker scheduler should periodically clean up old containers.
    - #### `prune_interval` (string: _24h_)
      Controls how often the prune container job should run.

  ```hcl
  scheduler {
    engine = "docker"
    docker {
      prune          = true
      prune_interval = "24h"
    }
  }
  ```

- #### `server` (block)

  Controls the settings for the Gofer service's server properties.

  - #### `dev_mode` (bool: _true_)
    Dev mode controls many aspects of Gofer to make it easier to run locally for development and testing. Because of this you should not run dev mode in production as it is not safe. A non-complete list of things dev-mode helps with: the use of localhost certificates, autogeneration of encryption key, bypass of authentication for all routes.
  - #### `shutdown_timeout` (string: _15s_)
    The time Gofer will wait for all connections to drain before exiting.
  - #### `tls_cert_path` (string: _required_)
    The TLS certificate Gofer will use for the main service endpoint. This is required.
  - #### `tls_key_path` (string: _required_)
    The TLS certificate key Gofer will use for the main service endpoint. This is required.
  - #### `tmp_dir` (string: _/tmp_)
    Gofer temporarily downloads pipeline configuration files so they can be parsed. This setting is the temp directory that those files are downloaded to. These files are also cleaned up afterwards.

  ```hcl
  server {
    dev_mode         = false
    tls_cert_path    = "./localhost.crt"
    tls_key_path     = "./localhost.key"
    tmp_dir          = "/tmp"
  }
  ```

- #### `triggers` (block)

  Controls settings for Gofer's trigger system. Triggers are different workflows for running pipelines usually based on some other event (like the passing of time).

  You can find [more information on the trigger block here.](../triggers/overview)

  - #### `stop_timeout` (string: _5m_)
    The amount of time Gofer will wait until trigger containers have stopped before sending a SIGKILL.
  - #### `healthcheck_interval` (string: _30s_)
    The amount of time between the check for if Gofer triggers are still alive. This check ensure that triggers aren't silently dead and workflows aren't silently being missed.
  - #### `tls_cert_path` (string: _required_)
    The TLS certificate path Gofer will use for the triggers. This should be a certificate that the main Gofer service will be able to access.
  - #### `tls_key_path` (string: _required_)
    The TLS certificate path key Gofer will use for the triggers. This should be a certificate that the main Gofer service will be able to access.
  - #### `registered_triggers` (block)
    Controls the list of triggers Gofer will start-up with. You can list this block many times and point to various images that use the [Gofer SDK](../triggers/overview) to create a trigger.
    - #### `kind` (label)
      The unique name of the trigger.
    - #### `image` (string: _""_)
      The registry and docker image name of the trigger.
    - #### `user` (string: _""_)
      User value for registries that need authentication.
    - #### `pass` (string: _""_)
      Pass value for registries that need authentication.
    - #### `env_vars` (map[string]string: _"":""_)
      A mapping of environment variables that will be passed to the container. This is useful for passing trigger specific values that edit the configuration of the triggers.
    - #### `secrets` (map[string]string: _"":""_)
      A mapping of secrets that will be passed to the container. This is useful for passing trigger specific secrets that modify the configuration of the triggers.

  ```hcl
  triggers {
    stop_timeout         = "5m"
    healthcheck_interval = "30s"
    tls_cert_path        = "./localhost.crt"
    tls_key_path         = "./localhost.key"
    registered_triggers "cron" {
        image = "ghcr.io/clintjedwards/gofer-containers/trigger_cron:latest"
    }
    registered_triggers "interval" {
        image = "ghcr.io/clintjedwards/gofer-container/trigger_interval:latest"
    }
  }
  ```
