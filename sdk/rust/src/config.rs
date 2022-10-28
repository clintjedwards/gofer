use crate::{dag::DAGError, dag::Dag, validate_identifier, validate_variables};
use downcast_rs::{impl_downcast, Downcast};
use gofer_proto::{
    CommonTaskConfig, CustomTaskConfig, PipelineConfig, PipelineTaskConfig, PipelineTriggerConfig,
};
use prost::Message;
use std::{collections::HashMap, io::Write};
use strum::{Display, EnumString};

#[derive(Debug, Display, EnumString, PartialEq, Eq, Clone)]
pub enum RequiredParentStatus {
    Unknown,
    Any,
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

#[derive(Debug, Display, EnumString, PartialEq, Eq, Clone)]
pub enum TaskKind {
    Unknown,
    Common,
    Custom,
}

pub trait Task: std::fmt::Debug + Downcast {
    fn kind(&self) -> TaskKind;
    fn id(&self) -> String;
    fn depends_on(&self) -> HashMap<String, RequiredParentStatus>;
    fn validate(&self) -> Result<(), ConfigError>;
}
impl_downcast!(Task);

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ConfigError {
    #[error("unknown error found during config parsing; {0}")]
    Unknown(String),

    #[error("invalid {argument}: '{value}'; {description}")]
    InvalidArgument {
        argument: String,
        value: String,
        description: String,
    },

    #[error("could not parse config; {0}")]
    Parsing(String),

    #[error("duplicate task names found; {0} shares an identifier with a task already logged")]
    IdenticalTaskNames(String),

    #[error("a cycle was detected created a dependency from task {0} to task {1}")]
    TaskCycle(String, String),

    #[error("task {0} is listed as a dependency within task {1} but does not exist")]
    DependencyNotFound(String, String),
}

/// Representation of a pipeline configuration.
#[must_use = "complete pipeline config with the .finish() method"]
#[derive(Debug)]
pub struct Pipeline {
    /// Unique user defined identifier.
    pub id: String,
    /// Humanized name, meant for display.
    pub name: String,
    /// Short description of what the pipeline is used for.
    pub description: Option<String>,
    /// Controls how many runs can be active at any single time.
    /// 0 defaults to whatever the global Gofer setting is.
    pub parallelism: i64,
    /// A mapping of pipeline owned tasks.
    pub tasks: Vec<Box<dyn Task>>,
    /// A mapping of pipeline owned triggers to their settings.
    pub triggers: Vec<Trigger>,
}

impl Pipeline {
    /// Construct a new pipeline.
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            parallelism: 0,
            tasks: Vec::new(),
            triggers: Vec::new(),
        }
    }

    fn is_dag(&self) -> Result<(), ConfigError> {
        let mut pipeline_dag = Dag::new();

        // Add all nodes first.
        for task in &self.tasks {
            pipeline_dag
                .add_node(&task.id())
                .map_err(|_| ConfigError::IdenticalTaskNames(task.id()))?
        }

        // Then add all edges.
        for task in &self.tasks {
            for id in task.depends_on().keys() {
                if let Err(e) = pipeline_dag.add_edge(id, &task.id()) {
                    match e {
                        DAGError::EdgeCreatesCycle(node1, node2) => {
                            return Err(ConfigError::TaskCycle(node1, node2))
                        }
                        DAGError::EntityNotFound => {
                            return Err(ConfigError::DependencyNotFound(task.id(), id.to_string()))
                        }
                        _ => return Err(ConfigError::Unknown(e.to_string())),
                    }
                }
            }
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_identifier("id", &self.id)?;

        self.is_dag()?;

        for task in &self.tasks {
            task.validate()?;
        }

        for trigger in &self.triggers {
            trigger.validate()?;
        }

        Ok(())
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn parallelism(mut self, parallelism: i64) -> Self {
        self.parallelism = parallelism;
        self
    }

    pub fn tasks(mut self, tasks: Vec<Box<dyn Task>>) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn triggers(mut self, triggers: Vec<Trigger>) -> Self {
        self.triggers = triggers;
        self
    }

    pub fn finish(self) -> Result<(), ConfigError> {
        self.validate()?;

        let pipeline_proto = self.proto();
        let pipeline_bytes = pipeline_proto.encode_to_vec();

        std::io::stdout().write_all(&pipeline_bytes).unwrap();
        std::io::stdout().flush().unwrap();

        Ok(())
    }

    fn proto(&self) -> PipelineConfig {
        let mut tasks: Vec<PipelineTaskConfig> = vec![];

        for task in &self.tasks {
            match task.kind() {
                TaskKind::Unknown => {
                    panic!("TaskKind for Task {} is found as Unknown; This should never happen; Please report this error.", task.id())
                }
                TaskKind::Common => {
                    let common_task = task.downcast_ref::<CommonTask>().expect("Could not unwrap task properly; This should never happen; Please report this error.");

                    tasks.push(PipelineTaskConfig {
                        task: Some(gofer_proto::pipeline_task_config::Task::CommonTask(
                            common_task.proto(),
                        )),
                    })
                }
                TaskKind::Custom => {
                    let custom_task = task.downcast_ref::<CustomTask>().expect("Could not unwrap task properly; This should never happen; Please report this error.");

                    tasks.push(PipelineTaskConfig {
                        task: Some(gofer_proto::pipeline_task_config::Task::CustomTask(
                            custom_task.proto(),
                        )),
                    })
                }
            }
        }

        let mut triggers: Vec<PipelineTriggerConfig> = vec![];

        for trigger in &self.triggers {
            triggers.push(trigger.proto())
        }

        PipelineConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone().unwrap_or_default(),
            parallelism: self.parallelism,
            tasks,
            triggers,
        }
    }
}

