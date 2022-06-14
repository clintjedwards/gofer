use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use crate::{validate_identifier, ConfigError};

#[must_use = "complete pipeline config with the .finish() method"]
#[derive(Debug, Serialize, Deserialize)]
pub struct Pipeline {
    /// Unique user defined identifier.
    pub id: String,
    /// Humanized name, meant for display.
    pub name: String,
    /// Short description of what the pipeline is used for.
    pub description: Option<String>,
    /// Controls how many runs can be active at any single time.
    /// 0 defaults to whatever the global Gofer setting is.
    pub parallelism: u64,
    /// A mapping of pipeline owned tasks.
    pub tasks: Vec<Task>,
    /// A mapping of pipeline owned triggers to their settings.
    pub triggers: Vec<PipelineTriggerConfig>,
    /// A mapping of pipeline owned notifiers to their settings.
    pub notifiers: Vec<PipelineNotifierConfig>,
}

impl From<gofer_proto::PipelineConfig> for Pipeline {
    fn from(p: gofer_proto::PipelineConfig) -> Self {
        Pipeline {
            id: p.id,
            name: p.name,
            description: {
                if p.description.is_empty() {
                    None
                } else {
                    Some(p.description)
                }
            },
            parallelism: p.parallelism,
            tasks: p.tasks.into_iter().map(|value| value.into()).collect(),
            triggers: p.triggers.into_iter().map(|value| value.into()).collect(),
            notifiers: p.notifiers.into_iter().map(|value| value.into()).collect(),
        }
    }
}

impl From<Pipeline> for gofer_proto::PipelineConfig {
    fn from(p: Pipeline) -> Self {
        gofer_proto::PipelineConfig {
            id: p.id,
            name: p.name,
            description: p.description.unwrap_or_default(),
            parallelism: p.parallelism,
            tasks: p.tasks.into_iter().map(|value| value.into()).collect(),
            triggers: p.triggers.into_iter().map(|value| value.into()).collect(),
            notifiers: p.notifiers.into_iter().map(|value| value.into()).collect(),
        }
    }
}

impl Pipeline {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            parallelism: 0,
            tasks: Vec::new(),
            triggers: Vec::new(),
            notifiers: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_identifier("id", &self.id)?;

        for task in &self.tasks {
            task.validate()?;
        }

        for trigger in &self.triggers {
            trigger.validate()?;
        }

        for notifier in &self.notifiers {
            notifier.validate()?;
        }

        Ok(())
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn parallelism(mut self, parallelism: u64) -> Self {
        self.parallelism = parallelism;
        self
    }

    pub fn tasks(mut self, tasks: Vec<Task>) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn triggers(mut self, triggers: Vec<PipelineTriggerConfig>) -> Self {
        self.triggers = triggers;
        self
    }

    pub fn notifiers(mut self, notifiers: Vec<PipelineNotifierConfig>) -> Self {
        self.notifiers = notifiers;
        self
    }

    pub fn finish(self) -> Result<(), ConfigError> {
        self.validate()?;
        println!(
            "{}",
            serde_json::to_string(&self).map_err(|e| ConfigError::Parsing(e.to_string()))?
        );
        Ok(())
    }
}

/// Every time a pipeline attempts to subscribe to a trigger, it passes certain
/// values back to that trigger for certain functionality. Since triggers keep no
/// permanent state, these settings are kept here so that when triggers are restarted
/// they can be restored with proper settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineTriggerConfig {
    /// A global unique identifier for the trigger type.
    pub kind: String,
    /// A user defined identifier for the trigger so that a pipeline with
    /// multiple notifiers can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific trigger.
    pub settings: HashMap<String, String>,
}

