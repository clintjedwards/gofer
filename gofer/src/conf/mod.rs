pub mod api;
pub mod cli;
use config::{Config, FileFormat};
#[allow(deprecated)]
use std::env::home_dir;
use std::error::Error;

const DEFAULT_API_CONFIG: &str = include_str!("./default_api_config.toml");
const DEFAULT_CLI_CONFIG: &str = include_str!("./default_cli_config.toml");

const LOCALHOST_CA: &str = include_str!("./localhost.ca");
const LOCALHOST_CRT: &str = include_str!("./localhost.crt");
const LOCALHOST_KEY: &str = include_str!("./localhost.key");

/// The configuration type. We box the API enum since it takes up
/// significantly more space than the CLI enum.
pub enum Kind {
    Api(Box<api::Config>),
    Cli(cli::Config),
}

impl Kind {
    /// returns an embedded default configuration file in bytes.
    fn default_config(&self) -> &str {
        match self {
            Kind::Api(_) => DEFAULT_API_CONFIG,
            Kind::Cli(_) => DEFAULT_CLI_CONFIG,
        }
    }

    /// returns the default configuration paths that are searched in case user does not specify.
    fn config_paths(&self) -> Vec<String> {
        match self {
            Self::Api(_) => {
                vec!["/etc/gofer/gofer.toml".to_string()]
            }
            Self::Cli(_) => {
                #[allow(deprecated)]
                let user_home = home_dir().unwrap();
                let first_location = user_home.to_string_lossy() + "/.gofer.toml";
                let second_location = user_home.to_string_lossy() + "/.config/gofer.toml";

                vec![first_location.to_string(), second_location.to_string()]
            }
        }
    }

    /// Instantiates an empty api config. Use `parse` to populate.
    ///
    /// `new_api_config::parse("/home/myfile.toml")`
    pub fn new_api_config() -> Self {
        Self::Api(Box::new(api::Config::default()))
    }

    /// Instantiates an empty cli config. Use `parse` to populate.
    ///
    /// `new_cli_config::parse("/home/myfile.toml")`
    pub fn new_cli_config() -> Self {
        Self::Cli(cli::Config::default())
    }

    /// Returns a correctly deserialized config struct from the configuration files and environment passed to it.
    ///
    /// The order of the configuration files read in is by order passed in. So [config_1.yml, config_2.yml] would cause
    /// any conflicting keys in both configs to inherit config_2's value.
    pub fn parse(&self, path_override: &Option<String>) -> Result<Kind, Box<dyn Error>> {
        let mut config_src_builder = Config::builder();

        config_src_builder = config_src_builder.add_source(config::File::from_str(
            self.default_config(),
            FileFormat::Toml,
        ));

        // Then parse user given paths.
        if path_override.is_none() {
            for path in self.config_paths() {
                config_src_builder =
                    config_src_builder.add_source(config::File::with_name(&path).required(false));
            }
        } else {
            config_src_builder = config_src_builder.add_source(
                config::File::with_name(path_override.as_ref().unwrap()).required(false),
            );
        }

        // Then attempt to deserialize based on which config needed.
        match self {
            Kind::Api(_) => {
                let config_src = config_src_builder.build()?;
                let parsed_config = config_src.try_deserialize::<api::Config>()?;

                // Lastly env vars always override everything.
                let mut parsed_config = econf::load(parsed_config, "GOFER");
                parsed_config.inject_localhost_dev_certs();

                Ok(Kind::Api(Box::new(parsed_config)))
            }
            Kind::Cli(_) => {
                let config_src = config_src_builder.build()?;
                let parsed_config = config_src.try_deserialize::<cli::Config>()?;

                // Lastly env vars always override everything.
                let mut parsed_config = econf::load(parsed_config, "GOFER_CLI");
                parsed_config.inject_localhost_dev_certs();
                Ok(Kind::Cli(parsed_config))
            }
        }
    }
}