pub fn pipeline_secret(key: &str) -> String {
    format!("pipeline_secret{{{}}}", key)
}

pub fn pipeline_object(key: &str) -> String {
    format!("pipeline_object{{{}}}", key)
}

pub fn run_object(key: &str) -> String {
    format!("run_object{{{}}}", key)
}

/// Every time a pipeline attempts to subscribe to a trigger, it passes certain
/// values back to that trigger for certain functionality. Since triggers keep no
/// permanent state, these settings are kept here so that when triggers are restarted
/// they can be restored with proper settings.
#[derive(Debug, PartialEq, Eq)]
pub struct Trigger {
    /// A global unique identifier for the trigger type.
    pub name: String,
    /// A user defined identifier for the trigger so that a pipeline with
    /// multiple commontasks can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific trigger.
    pub settings: HashMap<String, String>,
}

impl Trigger {
    pub fn new(name: &str, label: &str) -> Self {
        Trigger {
            name: name.to_string(),
            label: label.to_string(),
            settings: HashMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_identifier("label", &self.label)?;
        Ok(())
    }

    pub fn setting(mut self, key: &str, value: &str) -> Self {
        self.settings.insert(
            format!("GOFER_PLUGIN_PARAM_{}", key.to_uppercase()),
            value.to_string(),
        );
        self
    }

    pub fn settings(mut self, settings: HashMap<String, String>) -> Self {
        let settings: HashMap<String, String> = settings
            .iter()
            .map(|(key, value)| {
                (
                    format!("GOFER_PLUGIN_PARAM_{}", key.to_uppercase()),
                    value.clone(),
                )
            })
            .collect();
        self.settings.extend(settings);
        self
    }

    fn proto(&self) -> PipelineTriggerConfig {
        PipelineTriggerConfig {
            name: self.name.clone(),
            label: self.label.clone(),
            settings: self.settings.clone(),
        }
    }
}

#[derive(Debug)]
pub struct CommonTask {
    pub kind: TaskKind,
    /// A global unique identifier for the commontask type.
    pub name: String,
    /// A user defined identifier for the commontask so that a pipeline with
    /// multiple commontasks can be differentiated.
    pub label: String,
    pub description: Option<String>,
    pub depends_on: HashMap<String, RequiredParentStatus>,
    /// The settings for pertaining to that specific commontask.
    pub settings: HashMap<String, String>,
}

impl CommonTask {
    pub fn new(name: &str, label: &str) -> Self {
        CommonTask {
            kind: TaskKind::Common,
            name: name.to_string(),
            label: label.to_string(),
            description: None,
            depends_on: HashMap::new(),
            settings: HashMap::new(),
        }
    }

