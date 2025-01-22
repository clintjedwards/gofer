pub mod filesystem;

use async_trait::async_trait;
use bytes::Bytes;
use serde::Deserialize;
use std::{fmt::Debug, pin::Pin};
use strum::{Display, EnumString};
use tokio_stream::Stream;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Object {
    pub key: String,
    pub content: Bytes,
}

/// Represents different object store failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ObjectStoreError {
    #[allow(dead_code)]
    #[error("could not establish connection to object store; {0}")]
    Connection(String),

    #[error("requested entity not found")]
    NotFound,

    #[error("entity already exists")]
    Exists,

    #[error("unexpected storage error occurred")]
    Internal(String),

    /// Failed to start due to misconfigured settings, usually from a misconfigured settings file.
    #[error("could not init object store; {0}")]
    FailedPrecondition(String),
}

#[async_trait]
pub trait ObjectStore: Debug + Send + Sync + 'static {
    async fn exists(&self, key: &str) -> Result<bool, ObjectStoreError>;
    async fn get(&self, key: &str) -> Result<Bytes, ObjectStoreError>;
    async fn get_stream(
        &self,
        key: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, ObjectStoreError>> + Send>>, ObjectStoreError>;

    #[allow(dead_code)]
    async fn put(&self, key: &str, content: Bytes, force: bool) -> Result<(), ObjectStoreError>;
    async fn put_stream(
        &self,
        key: &str,
        mut content: Pin<Box<dyn Stream<Item = Bytes> + Send>>,
    ) -> Result<(), ObjectStoreError>;

    #[allow(dead_code)]
    async fn delete(&self, key: &str) -> Result<(), ObjectStoreError>;
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")] // This handles case insensitivity during deserialization
pub enum Engine {
    #[default]
    Filesystem,
}

pub async fn new(
    config: &crate::conf::api::ObjectStore,
) -> Result<Box<dyn ObjectStore>, ObjectStoreError> {
    #[allow(clippy::match_single_binding)]
    match config.engine {
        Engine::Filesystem => {
            if config.filesystem.is_none() {
                return Err(ObjectStoreError::FailedPrecondition(
                    "Filesystem engine settings not found in config".into(),
                ));
            }

            let engine = filesystem::Engine::new(&config.clone().filesystem.unwrap()).await;
            Ok(Box::new(engine))
        }
    }
}
