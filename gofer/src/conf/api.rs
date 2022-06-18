use crate::scheduler::Engine;
use serde::Deserialize;

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, econf::LoadEnv)]
pub struct Config {
    pub general: General,
    pub server: Server,
    pub scheduler: Scheduler,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, econf::LoadEnv)]
pub struct General {
    /// Turns on humanized debug messages, extra debug logging for the webserver and other
    /// convenient features for development. Usually turned on along side LogLevel=debug.
    pub dev_mode: bool,
    pub log_level: String,
    pub encryption_key: String,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, econf::LoadEnv)]
pub struct Server {
    pub url: String,
    pub storage_path: String,
    pub tls_cert: String,
    pub tls_key: String,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, econf::LoadEnv)]
pub struct Scheduler {
    pub engine: Engine,
    pub docker: Option<DockerScheduler>,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, econf::LoadEnv)]
pub struct DockerScheduler {
    pub prune: bool,
    pub prune_interval: u64, // in seconds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::Kind;
    use crate::scheduler::Engine;
    use config;
    use std::env::{remove_var, set_var};

    #[test]
    /// Test that the default api config is properly parsed from the configuration file.
    fn parse_default_config_from_file() {
        let config_src_builder = config::Config::builder();

        let config = Kind::new_api_config();

        // First parse embedded config defaults.
        let default_config_raw = config.default_config();
        let default_config = std::str::from_utf8(&default_config_raw).unwrap();

        let config_src = config_src_builder
            .add_source(config::File::from_str(
                default_config,
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap();

        let parsed_config = config_src.try_deserialize::<Config>().unwrap();
        let expected_config = Config {
            general: General {
                dev_mode: true,
                log_level: "debug".to_string(),
                encryption_key: "default".to_string(),
            },
            server: Server {
                url: "127.0.0.1:8080".to_string(),
                storage_path: "/tmp/gofer.db".to_string(),
                ..Default::default()
            },
            scheduler: Scheduler {
                engine: Engine::Docker,
                docker: Some(DockerScheduler {
                    prune: true,
                    prune_interval: 604800,
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
        set_var("GOFER_SCHEDULER_DOCKER_PRUNE", "false");

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
                engine: Engine::Docker,
                docker: Some(DockerScheduler {
                    prune: false,
                    ..Default::default()
                }),
            },
        };
        let parsed_config = econf::load(config, "GOFER");

        assert_eq!(expected_config, parsed_config);

        // Cleanup vars so we don't infect testers envs.
        remove_var("GOFER_GENERAL_DEV_MODE");
        remove_var("GOFER_GENERAL_LOG_LEVEL");
        remove_var("GOFER_SERVER_URL");
        remove_var("GOFER_SCHEDULER_ENGINE");
        remove_var("GOFER_SCHEDULER_DOCKER_PRUNE");
    }
}
