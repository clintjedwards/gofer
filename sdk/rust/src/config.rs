use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use crate::{validate_identifier, ConfigError};

#[must_use = "complete pipeline config with the .finish() method"]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// A mapping of pipeline owned commontasks to their settings.
    pub common_tasks: Vec<PipelineCommonTaskConfig>,
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
            common_tasks: Vec::new(),
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

        for common_task in &self.common_tasks {
            common_task.validate()?;
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

    pub fn common_tasks(mut self, common_tasks: Vec<PipelineCommonTaskConfig>) -> Self {
        self.common_tasks = common_tasks;
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
#[serde(rename_all = "lowercase")]
pub struct PipelineTriggerConfig {
    /// A global unique identifier for the trigger type.
    pub name: String,
    /// A user defined identifier for the trigger so that a pipeline with
    /// multiple commontasks can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific trigger.
    pub settings: HashMap<String, String>,
}

impl PipelineTriggerConfig {
    pub fn new(name: &str, label: &str) -> Self {
        PipelineTriggerConfig {
            name: name.to_string(),
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct PipelineCommonTaskConfig {
    /// A global unique identifier for the commontask type.
    pub name: String,
    /// A user defined identifier for the commontask so that a pipeline with
    /// multiple commontasks can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific commontask.
    pub settings: HashMap<String, String>,
}

impl PipelineCommonTaskConfig {
    pub fn new(name: &str, label: &str) -> Self {
        PipelineCommonTaskConfig {
            name: name.to_string(),
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequiredParentStatus {
    Unknown,
    Any,
    Success,
    Failure,
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
#[serde(rename_all = "lowercase")]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Task {
    pub id: String,
    pub description: Option<String>,
    pub image: String,
    pub registry_auth: Option<RegistryAuth>,
    pub depends_on: HashMap<String, RequiredParentStatus>,
    pub variables: HashMap<String, String>,
    pub entrypoint: Vec<String>,
    pub command: Vec<String>,
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
            entrypoint: Vec::new(),
            command: Vec::new(),
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

    pub fn variables(mut self, variables: HashMap<&str, &str>) -> Self {
        let variables: HashMap<String, String> = variables
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        self.variables.extend(variables);
        self
    }

    pub fn entrypoint(mut self, entrypoint: Vec<&str>) -> Self {
        self.entrypoint = entrypoint.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn command(mut self, command: Vec<&str>) -> Self {
        self.command = command.into_iter().map(|s| s.to_string()).collect();
        self
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
