pub mod sqlite;

use async_trait::async_trait;
use futures::TryFutureExt;
use serde::Deserialize;
use sqlx::FromRow;
use std::fmt::Debug;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Value(pub Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq, Eq, FromRow)]
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
}

#[async_trait]
pub trait SecretStore: Debug + Send + Sync + 'static {
    async fn get(&self, key: &str) -> Result<Value, SecretStoreError>;
    async fn put(&self, key: &str, content: Vec<u8>, force: bool) -> Result<(), SecretStoreError>;
    #[allow(dead_code)]
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, SecretStoreError>;
    async fn delete(&self, key: &str) -> Result<(), SecretStoreError>;
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")] // This handles case insensitivity during deserialization
pub enum Engine {
    #[default]
    Sqlite,
}

pub async fn new(
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
                .map_err(|err| SecretStoreError::FailedPrecondition(err.to_string()))
                .await?;
            Ok(Box::new(engine))
        }
    }
}
