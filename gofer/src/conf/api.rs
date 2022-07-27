use crate::conf::{LOCALHOST_CA, LOCALHOST_CRT, LOCALHOST_KEY};
use crate::{object_store, scheduler, secret_store};
use econf::LoadEnv;
use serde::Deserialize;

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct Config {
    pub general: General,
    pub server: Server,
    pub scheduler: Scheduler,
    pub triggers: Triggers,
    pub object_store: ObjectStore,
    pub secret_store: SecretStore,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct General {
    /// Turns on humanized debug messages, extra debug logging for the webserver and other
    /// convenient features for development. Usually turned on along side LogLevel=debug.
    pub dev_mode: bool,

    pub log_level: String,

    /// The encryption key is used to store sensitive Gofer values. It MUST be 32 characters long.
    pub encryption_key: String,

    /// How often the background process for pruning events should run (in seconds).
    pub event_prune_interval: u64,

    /// Controls how long Gofer will hold onto events before discarding them.
    /// This is important factor in disk space and memory footprint.
    ///
    /// Example: Rough math on a 5,000 pipeline Gofer instance with a full 6 months of retention
    ///  puts the memory and storage footprint at about 9GB.
    pub event_retention: u64,

    /// The limit automatically imposed if the pipeline does not define a limit. 0 is unlimited.
    pub run_parallelism_limit: u64,

    /// The total amount of runs before logs of the oldest run will be deleted.
    pub task_run_log_expiry: u64,

    /// Directory to store task run log files.
    pub task_run_logs_dir: String,

    /// Time in seconds the scheduler will wait for a normal user container(not-trigger containers)
    /// to stop. When the timeout is reached the container will be forcefully terminated.
    /// You can you use a timeout of 0 to convey that no timeout should be specified and the
    /// scheduler should wait however long it takes the container to respond to the terminal signal.
    /// This is usually passed to the scheduler when a request to cancel a task run is being made.
    pub task_run_stop_timeout: u64,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct Server {
    pub url: String,

    /// How many seconds to wait for the server to gracefully shutdown.
    pub shutdown_timeout: u64,
    pub storage_path: String,
    pub tls_cert: String,
    pub tls_key: String,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct Scheduler {
    pub engine: scheduler::Engine,
    pub docker: Option<DockerScheduler>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct DockerScheduler {
    pub prune: bool,
    pub prune_interval: u64, // in seconds
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct Triggers {
    pub tls_ca: Option<String>,
    pub tls_cert: String,
    pub tls_key: String,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct ObjectStore {
    pub engine: object_store::Engine,
    pub embedded: Option<EmbeddedObjectStore>,
    pub pipeline_object_limit: u64,
    pub run_object_expiry: u64,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct EmbeddedObjectStore {
    pub path: String,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct SecretStore {
    pub engine: secret_store::Engine,
    pub embedded: Option<EmbeddedSecretStore>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, LoadEnv)]
pub struct EmbeddedSecretStore {
    pub path: String,
    /// The encryption key is used to store sensitive Gofer values. It MUST be 32 characters long.
    pub encryption_key: String,
}

impl Config {
    pub fn inject_localhost_dev_certs(&mut self) {
        // If the user has entered their own custom TLS,
        // or is not in dev mode
        // or entered their own custom ca, then don't
        // populate the localhost trigger certs.
        if self.triggers.tls_cert.is_empty()
            && self.triggers.tls_key.is_empty()
            && self.general.dev_mode
            && self.triggers.tls_ca.is_none()
        {
            self.triggers.tls_ca = Some(LOCALHOST_CA.to_string());
            self.triggers.tls_cert = LOCALHOST_CRT.to_string();
            self.triggers.tls_key = LOCALHOST_KEY.to_string();
        }

        // if the user has is in dev mode, and has not entered
        // a custom TLS cert/key, fill in the localhost certs.
        if self.general.dev_mode
            && self.server.tls_cert.is_empty()
            && self.server.tls_key.is_empty()
        {
            self.server.tls_cert = LOCALHOST_CRT.to_string();
            self.server.tls_key = LOCALHOST_KEY.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::{Kind, LOCALHOST_CA, LOCALHOST_CRT, LOCALHOST_KEY};
    use crate::{object_store, scheduler, secret_store};
    use config;
    use pretty_assertions::assert_eq;
    use std::env::{remove_var, set_var};

    #[test]
    /// Test that the default api config is properly parsed from the configuration file.
    fn parse_default_config_from_file() {
        let config_src_builder = config::Config::builder();

        let config = Kind::new_api_config();

        let config_src = config_src_builder
            .add_source(config::File::from_str(
                config.default_config(),
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap();

        let parsed_config = config_src.try_deserialize::<Config>().unwrap();
        let expected_config = Config {
            general: General {
                dev_mode: true,
                log_level: "debug".to_string(),
                encryption_key: "changemechangemechangemechangeme".to_string(),
                event_prune_interval: 604800,
                event_retention: 7889238,
                run_parallelism_limit: 0,
                task_run_log_expiry: 20,
                task_run_logs_dir: "/tmp".to_string(),
                task_run_stop_timeout: 15,
            },
            server: Server {
                url: "127.0.0.1:8080".to_string(),
                storage_path: "/tmp/gofer.db".to_string(),
                shutdown_timeout: 15,
                ..Default::default()
            },
            scheduler: Scheduler {
                engine: scheduler::Engine::Docker,
                docker: Some(DockerScheduler {
                    prune: true,
                    prune_interval: 604800,
                }),
            },
            triggers: Triggers {
                ..Default::default()
            },
            object_store: ObjectStore {
                engine: object_store::Engine::Embedded,
                embedded: Some(EmbeddedObjectStore {
                    path: "/tmp/gofer-object-store".to_string(),
                }),
                pipeline_object_limit: 10,
                run_object_expiry: 20,
            },
            secret_store: SecretStore {
                engine: secret_store::Engine::Embedded,
                embedded: Some(EmbeddedSecretStore {
                    path: "/tmp/gofer-secret-store".to_string(),
                    encryption_key: "changemechangemechangemechangeme".to_string(),
                }),
            },
        };

        assert_eq!(parsed_config, expected_config);
    }

    #[test]
    /// Test that the TLS replacement for local builds works correctly.
    fn parse_default_config_with_default_tls() {
        let config_src_builder = config::Config::builder();

        let config = Kind::new_api_config();

        let config_src = config_src_builder
            .add_source(config::File::from_str(
                config.default_config(),
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap();

        let mut parsed_config = config_src.try_deserialize::<Config>().unwrap();
        parsed_config.inject_localhost_dev_certs();

        let expected_config = Config {
            general: General {
                dev_mode: true,
                log_level: "debug".to_string(),
                encryption_key: "changemechangemechangemechangeme".to_string(),
                event_prune_interval: 604800,
                event_retention: 7889238,
                run_parallelism_limit: 0,
                task_run_log_expiry: 20,
                task_run_logs_dir: "/tmp".to_string(),
                task_run_stop_timeout: 15,
            },
            server: Server {
                url: "127.0.0.1:8080".to_string(),
                shutdown_timeout: 15,
                storage_path: "/tmp/gofer.db".to_string(),
                tls_cert: LOCALHOST_CRT.to_string(),
                tls_key: LOCALHOST_KEY.to_string(),
            },
            scheduler: Scheduler {
                engine: scheduler::Engine::Docker,
                docker: Some(DockerScheduler {
                    prune: true,
                    prune_interval: 604800,
                }),
            },
            triggers: Triggers {
                tls_ca: Some(LOCALHOST_CA.to_string()),
                tls_cert: LOCALHOST_CRT.to_string(),
                tls_key: LOCALHOST_KEY.to_string(),
            },
            object_store: ObjectStore {
                engine: object_store::Engine::Embedded,
                embedded: Some(EmbeddedObjectStore {
                    path: "/tmp/gofer-object-store".to_string(),
                }),
                pipeline_object_limit: 10,
                run_object_expiry: 20,
            },
            secret_store: SecretStore {
                engine: secret_store::Engine::Embedded,
                embedded: Some(EmbeddedSecretStore {
                    path: "/tmp/gofer-secret-store".to_string(),
                    encryption_key: "changemechangemechangemechangeme".to_string(),
                }),
            },
        };

        assert_eq!(parsed_config, expected_config);
    }

    #[test]
    /// Test that env vars correctly overwrite struct vars and are parsed correctly.
    fn parse_env_vars() {
        let config = Config::default();
        let parsed_config = econf::load(config.clone(), "GOFER");

        // First check that empty env_vars don't incorrectly clear out
        // populated values.
        assert_eq!(config, parsed_config);

        // Then we check with various inputs.
        set_var("GOFER_GENERAL_DEV_MODE", "true");
        set_var("GOFER_GENERAL_LOG_LEVEL", "test_value");
        set_var("GOFER_SERVER_URL", "test_value");
        set_var("GOFER_SCHEDULER_ENGINE", "Docker");

        let expected_config = Config {
            general: General {
                dev_mode: true,
                log_level: "test_value".to_string(),
                ..Default::default()
            },
            server: Server {
                url: "test_value".to_string(),
                ..Default::default()
            },
            scheduler: Scheduler {
                engine: scheduler::Engine::Docker,
                ..Default::default()
            },
            triggers: Triggers {
                ..Default::default()
            },
            object_store: ObjectStore {
                engine: object_store::Engine::Embedded,
                ..Default::default()
            },
            secret_store: SecretStore {
                engine: secret_store::Engine::Embedded,
                ..Default::default()
            },
        };
        let parsed_config = econf::load(config, "GOFER");

        assert_eq!(expected_config, parsed_config);

        // Cleanup vars so we don't infect testers envs.
        remove_var("GOFER_GENERAL_DEV_MODE");
        remove_var("GOFER_GENERAL_LOG_LEVEL");
        remove_var("GOFER_SERVER_URL");
        remove_var("GOFER_SCHEDULER_ENGINE");
    }
}
