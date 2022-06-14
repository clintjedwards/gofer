pub mod api;
pub mod cli;
use config::{Config, FileFormat};
use rust_embed::RustEmbed;
#[allow(deprecated)]
use std::env::home_dir;
use std::{borrow::Cow, error::Error};

#[derive(RustEmbed)]
#[folder = "src/conf/"]
#[include = "*.toml"]
struct EmbeddedConfigFS;

/// The configuration type.
pub enum Kind {
    Api(api::Config),
    Cli(cli::Config),
}

impl Kind {
    /// returns an embedded default configuration file in bytes.
    fn default_config(&self) -> Cow<'static, [u8]> {
        match self {
            Kind::Api(_) => {
                let config_file = EmbeddedConfigFS::get("default_api_config.toml").unwrap();
                config_file.data
            }
            Kind::Cli(_) => {
                let config_file = EmbeddedConfigFS::get("default_cli_config.toml").unwrap();
                config_file.data
            }
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
        Self::Api(api::Config::default())
    }

    /// Instantiates an empty cli config. Use `parse` to populate.
    ///
    /// `new_cli_config::parse("/home/myfile.toml")`
    pub fn new_cli_config() -> Self {
        Self::Cli(cli::Config::default())
    }

    /// returns a correctly deserialized config struct from the configuration files and environment passed to it.
    ///
    /// The order of the configuration files read in is by order passed in. So [config_1.yml, config_2.yml] would cause
    /// any conflicting keys in both configs to inherit config_2's value.
    pub fn parse(&self, path_override: &Option<String>) -> Result<Kind, Box<dyn Error>> {
        let mut config_src_builder = Config::builder();

        // First parse embedded config defaults.
        let default_config_raw = self.default_config();
        let default_config = std::str::from_utf8(&default_config_raw)?;

        config_src_builder =
            config_src_builder.add_source(config::File::from_str(default_config, FileFormat::Toml));

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
                // Lastly env vars always override everything.
                let config_src = config_src_builder
                    .add_source(
                        config::Environment::with_prefix("GOFER")
                            .prefix_separator("_")
                            .separator("__")
                            .ignore_empty(true),
                    )
                    .build()?;

                let parsed_config = config_src.try_deserialize::<api::Config>()?;
                Ok(Kind::Api(parsed_config))
            }
            Kind::Cli(_) => {
                // Lastly env vars always override everything.
                let config_src = config_src_builder
                    .add_source(
                        config::Environment::with_prefix("GOFER_CLI")
                            .separator("_")
                            .ignore_empty(true),
                    )
                    .build()?;

                let parsed_config = config_src.try_deserialize::<cli::Config>()?;
                Ok(Kind::Cli(parsed_config))
            }
        }
    }
}