impl PipelineTriggerConfig {
    pub fn new(kind: &str, label: &str) -> Self {
        PipelineTriggerConfig {
            kind: kind.to_string(),
            label: label.to_string(),
            settings: HashMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_identifier("label", &self.label)?;
        Ok(())
    }

    pub fn settings(mut self, settings: HashMap<String, String>) -> Self {
        self.settings = settings;
        self
    }
}

impl From<gofer_proto::PipelineTriggerConfig> for PipelineTriggerConfig {
    fn from(p: gofer_proto::PipelineTriggerConfig) -> Self {
        PipelineTriggerConfig {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
        }
    }
}

impl From<PipelineTriggerConfig> for gofer_proto::PipelineTriggerConfig {
    fn from(p: PipelineTriggerConfig) -> Self {
        gofer_proto::PipelineTriggerConfig {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineNotifierConfig {
    /// A global unique identifier for the notifier type.
    pub kind: String,
    /// A user defined identifier for the notifier so that a pipeline with
    /// multiple notifiers can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific notifier.
    pub settings: HashMap<String, String>,
}

impl PipelineNotifierConfig {
    pub fn new(kind: &str, label: &str) -> Self {
        PipelineNotifierConfig {
            kind: kind.to_string(),
            label: label.to_string(),
            settings: HashMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_identifier("label", &self.label)?;
        Ok(())
    }

    pub fn settings(mut self, settings: HashMap<String, String>) -> Self {
        self.settings = settings;
        self
    }
}

impl From<gofer_proto::PipelineNotifierConfig> for PipelineNotifierConfig {
    fn from(p: gofer_proto::PipelineNotifierConfig) -> Self {
        PipelineNotifierConfig {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
        }
    }
}

impl From<PipelineNotifierConfig> for gofer_proto::PipelineNotifierConfig {
    fn from(p: PipelineNotifierConfig) -> Self {
        gofer_proto::PipelineNotifierConfig {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequiredParentStatus {
    Unknown,
    Any,
    Success,
    Failure,
}

impl From<gofer_proto::task::RequiredParentStatus> for RequiredParentStatus {
    fn from(r: gofer_proto::task::RequiredParentStatus) -> Self {
        match r {
            gofer_proto::task::RequiredParentStatus::Unknown => RequiredParentStatus::Unknown,
            gofer_proto::task::RequiredParentStatus::Any => RequiredParentStatus::Any,
            gofer_proto::task::RequiredParentStatus::Success => RequiredParentStatus::Success,
            gofer_proto::task::RequiredParentStatus::Failure => RequiredParentStatus::Failure,
        }
    }
}

impl From<RequiredParentStatus> for gofer_proto::task::RequiredParentStatus {
    fn from(r: RequiredParentStatus) -> Self {
        match r {
            RequiredParentStatus::Unknown => gofer_proto::task::RequiredParentStatus::Unknown,
            RequiredParentStatus::Any => gofer_proto::task::RequiredParentStatus::Any,
            RequiredParentStatus::Success => gofer_proto::task::RequiredParentStatus::Success,
            RequiredParentStatus::Failure => gofer_proto::task::RequiredParentStatus::Failure,
        }
    }
}

impl FromStr for RequiredParentStatus {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "unknown" => Ok(RequiredParentStatus::Unknown),
            "any" => Ok(RequiredParentStatus::Any),
            "success" => Ok(RequiredParentStatus::Success),
            "failure" => Ok(RequiredParentStatus::Failure),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

impl From<gofer_proto::RegistryAuth> for RegistryAuth {
    fn from(p: gofer_proto::RegistryAuth) -> Self {
        RegistryAuth {
            user: p.user,
            pass: p.pass,
        }
    }
}

impl From<RegistryAuth> for gofer_proto::RegistryAuth {
    fn from(p: RegistryAuth) -> Self {
        gofer_proto::RegistryAuth {
            user: p.user,
            pass: p.pass,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Exec {
    pub shell: String,
    pub script: String,
}

impl From<gofer_proto::Exec> for Exec {
    fn from(p: gofer_proto::Exec) -> Self {
        Exec {
            shell: p.shell,
            script: p.script,
        }
    }
}

impl From<Exec> for gofer_proto::Exec {
    fn from(p: Exec) -> Self {
        gofer_proto::Exec {
            shell: p.shell,
            script: p.script,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: Option<String>,
    pub image: String,
    pub registry_auth: Option<RegistryAuth>,
    pub depends_on: HashMap<String, RequiredParentStatus>,
    pub variables: HashMap<String, String>,
    pub exec: Option<Exec>,
}

impl Task {
    pub fn new(id: &str, image: &str) -> Self {
        Self {
            id: id.to_string(),
            description: None,
            image: image.to_string(),
            registry_auth: None,
            depends_on: HashMap::new(),
            variables: HashMap::new(),
            exec: None,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_identifier("id", &self.id)?;
        Ok(())
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn registry_auth(mut self, username: &str, password: &str) -> Self {
        self.registry_auth = Some(RegistryAuth {
            user: username.to_string(),
            pass: password.to_string(),
        });
        self
    }

    pub fn depends_on_one(mut self, task_id: &str, state: RequiredParentStatus) -> Self {
        self.depends_on.insert(task_id.to_string(), state);
        self
    }

    pub fn depends_on_many(mut self, depends_on: HashMap<String, RequiredParentStatus>) -> Self {
        self.depends_on.extend(depends_on);
        self
    }

    pub fn variable(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }

    pub fn variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables.extend(variables);
        self
    }

    pub fn exec(mut self, exec: Exec) -> Self {
        self.exec = Some(exec);
        self
    }
}

impl From<gofer_proto::TaskConfig> for Task {
    fn from(p: gofer_proto::TaskConfig) -> Self {
        Task {
            id: p.id,
            description: {
                if p.description.is_empty() {
                    None
                } else {
                    Some(p.description)
                }
            },
            image: p.image,
            registry_auth: p.registry_auth.map(RegistryAuth::from),
            depends_on: p
                .depends_on
                .into_iter()
                .map(|(key, value)| {
                    let value = gofer_proto::task::RequiredParentStatus::from_i32(value).unwrap();
                    (key, value.into())
                })
                .collect(),
            variables: p.variables,
            exec: p.exec.map(Exec::from),
        }
    }
}

impl From<Task> for gofer_proto::TaskConfig {
    fn from(p: Task) -> Self {
        gofer_proto::TaskConfig {
            id: p.id,
            description: p.description.unwrap_or_default(),
            image: p.image,
            registry_auth: p.registry_auth.map(gofer_proto::RegistryAuth::from),
            depends_on: p
                .depends_on
                .into_iter()
                .map(|(key, value)| {
                    (
                        key,
                        gofer_proto::task::RequiredParentStatus::from(value) as i32,
                    )
                })
                .collect(),
            variables: p.variables,
            exec: p.exec.map(gofer_proto::Exec::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline() {
        Pipeline::new("simple_pipeline", "Simple Pipeline")
            .description("Test Description")
            .tasks(vec![])
            .finish()
            .expect("config failed");
    }
}
