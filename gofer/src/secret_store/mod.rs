pub mod sqlite;

use serde::Deserialize;
use std::fmt::Debug;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value(pub Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Secret {
    pub key: String,
    pub value: Vec<u8>,
}

/// Represents different secret store failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum SecretStoreError {
    #[error("could not establish connection to secret store; {0}")]
    Connection(String),

    #[error("requested entity not found")]
    NotFound,

    #[error("entity already exists")]
    Exists,

    #[error("unexpected storage error occurred")]
    Internal(String),

    /// Failed to start due to misconfigured settings, usually from a misconfigured settings file.
    #[error("could not init secret store; {0}")]
    FailedPrecondition(String),

    #[error(
        "unexpected storage error occurred; code: {code:?}; message: {message}; query: {query}"
    )]
    GenericDBError {
        code: Option<String>,
        message: String,
        query: String,
    },
}

pub trait SecretStore: Debug + Send + Sync + 'static {
    fn get(&self, key: &str) -> Result<Value, SecretStoreError>;
    fn put(&self, key: &str, content: Vec<u8>, force: bool) -> Result<(), SecretStoreError>;
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>, SecretStoreError>;
    fn delete(&self, key: &str) -> Result<(), SecretStoreError>;
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")] // This handles case insensitivity during deserialization
pub enum Engine {
    #[default]
    Sqlite,
}

pub fn new(
    config: &crate::conf::api::SecretStore,
) -> Result<Box<dyn SecretStore>, SecretStoreError> {
    #[allow(clippy::match_single_binding)]
    match config.engine {
        Engine::Sqlite => {
            if config.sqlite.is_none() {
                return Err(SecretStoreError::FailedPrecondition(
                    "Sqlite engine settings not found in config".into(),
                ));
            }

            let engine = sqlite::Engine::new(&config.clone().sqlite.unwrap())
                .map_err(|err| SecretStoreError::FailedPrecondition(err.to_string()))?;
            Ok(Box::new(engine))
        }
    }
}
