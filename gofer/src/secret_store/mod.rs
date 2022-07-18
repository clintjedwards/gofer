mod embedded;

use crate::conf;
use async_trait::async_trait;
use econf::LoadEnv;
use serde::Deserialize;
use slog_scope::error;
use std::fmt::Debug;
use std::sync::Arc;
use strum::{Display, EnumString};

#[cfg(test)]
mod tests;

/// Represents different secret store failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum SecretStoreError {
    #[error("unknown error occurred; {0}")]
    Unknown(String),

    #[error("requested entity not found")]
    NotFound,

    #[error("entity already exists")]
    Exists,

    #[error("entity was not in correct state for operation")]
    FailedPrecondition,

    #[error("could not init store; {0}")]
    FailedInitPrecondition(String),

    #[error("could not encrypt/decrypt key; {0}")]
    FailedEncryption(String),
}

/// The store trait defines what the interface between Gofer and a Secret store should adhere to.
#[async_trait]
pub trait Store: Debug {
    async fn get_secret(&self, key: &str) -> Result<Vec<u8>, SecretStoreError>;
    async fn put_secret(&self, key: &str, value: &str, force: bool)
        -> Result<(), SecretStoreError>;
    async fn delete_secret(&self, key: &str) -> Result<(), SecretStoreError>;
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Display, EnumString, LoadEnv)]
pub enum Engine {
    Embedded,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::Embedded
    }
}

pub async fn init_secret_store(
    config: &conf::api::SecretStore,
) -> Result<Arc<dyn Store + Send + Sync>, SecretStoreError> {
    #[allow(clippy::match_single_binding)]
    match config.engine {
        Engine::Embedded => {
            if let Some(config) = &config.embedded {
                let engine = embedded::Engine::new(&config.path, &config.encryption_key).await?;
                Ok(Arc::new(engine))
            } else {
                Err(SecretStoreError::FailedInitPrecondition(
                    "engine settings not found in config".into(),
                ))
            }
        }
    }
}
