use super::task;
use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use gofer_sdk;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineConfigPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineConfigPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The version of the configuration you want to target. 0 means return the latest.
    pub version: i64,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum ConfigState {
    #[default]
    Unknown,

    /// Has never been deployed.
    Unreleased,

    /// Currently deployed.
    Live,

    /// Has previously been deployed and is now defunct.
    Deprecated,
}

/// A representation of the user's configuration settings for a particular pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// The iteration number for this pipeline's configs.
    pub version: u64,

    /// The amount of runs allowed to happen at any given time.
    pub parallelism: u64,

    /// Human readable name for pipeline.
    pub name: String,

    /// Description of pipeline's purpose and other details.
    pub description: String,

    /// Tasks associated with this pipeline.
    pub tasks: HashMap<String, task::Task>,

    /// The deployment state of the config. This is used to determine the state of this particular config and if it
    /// is currently being used or not.
    pub state: ConfigState,

    /// Time in epoch milliseconds when this pipeline config was registered.
    pub registered: u64,

    /// Time in epoch milliseconds when this pipeline config was not longer used.
    pub deprecated: u64,
}

impl Config {
    pub fn new(
        namespace_id: &str,
        pipeline_id: &str,
        version: u64,
        config: gofer_sdk::config::Pipeline,
    ) -> Result<Self> {
        Ok(Config {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            version,
            parallelism: config.parallelism.try_into()?,
            name: config.name,
            description: config.description.unwrap_or_default(),
            tasks: config
                .tasks
                .into_iter()
                .map(|task| (task.id.clone(), task::Task::from(task)))
                .collect(),
            state: ConfigState::Unreleased,
            registered: epoch_milli(),
            deprecated: 0,
        })
    }
}

impl Config {
    pub fn to_storage(
        &self,
    ) -> Result<(
        storage::pipeline_config::PipelineConfig,
        Vec<storage::task::Task>,
    )> {
        let config = storage::pipeline_config::PipelineConfig {
            namespace_id: self.namespace_id.clone(),
            pipeline_id: self.pipeline_id.clone(),
            version: self.version.try_into()?,
            parallelism: self.parallelism.try_into()?,
            name: self.name.clone(),
            description: self.description.clone(),
            registered: self.registered.to_string(),
            deprecated: self.deprecated.to_string(),
            state: self.state.to_string(),
        };

        let mut tasks: Vec<storage::task::Task> = vec![];
        for task in self.tasks.values() {
            let storage_task = task
                .to_storage(
                    self.namespace_id.clone(),
                    self.pipeline_id.clone(),
                    self.version.try_into()?,
                )
                .context("Could not properly serialize task to DB")?;

            tasks.push(storage_task);
        }

        Ok((config, tasks))
    }

    pub fn from_storage(
        config: storage::pipeline_config::PipelineConfig,
        tasks: Vec<storage::task::Task>,
    ) -> Result<Self> {
        let registered = config.registered.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'registered' from storage value '{}'",
                config.registered
            )
        })?;

        let deprecated = config.deprecated.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'deprecated' from storage value '{}'",
                config.deprecated
            )
        })?;

        let state = ConfigState::from_str(&config.state).with_context(|| {
            format!(
                "Could not parse field 'state' from storage value '{}'",
                config.state
            )
        })?;

        Ok(Config {
            namespace_id: config.namespace_id,
            pipeline_id: config.pipeline_id,
            version: config.version.try_into()?,
            parallelism: config.parallelism.try_into()?,
            name: config.name,
            description: config.description,
            tasks: tasks
                .into_iter()
                .map(|task| {
                    (
                        task.task_id.clone(),
                        task::Task::from_storage(task).unwrap(),
                    )
                })
                .collect(),
            state,
            registered,
            deprecated,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelineConfigsResponse {
    /// A list of all pipelines configs.
    pub configs: Vec<Config>,
}
