// Gofer Service configuration file is used as an alternative to providing the server configurations via envvars.
// You can find an explanation of these configuration variables and where to put this file so the server can read this
// file in the documenation: https://clintjedwards.com/gofer/ref/server_configuration/index.html
ignore_pipeline_run_events = false
run_parallelism_limit      = 200
pipeline_version_limit     = 5
event_log_retention        = "4380h"
event_prune_interval       = "3h"
log_level                  = "info"
task_run_log_expiry        = 50
task_run_logs_dir          = "/tmp"
task_run_stop_timeout      = "5m"

external_events_api {
  enable = true
  host   = "localhost:8081"
}

object_store {
  engine = "sqlite"
  sqlite {
    path = "/tmp/gofer-object.db"
  }
  pipeline_object_limit = 50
  run_object_expiry     = 50
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
  host                  = "localhost:8080"
  shutdown_timeout      = "15s"
  tls_cert_path         = "./localhost.crt"
  tls_key_path          = "./localhost.key"
  storage_path          = "/tmp/gofer.db"
  storage_results_limit = 200
}

extensions {
  install_base_extensions = true
  stop_timeout          = "5m"
  tls_cert_path         = "./localhost.crt"
  tls_key_path          = "./localhost.key"
}

