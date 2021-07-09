// Gofer Service configuration file is used as an alternative to proving the server configurations via envvars.
// You can find an explanation of these configuration variables and where to put this file so the server can read this
// file in the documenation: https://clintjedwards.com/gofer/docs/server-configuration/overview
accept_events_on_startup = true
event_loop_channel_size  = 100
host                     = "localhost:8080"
log_level                = "info"
run_log_expiry           = 20
task_run_logs_dir        = "/tmp"
task_run_stop_timeout    = "5m"
encryption_key           = "change_me"

external_events_api {
  enable = true
  host   = "localhost:8081"
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
  pipeline_object_limit = 10
  run_object_expiry     = 20
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
  shutdown_timeout = "15s"
  tls_cert_path    = "./localhost.crt"
  tls_key_path     = "./localhost.key"
  tmp_dir          = "/tmp"
}

triggers {
  stop_timeout         = "5m"
  healthcheck_interval = "30s"
  tls_cert_path        = "./localhost.crt"
  tls_key_path         = "./localhost.key"
  registered_triggers "cron" {
    image = "ghcr.io/clintjedwards/gofer/trigger_cron:latest"
  }
  registered_triggers "interval" {
    image = "ghcr.io/clintjedwards/gofer/trigger_interval:latest"
  }
}

