//! # Gofer Types
//!
//! This crate exists to separate the types that Gofer uses from business logic of the API.
//! This is purely so that we can set up our dropshot library (which provides automatic OpenAPI generation) with
//! a trait that allows us to define what the API looks like.
//!
//! By separating the types and trait that dropshot needs into a separate crate we prevent unneeded recompilations
//! of OpenAPI code. Speeding up the development cycle.
//!
//! It's also somewhat nice to have a single crate that just handles types to avoid cyclical relationships.
mod api;
mod api_service;
mod storage;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use strum::{Display, EnumString};

/// Return the current epoch time in milliseconds.
pub fn epoch_milli() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Authentication information for container registries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

// impl From<gofer_sdk::config::RegistryAuth> for RegistryAuth {
//     fn from(value: gofer_sdk::config::RegistryAuth) -> Self {
//         RegistryAuth {
//             user: value.user,
//             pass: value.pass,
//         }
//     }
// }

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum VariableSource {
    #[default]
    Unknown,

    /// From the user's own pipeline configuration.
    PipelineConfig,

    /// From the Gofer API executor itself.
    System,

    /// Injected at the beginning of a particular run.
    RunOptions,

    /// Injected by a subscribed extension.
    Extension,
}

/// A variable is a key value pair that is used either at a run or task level.
/// The variable is inserted as an environment variable to an eventual task execution.
/// It can be owned by different parts of the system which control where the potentially
/// sensitive variables might show up.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Variable {
    pub key: String,
    pub value: String,
    pub source: VariableSource,
}