    pub fn setting(mut self, key: &str, value: &str) -> Self {
        self.settings.insert(
            format!("GOFER_PLUGIN_PARAM_{}", key.to_uppercase()),
            value.to_string(),
        );
        self
    }

    pub fn settings(mut self, settings: HashMap<String, String>) -> Self {
        self.settings.extend(settings);
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn depends_on(mut self, task_id: &str, state: RequiredParentStatus) -> Self {
        self.depends_on.insert(task_id.to_string(), state);
        self
    }

    pub fn depends_on_many(mut self, depends_on: HashMap<String, RequiredParentStatus>) -> Self {
        self.depends_on.extend(depends_on);
        self
    }

    fn proto(&self) -> CommonTaskConfig {
        let mut depends_on: HashMap<String, i32> = HashMap::new();
        for (key, value) in &self.depends_on {
            let value = match value {
                RequiredParentStatus::Unknown => {
                    gofer_proto::common_task_config::RequiredParentStatus::Unknown
                }
                RequiredParentStatus::Any => {
                    gofer_proto::common_task_config::RequiredParentStatus::Any
                }
                RequiredParentStatus::Success => {
                    gofer_proto::common_task_config::RequiredParentStatus::Success
                }
                RequiredParentStatus::Failure => {
                    gofer_proto::common_task_config::RequiredParentStatus::Failure
                }
            };

            depends_on.insert(key.clone(), value.into());
        }

        CommonTaskConfig {
            name: self.name.clone(),
            label: self.label.clone(),
            description: self.description.clone().unwrap_or_default(),
            depends_on,
            settings: self.settings.clone(),
        }
    }
}

impl Task for CommonTask {
    fn validate(&self) -> Result<(), ConfigError> {
        validate_variables(self.settings.clone())?;
        validate_identifier("label", &self.label)?;
        Ok(())
    }

    fn kind(&self) -> TaskKind {
        TaskKind::Common
    }

    fn id(&self) -> String {
        self.label.clone()
    }

