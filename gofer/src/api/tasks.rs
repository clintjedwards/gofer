use crate::{
    api::{RegistryAuth, Variable, VariableSource},
    storage,
};
use anyhow::Result;
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

    /// Which container image to run for this specific task.
    ///
    /// Example: "ubuntu:latest"
    pub image: String,

    /// Auth credentials for the image's registry
    pub registry_auth: Option<RegistryAuth>,

    /// Which other tasks (by id) this task depends on.
    pub depends_on: HashMap<String, RequiredParentStatus>,

    /// Variables which will be passed in as env vars to the task.
    pub variables: Vec<Variable>,

    /// Command to run on init of container; follows normal docker convention for entrypoint: https://docs.docker.com/reference/dockerfile/#entrypoint
    pub entrypoint: Option<Vec<String>>,

    /// Command to run on init of container; follows normal docker convention of command: https://docs.docker.com/reference/dockerfile/#cmd
    pub command: Option<Vec<String>>,

    /// Whether to inject a run specific Gofer API key. Useful for using Gofer API within the container.
    pub inject_api_token: bool,
}

impl From<gofer_sdk::config::Task> for Task {
    fn from(value: gofer_sdk::config::Task) -> Self {
        Task {
            id: value.id,
            description: value.description.unwrap_or_default(),
            image: value.image,
            registry_auth: value.registry_auth.map(RegistryAuth::from),
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
        }
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
            image: self.image.clone(),
            registry_auth: serde_json::to_string(&self.registry_auth)?,
            depends_on: serde_json::to_string(&self.depends_on)?,
            variables: serde_json::to_string(&self.variables)?,
            entrypoint: serde_json::to_string(&self.entrypoint)?,
            command: serde_json::to_string(&self.command)?,
            inject_api_token: self.inject_api_token,
        };

        Ok(task)
    }

    pub fn from_storage(storage_task: storage::tasks::Task) -> Result<Self> {
        let task = Self {
            id: storage_task.task_id,
            description: storage_task.description,
            image: storage_task.image,
            registry_auth: serde_json::from_str(&storage_task.registry_auth)?,
            depends_on: serde_json::from_str(&storage_task.depends_on)?,
            variables: serde_json::from_str(&storage_task.variables)?,
            entrypoint: serde_json::from_str(&storage_task.entrypoint)?,
            command: serde_json::from_str(&storage_task.command)?,
            inject_api_token: storage_task.inject_api_token,
        };

        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_storage_serialization() {
        let task = Task {
            id: "task1".to_string(),
            description: "A test task".to_string(),
            image: "rust:latest".to_string(),
            registry_auth: None,
            depends_on: HashMap::from([("task_0".into(), RequiredParentStatus::Success)]),
            variables: vec![Variable {
                key: "test".into(),
                value: "value".into(),
                source: VariableSource::System,
            }],
            entrypoint: Some(vec!["/entrypoint.sh".to_string()]),
            command: Some(vec!["run".to_string(), "--option".to_string()]),
            inject_api_token: true,
        };

        let namespace_id = "ns1".to_string();
        let pipeline_id = "pipeline1".to_string();
        let version = 1i64;

        // Testing to_storage
        let storage_task = task
            .to_storage(namespace_id.clone(), pipeline_id.clone(), version)
            .expect("Failed to convert Task to storage::tasks::Task");
        assert_eq!(storage_task.namespace_id, namespace_id);
        assert_eq!(storage_task.pipeline_id, pipeline_id);
        assert_eq!(storage_task.pipeline_config_version, version);
        assert_eq!(storage_task.task_id, task.id);
        assert_eq!(storage_task.description, task.description);
        assert_eq!(storage_task.image, task.image);
        assert_eq!(
            storage_task.registry_auth,
            serde_json::to_string(&task.registry_auth).unwrap()
        );
        assert_eq!(
            storage_task.depends_on,
            serde_json::to_string(&task.depends_on).unwrap()
        );
        assert_eq!(
            storage_task.variables,
            serde_json::to_string(&task.variables).unwrap()
        );
        assert_eq!(
            storage_task.entrypoint,
            serde_json::to_string(&task.entrypoint).unwrap()
        );
        assert_eq!(
            storage_task.command,
            serde_json::to_string(&task.command).unwrap()
        );
        assert_eq!(storage_task.inject_api_token, task.inject_api_token);

        // Testing from_storage
        let reconstructed_task = Task::from_storage(storage_task)
            .expect("Failed to convert storage::tasks::Task to Task");
        assert_eq!(reconstructed_task.id, task.id);
        assert_eq!(reconstructed_task.description, task.description);
        assert_eq!(reconstructed_task.image, task.image);
        assert_eq!(reconstructed_task.registry_auth, task.registry_auth);
        assert_eq!(reconstructed_task.depends_on, task.depends_on);
        assert_eq!(reconstructed_task.variables, task.variables);
        assert_eq!(reconstructed_task.entrypoint, task.entrypoint);
        assert_eq!(reconstructed_task.command, task.command);
        assert_eq!(reconstructed_task.inject_api_token, task.inject_api_token);
    }
}
