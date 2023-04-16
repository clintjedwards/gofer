use crate::{dag::DAGError, dag::Dag, validate_identifier};
use gofer_proto::{UserPipelineConfig, UserPipelineTaskConfig};
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
    pub tasks: Vec<Task>,
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
        }
    }

    fn is_dag(&self) -> Result<(), ConfigError> {
        let mut pipeline_dag = Dag::new();

        // Add all nodes first.
        for task in &self.tasks {
            pipeline_dag
                .add_node(&task.id)
                .map_err(|_| ConfigError::IdenticalTaskNames(task.id.clone()))?
        }

        // Then add all edges.
        for task in &self.tasks {
            for id in task.depends_on.keys() {
                if let Err(e) = pipeline_dag.add_edge(id, &task.id) {
                    match e {
                        DAGError::EdgeCreatesCycle(node1, node2) => {
                            return Err(ConfigError::TaskCycle(node1, node2))
                        }
                        DAGError::EntityNotFound => {
                            return Err(ConfigError::DependencyNotFound(
                                task.id.clone(),
                                id.to_string(),
                            ))
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

    pub fn tasks(mut self, tasks: Vec<Task>) -> Self {
        self.tasks = tasks;
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

    fn proto(&self) -> UserPipelineConfig {
        let mut tasks: Vec<UserPipelineTaskConfig> = vec![];

        for task in &self.tasks {
            tasks.push(task.proto())
        }

        UserPipelineConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone().unwrap_or_default(),
            parallelism: self.parallelism,
            tasks,
        }
    }
}

pub fn pipeline_secret(key: &str) -> String {
    format!("pipeline_secret{{{key}}}")
}

pub fn global_secret(key: &str) -> String {
    format!("global_secret{{{key}}}")
}

pub fn pipeline_object(key: &str) -> String {
    format!("pipeline_object{{{key}}}")
}

pub fn run_object(key: &str) -> String {
    format!("run_object{{{key}}}")
}

#[derive(Debug)]
pub struct Task {
    pub id: String,
    pub description: Option<String>,
    pub image: String,
    pub registry_auth: Option<RegistryAuth>,
    pub depends_on: HashMap<String, RequiredParentStatus>,
    pub variables: HashMap<String, String>,
    pub entrypoint: Option<Vec<String>>,
    pub command: Option<Vec<String>>,
    pub inject_api_token: bool,
}

impl Task {
    pub fn new(id: &str, image: &str) -> Self {
        Task {
            id: id.to_string(),
            description: None,
            image: image.to_string(),
            registry_auth: None,
            depends_on: HashMap::new(),
            variables: HashMap::new(),
            entrypoint: None,
            command: None,
            inject_api_token: false,
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
            format!("GOFER_EXTENSION_PARAM_{}", key.to_uppercase()),
            value.to_string(),
        );
        self
    }

    pub fn variables(mut self, variables: HashMap<String, String>) -> Self {
        let variables: HashMap<String, String> = variables
            .iter()
            .map(|(key, value)| {
                (
                    format!("GOFER_EXTENSION_PARAM_{}", key.to_uppercase()),
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

    /// Gofer will auto-generate and inject a Gofer API token as `GOFER_API_TOKEN`. This allows you to easily have tasks
    /// communicate with Gofer by either embedding Gofer's CLI or just simply using the token to authenticate to the API.
    ///
    /// This auto-generated token is stored in this pipeline's secret store and automatically cleaned up when the run
    /// objects get cleaned up.
    pub fn inject_api_token(mut self, inject_token: bool) -> Self {
        self.inject_api_token = inject_token;
        self
    }

    fn proto(&self) -> UserPipelineTaskConfig {
        let mut depends_on: HashMap<String, i32> = HashMap::new();
        for (key, value) in &self.depends_on {
            let value = match value {
                RequiredParentStatus::Unknown => {
                    gofer_proto::user_pipeline_task_config::RequiredParentStatus::Unknown
                }
                RequiredParentStatus::Any => {
                    gofer_proto::user_pipeline_task_config::RequiredParentStatus::Any
                }
                RequiredParentStatus::Success => {
                    gofer_proto::user_pipeline_task_config::RequiredParentStatus::Success
                }
                RequiredParentStatus::Failure => {
                    gofer_proto::user_pipeline_task_config::RequiredParentStatus::Failure
                }
            };

            depends_on.insert(key.clone(), value.into());
        }

        UserPipelineTaskConfig {
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
            inject_api_token: self.inject_api_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_pipeline_cyclical() {
        let task_a = Task::new("task_a", "").depends_on("task_b", RequiredParentStatus::Any);
        let task_b = Task::new("task_b", "").depends_on("task_c", RequiredParentStatus::Any);
        let task_c = Task::new("task_c", "").depends_on("task_a", RequiredParentStatus::Any);

        let result = Pipeline::new("invalid_pipeline", "")
            .tasks(vec![task_a, task_b, task_c])
            .finish();

        assert_eq!(
            std::mem::discriminant(&result.unwrap_err()),
            std::mem::discriminant(&ConfigError::TaskCycle("".to_string(), "".to_string())),
        )
    }
}
