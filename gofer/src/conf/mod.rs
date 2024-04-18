pub mod api;
pub mod cli;

use anyhow::Result;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;
#[allow(deprecated)]
use std::path::PathBuf;

pub trait ConfigType: Deserialize<'static> {
    fn default_config() -> &'static str;
    fn config_paths() -> Vec<PathBuf>;
    fn env_prefix() -> &'static str;
}

pub struct Configuration<T: ConfigType> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: ConfigType> Configuration<T> {
    pub fn load(path_override: Option<PathBuf>) -> Result<T> {
        let mut config = Figment::new().merge(Toml::string(T::default_config()));

        if let Some(path) = path_override {
            config = config.merge(Toml::file(path));
        } else {
            for path in T::config_paths() {
                config = config.merge(Toml::file(path));
            }
        }

        // The split function below is actually pretty load bearing.
        // We use a double underscore `__` to differentiate the difference between
        // a level of the struct and a key in that same struct when we read in environment variables.
        //
        // For example, if you have a doubly nested struct `app -> general` with a key that also has an
        // underline like `log_level`, when the resolution of configuration happens there is no
        // determinate way to resolve the difference between a key is named `general_log_level` and a key
        // that is simply just `level` with the potential to be nested as `app -> general -> log`.
        //
        // To solve this we use a double underscore which denotes the difference between what are actual
        // keys and what are levels of the struct we need to dive into.
        config = config.merge(Env::prefixed(T::env_prefix()).split("__"));
        let parsed_config: T = config.extract()?;

        Ok(parsed_config)
    }
}
