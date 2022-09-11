// Gofer Service configuration file is used as an alternative to providing the server configurations via envvars.
// You can find an explanation of these configuration variables and where to put this file so the server can read this
// file in the documenation: https://clintjedwards.com/gofer/docs/server-configuration/overview
event_log_retention        = "4380h"
host                       = "localhost:8080"
ignore_pipeline_run_events = false
log_level                  = "info"
prune_events_interval      = "3h"
run_log_expiry             = 30
task_run_logs_dir          = "/tmp"
task_run_stop_timeout      = "5m"

external_events_api {
  enable = true
  host   = "localhost:8081"
}

database {
  max_results_limit = 200
  path = "/tmp/gofer.db"
}

object_store {
  engine = "sqlite"
  sqlite {
    path = "/tmp/gofer-object.db"
  }
  pipeline_object_limit = 30
  run_object_expiry     = 30
}

secret_store {
  engine = "sqlite"
  sqlite {
    path           = "/tmp/gofer-secret.db"
    encryption_key = "changemechangemechangemechangeme"
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
  shutdown_timeout = "15s"
  tls_cert_path    = "./localhost.crt"
  tls_key_path     = "./localhost.key"
  tmp_dir          = "/tmp"
}

triggers {
  install_base_triggers = true
  stop_timeout          = "5m"
  healthcheck_interval  = "30s"
  tls_cert_path         = "./localhost.crt"
  tls_key_path          = "./localhost.key"
}

