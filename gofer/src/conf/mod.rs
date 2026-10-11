pub mod api;
pub mod cli;

use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
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
        let parsed_config: T = Self::figment(path_override).extract()?;

        Ok(parsed_config)
    }

    /// The config file that settings are read from, if there is one. Only the first file that exists is used, so
    /// a stray second file can't quietly override the one the user is actually editing.
    pub fn config_file(path_override: Option<PathBuf>) -> Option<PathBuf> {
        match path_override {
            Some(path) => Some(path).filter(|path| path.exists()),
            None => T::config_paths().into_iter().find(|path| path.exists()),
        }
    }

    /// The merged configuration before it's parsed. Useful for finding out where a setting's value came from
    /// through [`Figment::find_metadata`].
    pub fn figment(path_override: Option<PathBuf>) -> Figment {
        let mut config = Figment::new().merge(Toml::string(T::default_config()));

        if let Some(path) = Self::config_file(path_override) {
            config = config.merge(Toml::file(path));
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
        config.merge(Env::prefixed(T::env_prefix()).split("__"))
    }
}
