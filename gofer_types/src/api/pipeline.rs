use super::pipeline_config;
use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelinePathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelinePathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum PipelineState {
    #[default]
    Unknown,

    Active,

    Disabled,
}

/// Details about the pipeline itself, not including the configuration that the user can change.
/// All these values are changed by the system or never changed at all. This sits in contrast to
/// the config which the user can change freely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Metadata {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Time of pipeline creation in epoch milliseconds.
    pub created: u64,

    /// Time pipeline was updated to a new version in epoch milliseconds.
    pub modified: u64,

    /// The current running state of the pipeline. This is used to determine if the pipeline should run or not.
    pub state: PipelineState,
}

impl Metadata {
    pub fn new(namespace_id: &str, pipeline_id: &str) -> Self {
        Metadata {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            created: epoch_milli(),
            modified: 0,
            state: PipelineState::Active,
        }
    }
}

impl TryFrom<storage::pipeline_metadata::PipelineMetadata> for Metadata {
    type Error = anyhow::Error;

    fn try_from(value: storage::pipeline_metadata::PipelineMetadata) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let modified = value.modified.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'modified' from storage value '{}'",
                value.modified
            )
        })?;

        let state = PipelineState::from_str(&value.state).with_context(|| {
            format!(
                "Could not parse field 'token type' from storage value '{}'",
                value.state
            )
        })?;

        Ok(Metadata {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            created,
            modified,
            state,
        })
    }
}

impl From<Metadata> for storage::pipeline_metadata::PipelineMetadata {
    fn from(value: Metadata) -> Self {
        Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            created: value.created.to_string(),
            modified: value.modified.to_string(),
            state: value.state.to_string(),
        }
    }
}

/// A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
/// Pipeline is a secondary level unit being contained within namespaces and containing runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Pipeline {
    /// Macro level details on the targeted pipeline.
    pub metadata: Metadata,

    /// User controlled data for the targeted pipeline.
    pub config: pipeline_config::Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelinesResponse {
    /// A list of all pipelines metadata.
    pub pipelines: Vec<Metadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineResponse {
    /// The metadata for the pipeline.
    pub pipeline: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdatePipelineRequest {
    pub state: Option<PipelineState>,
}