    fn depends_on(&self) -> HashMap<String, RequiredParentStatus> {
        self.depends_on.clone()
    }
}

#[derive(Debug)]
pub struct CustomTask {
    pub kind: TaskKind,
    pub id: String,
    pub description: Option<String>,
    pub image: String,
    pub registry_auth: Option<RegistryAuth>,
    pub depends_on: HashMap<String, RequiredParentStatus>,
    pub variables: HashMap<String, String>,
    pub entrypoint: Option<Vec<String>>,
    pub command: Option<Vec<String>>,
}

impl CustomTask {
    pub fn new(id: &str, image: &str) -> Self {
        CustomTask {
            kind: TaskKind::Custom,
            id: id.to_string(),
            description: None,
            image: image.to_string(),
            registry_auth: None,
            depends_on: HashMap::new(),
            variables: HashMap::new(),
            entrypoint: None,
            command: None,
        }
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn registry_auth(mut self, user: &str, pass: &str) -> Self {
        self.registry_auth = Some(RegistryAuth {
            user: user.to_string(),
            pass: pass.to_string(),
        });
        self
    }

    pub fn depends_on(mut self, task_id: &str, state: RequiredParentStatus) -> Self {
        self.depends_on.insert(task_id.to_string(), state);
        self
    }

    pub fn depends_on_many(mut self, depends_on: HashMap<String, RequiredParentStatus>) -> Self {
        self.depends_on.extend(depends_on);
        self
    }

    pub fn variable(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(
            format!("GOFER_PLUGIN_PARAM_{}", key.to_uppercase()),
            value.to_string(),
        );
        self
    }

    pub fn variables(mut self, variables: HashMap<String, String>) -> Self {
        let variables: HashMap<String, String> = variables
            .iter()
            .map(|(key, value)| {
                (
                    format!("GOFER_PLUGIN_PARAM_{}", key.to_uppercase()),
                    value.clone(),
                )
            })
            .collect();

        self.variables.extend(variables);
        self
    }

    pub fn entrypoint(mut self, entrypoint: Vec<String>) -> Self {
        self.entrypoint = Some(entrypoint);
        self
    }

    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    fn proto(&self) -> CustomTaskConfig {
        let mut depends_on: HashMap<String, i32> = HashMap::new();
        for (key, value) in &self.depends_on {
            let value = match value {
                RequiredParentStatus::Unknown => {
                    gofer_proto::custom_task_config::RequiredParentStatus::Unknown
                }
                RequiredParentStatus::Any => {
                    gofer_proto::custom_task_config::RequiredParentStatus::Any
                }
                RequiredParentStatus::Success => {
                    gofer_proto::custom_task_config::RequiredParentStatus::Success
                }
                RequiredParentStatus::Failure => {
                    gofer_proto::custom_task_config::RequiredParentStatus::Failure
                }
            };

            depends_on.insert(key.clone(), value.into());
        }

        CustomTaskConfig {
            id: self.id.clone(),
            description: self.description.clone().unwrap_or_default(),
            image: self.image.clone(),
            registry_auth: self
                .registry_auth
                .clone()
                .map(|ra| gofer_proto::RegistryAuth {
                    user: ra.user,
                    pass: ra.pass,
                }),
            depends_on,
            variables: self.variables.clone(),
            entrypoint: self.entrypoint.clone().unwrap_or_default(),
            command: self.command.clone().unwrap_or_default(),
        }
    }
}

impl Task for CustomTask {
    fn validate(&self) -> Result<(), ConfigError> {
        validate_variables(self.variables.clone())?;
        validate_identifier("id", &self.id)?;
        Ok(())
    }

    fn kind(&self) -> TaskKind {
        TaskKind::Custom
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn depends_on(&self) -> HashMap<String, RequiredParentStatus> {
        self.depends_on.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_pipeline_cyclical() {
        let task_a = CustomTask::new("task_a", "").depends_on("task_b", RequiredParentStatus::Any);
        let task_b = CustomTask::new("task_b", "").depends_on("task_c", RequiredParentStatus::Any);
        let task_c = CustomTask::new("task_c", "").depends_on("task_a", RequiredParentStatus::Any);

        let result = Pipeline::new("invalid_pipeline", "")
            .tasks(vec![Box::new(task_a), Box::new(task_b), Box::new(task_c)])
            .finish();

        assert_eq!(
            std::mem::discriminant(&result.unwrap_err()),
            std::mem::discriminant(&ConfigError::TaskCycle("".to_string(), "".to_string())),
        )
    }

    // Test that pipeline validation fails if user attempts to request a global variable.
    #[test]
    fn test_invalid_config_global_secrets() {
        let result = Pipeline::new("simple_test_pipeline", "Simple Test Pipeline")
            .description("Simple Test Pipeline")
            .tasks(vec![Box::new(
                CustomTask::new("simple_task", "ubuntu:latest")
                    .description("This task simply prints our hello-world message and exists!")
                    .command(vec!["echo".to_string(), "Hello from Gofer!".to_string()])
                    .variable("test_var", "global_secret{{some_secret_here}}"),
            )])
            .finish();

        assert_eq!(
            std::mem::discriminant(&result.unwrap_err()),
            std::mem::discriminant(&ConfigError::InvalidArgument {
                argument: "".to_string(),
                value: "".to_string(),
                description: "".to_string()
            }),
        )
    }
}
