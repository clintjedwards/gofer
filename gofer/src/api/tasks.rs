use crate::{
    api::{Variable, VariableSource},
    storage,
};
use anyhow::{Context, Result};
use gofer_sdk::config::{Build, BuiltImage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumString};

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum RequiredParentStatus {
    #[default]
    Unknown,
    Any,
    Success,
    Failure,
}

impl From<gofer_sdk::config::RequiredParentStatus> for RequiredParentStatus {
    fn from(value: gofer_sdk::config::RequiredParentStatus) -> Self {
        match value {
            gofer_sdk::config::RequiredParentStatus::Unknown => RequiredParentStatus::Unknown,
            gofer_sdk::config::RequiredParentStatus::Any => RequiredParentStatus::Any,
            gofer_sdk::config::RequiredParentStatus::Success => RequiredParentStatus::Success,
            gofer_sdk::config::RequiredParentStatus::Failure => RequiredParentStatus::Failure,
        }
    }
}

/// A task represents a particular workload within a pipeline. Tasks are composable within a larger pipeline, meaning
/// they can be run before, after, or alongside other tasks. Tasks represent the lowest level of the Gofer hierarchy
/// and is what Gofer references to see how a user might want their workload handled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Task {
    /// Unique identifier for the task.
    pub id: String,

    /// Short description about the workload.
    pub description: String,

    /// How the task's image is made. `gofer up` builds it on the user's machine before it registers the config.
    pub build: Build,

    /// The image that was built for this version of the task, which is what its runs use.
    pub image: BuiltImage,

    /// Which other tasks (by id) this task depends on.
    pub depends_on: HashMap<String, RequiredParentStatus>,

    /// Variables which will be passed in as env vars to the task.
    pub variables: Vec<Variable>,

    /// Replaces the image's entrypoint; follows normal docker convention for entrypoint:
    /// https://docs.docker.com/reference/dockerfile/#entrypoint. It's already built into the image.
    pub entrypoint: Option<Vec<String>>,

    /// Replaces the image's command; follows normal docker convention of command:
    /// https://docs.docker.com/reference/dockerfile/#cmd. It's already built into the image.
    pub command: Option<Vec<String>>,

    /// Whether to inject a run specific Gofer API key. Useful for using Gofer API within the container.
    pub inject_api_token: bool,

    /// How long, in seconds, the task's container can run before Gofer stops it and fails the task. 0 means the
    /// server's `api.task_execution_max_timeout_seconds` applies, which also caps anything set here.
    pub timeout_seconds: u64,
}

impl TryFrom<gofer_sdk::config::Task> for Task {
    type Error = anyhow::Error;

    fn try_from(value: gofer_sdk::config::Task) -> Result<Self> {
        let image = value.image.with_context(|| {
            format!(
                "task '{}' has no image; `gofer up` builds one before it registers a config",
                value.id
            )
        })?;

        Ok(Task {
            id: value.id,
            description: value.description.unwrap_or_default(),
            build: value.build,
            image,
            depends_on: value
                .depends_on
                .into_iter()
                .map(|(task_id, status)| (task_id, RequiredParentStatus::from(status)))
                .collect(),
            variables: value
                .variables
                .into_iter()
                .map(|(key, value)| Variable {
                    key,
                    value,
                    source: VariableSource::PipelineConfig,
                })
                .collect(),
            entrypoint: value.entrypoint,
            command: value.command,
            inject_api_token: value.inject_api_token,
            timeout_seconds: value.timeout_seconds,
        })
    }
}

impl Task {
    pub fn to_storage(
        &self,
        namespace_id: String,
        pipeline_id: String,
        version: i64,
    ) -> Result<storage::tasks::Task> {
        let task = storage::tasks::Task {
            namespace_id,
            pipeline_id,
            pipeline_config_version: version,
            task_id: self.id.clone(),
            description: self.description.clone(),
            build: serde_json::to_string(&self.build)?,
            image_digest: self.image.digest.clone(),
            source_hash: self.image.source_hash.clone(),
            depends_on: serde_json::to_string(&self.depends_on)?,
            variables: serde_json::to_string(&self.variables)?,
            entrypoint: serde_json::to_string(&self.entrypoint)?,
            command: serde_json::to_string(&self.command)?,
            inject_api_token: self.inject_api_token,
            timeout_seconds: self.timeout_seconds.try_into()?,
        };

        Ok(task)
    }

    pub fn from_storage(storage_task: storage::tasks::Task) -> Result<Self> {
        let task = Self {
            id: storage_task.task_id,
            description: storage_task.description,
            build: serde_json::from_str(&storage_task.build)?,
            image: BuiltImage {
                digest: storage_task.image_digest,
                source_hash: storage_task.source_hash,
            },
            depends_on: serde_json::from_str(&storage_task.depends_on)?,
            variables: serde_json::from_str(&storage_task.variables)?,
            entrypoint: serde_json::from_str(&storage_task.entrypoint)?,
            command: serde_json::from_str(&storage_task.command)?,
            inject_api_token: storage_task.inject_api_token,
            timeout_seconds: storage_task.timeout_seconds.try_into()?,
        };

        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gofer_sdk::config::{Keep, Step, Steps};
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_task_storage_serialization() {
        let task = Task {
            id: "task1".to_string(),
            description: "A test task".to_string(),
            build: Build::Steps(Steps {
                base: "rust:1".into(),
                steps: vec![
                    Step::Copy {
                        from: "./app".into(),
                        to: "/src".into(),
                    },
                    Step::Run {
                        command: "cargo build --release".into(),
                    },
                ],
                keep: Some(Keep {
                    paths: vec!["/src/target/release/app".into()],
                    base: "debian:13-slim".into(),
                }),
            }),
            image: BuiltImage {
                digest: format!("sha256:{}", "a".repeat(64)),
                source_hash: "b".repeat(64),
            },
            depends_on: HashMap::from([("task_0".into(), RequiredParentStatus::Success)]),
            variables: vec![Variable {
                key: "test".into(),
                value: "value".into(),
                source: VariableSource::System,
            }],
            entrypoint: Some(vec!["/entrypoint.sh".to_string()]),
            command: Some(vec!["run".to_string(), "--option".to_string()]),
            inject_api_token: true,
            timeout_seconds: 600,
        };

        let storage_task = task
            .to_storage("ns1".into(), "pipeline1".into(), 1)
            .expect("Failed to convert Task to storage::tasks::Task");
        assert_eq!(storage_task.namespace_id, "ns1");
        assert_eq!(storage_task.pipeline_id, "pipeline1");
        assert_eq!(storage_task.pipeline_config_version, 1);
        assert_eq!(storage_task.image_digest, task.image.digest);

        let reconstructed_task = Task::from_storage(storage_task)
            .expect("Failed to convert storage::tasks::Task to Task");
        assert_eq!(reconstructed_task, task);
    }

    #[test]
    fn test_tasks_need_an_image_to_be_registered() {
        let task = gofer_sdk::config::Task::new("report", "rust:1");
        let error = Task::try_from(task).unwrap_err().to_string();
        assert!(error.contains("`gofer up` builds one"), "{error}");
    }
}
