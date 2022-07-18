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

/// Represents different object store failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ObjectStoreError {
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
}

/// The store trait defines what the interface between Gofer and an Object store should adhere to.
#[async_trait]
pub trait Store: Debug {
    async fn get_object(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError>;
    async fn put_object(
        &self,
        key: &str,
        value: Vec<u8>,
        force: bool,
    ) -> Result<(), ObjectStoreError>;
    async fn delete_object(&self, key: &str) -> Result<(), ObjectStoreError>;
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

pub async fn init_object_store(
    config: &conf::api::ObjectStore,
) -> Result<Arc<dyn Store + Send + Sync>, ObjectStoreError> {
    #[allow(clippy::match_single_binding)]
    match config.engine {
        Engine::Embedded => {
            if let Some(config) = &config.embedded {
                let engine = embedded::Engine::new(&config.path).await?;
                Ok(Arc::new(engine))
            } else {
                Err(ObjectStoreError::FailedInitPrecondition(
                    "engine settings not found in config".into(),
                ))
            }
        }
    }
}
