---
id: overview
title: Overview
sidebar_position: 1
---

# Server Configuration

Gofer runs as a single static binary that you deploy onto your favorite VPS.

While Gofer will happily run in development mode without any additional configuration, this mode is **NOT** recommended for production workloads and not intended to be secure.

Instead Gofer allows you to edit it's startup configuration allowing you to configure it to run on your favorite container orchestrator, object store, and/or storage backend.

## Setup

Gofer accepts configuration through environment variables or a configuration file. If a configuration key is set both in an environment variable and in a configuration file, the value of the environment variable's value will be the final value.

You can view a list of environment variables Gofer takes by using the `gofer service printenv` command. It's important to note that each environment variable starts with a prefix of `GOFER_`. So setting the `host` configuration can be set as:

```bash
export GOFER_HOST=localhost:8080
```

## Configuration file

The Gofer service configuration file is written in [HCL](https://octopus.com/blog/introduction-to-hcl-and-hcl-tooling).

### Load order

The Gofer service looks for its configuration in one of several places (ordered by first searched):

1. Path given through the `GOFER_CONFIG_PATH` environment variable
2. /etc/gofer/gofer.hcl

:::tip
You can generate a sample Gofer configuration file by using the command: `gofer service init-config`
:::

## Bare minimum production file

These are the bare minimum values you should populate for a production ready Gofer configuration.

The values below should be changed depending on your environment; leaving them as they currently are will lead to loss of data on server restarts.

```hcl
host                     = "0.0.0.0:8080"
log_level                = "info"
task_run_logs_dir        = "/tmp"
encryption_key           = "change_me"

external_events_api {
  enable = true
  host   = "0.0.0.0:8081"
}

database {
  engine            = "bolt"
  max_results_limit = 100
  boltdb {
    path = "/tmp/gofer.db"
  }
}

object_store {
  engine = "bolt"
  boltdb {
    path = "/tmp/gofer-os.db"
  }
}

scheduler {
  engine = "docker"
  docker {
    prune          = true
    prune_interval = "24h"
  }
}

server {
  dev_mode         = false
  tls_cert_path    = "./localhost.crt"
  tls_key_path     = "./localhost.key"
  tmp_dir          = "/tmp"
}

triggers {
  tls_cert_path        = "./localhost.crt"
  tls_key_path         = "./localhost.key"
}
```
