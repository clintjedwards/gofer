use crate::conf::ConfigType;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

const DEFAULT_CLI_CONFIG: &str = include_str!("./default_cli_config.toml");

#[derive(Debug, Clone, Display, Default, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputFormat {
    #[serde(alias = "plain", alias = "PLAIN")]
    #[default]
    Plain,

    #[serde(alias = "silent", alias = "SILENT")]
    Silent,

    #[serde(alias = "json", alias = "JSON")]
    Json,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct CliConfig {
    /// The URL of the Gofer API.
    pub api_base_url: String,

    /// Provides extra debug output.
    pub debug: bool,

    /// Turn on extra detail for certain commands. Controls things like what format time is in.
    pub detail: bool,

    /// Don't verify server certificate; useful for development.
    pub insecure_skip_tls_verify: Option<bool>,

    /// The default namespace to work within. You can change this on the fly via the CLI.
    pub namespace: String,

    /// What format the CLI will write to the terminal in.
    pub output_format: OutputFormat,

    /// The Gofer API token.
    pub token: String,
}

impl ConfigType for CliConfig {
    fn default_config() -> &'static str {
        DEFAULT_CLI_CONFIG
    }

    // We look for configuration to help developers not mix up their real config from their development config.

    #[cfg(debug_assertions)]
    fn config_paths() -> Vec<std::path::PathBuf> {
        let user_home = dirs::home_dir().expect("Unable to get home directory");

        vec![
            user_home.join(".gofer_dev.toml"),
            user_home.join(".config/gofer_dev.toml"),
        ]
    }

    #[cfg(not(debug_assertions))]
    fn config_paths() -> Vec<std::path::PathBuf> {
        let user_home = dirs::home_dir().expect("Unable to get home directory");

        vec![
            user_home.join(".gofer.toml"),
            user_home.join(".config/gofer.toml"),
        ]
    }

    fn env_prefix() -> &'static str {
        "GOFER_"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::Configuration;
    use pretty_assertions::assert_eq;
    use std::env;

    #[test]
    fn load_from_environment_variables() {
        env::set_var("GOFER_API_BASE_URL", "http://localhost:3001");
        env::set_var("GOFER_ADMIN_KEY", "envoverride");

        let config = Configuration::<CliConfig>::load(None).unwrap();

        // Cleanup environment variables after test
        env::remove_var("GOFER_API_BASE_URL");
        env::remove_var("GOFER_ADMIN_KEY");

        assert_eq!(
            config,
            CliConfig {
                namespace: "default".to_string(),
                detail: false,
                token: "example".to_string(),
                api_base_url: "http://localhost:3001".to_string(),
                output_format: OutputFormat::Plain,
                debug: false,
                insecure_skip_tls_verify: None,
            }
        );
    }
}
