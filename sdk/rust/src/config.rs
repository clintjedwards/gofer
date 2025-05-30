use crate::{dag::DAGError, dag::Dag, validate_identifier};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write};
use strum::{Display, EnumString};

#[derive(Debug, Display, EnumString, PartialEq, Eq, Clone, Deserialize, Serialize, JsonSchema)]
pub enum RequiredParentStatus {
    Unknown,
    Any,
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
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

/// `Pipeline` represents a sequence of tasks, where each task is a discrete unit of work encapsulated within a container.
/// This structure allows you to organize and define the workflow for the tasks you want to execute.
///   - The ID must be between 3 and 32 characters long and only alphanumeric, hyphens are the only allowed
///     alphanumeric character.
///     Ex. `simple-pipeline`
///   - The name is a human friendly name to represent the pipeline.
///     Ex. `Simple Pipeline`
///
/// # Example
///
/// The following example demonstrates how to create a simple pipeline in Gofer, which is familiar to those experienced with CI/CD tooling.
/// It outlines how to define a simple task within a pipeline, use a standard Ubuntu container, and execute a basic command.
///
/// This simple example serves as a foundation, illustrating the pattern of defining tasks as building blocks of a pipeline.
/// In practice, you would create custom containers designed specifically for the tasks in your Gofer workflows,
/// keeping your pipeline configuration clean and focused on orchestration rather than embedding complex logic.
///
/// ```ignore
///  // Create a new pipeline with a name and a descriptive label.
///  Pipeline::new("simple", "Simple Pipeline")
///      .description("This pipeline demonstrates a simple Gofer pipeline that pulls in a container and runs a command. \
///                    This pattern will be familiar to those experienced with CI/CD tools. \
///                    Tasks in this pipeline are individual containers that can depend on other tasks, illustrating the modular nature of Gofer.")
///      // Adding a single task to the pipeline.
///      .tasks(vec![
///          Task::new("simple_task", "ubuntu:latest")
///              .description("This task uses the Ubuntu container to print a 'Hello World' message.")
///              .command(vec!["echo".to_string(), "Hello from Gofer!".to_string()])
///      ])
///      .finish() // Finalize and validate the pipeline setup.
///      .unwrap(); // Handle potential errors during pipeline creation.
/// ```
#[must_use = "complete pipeline config with the .finish() method"]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

        let json_str = serde_json::to_string(&self).map_err(|e| {
            ConfigError::Parsing(format!(
                "Could not successfully serialize pipeline; {:#?}",
                e
            ))
        })?;

        write!(std::io::stdout(), "{json_str}").map_err(|e| {
            ConfigError::Unknown(format!(
                "Could not successfully write out serialized pipeline; {:#?}",
                e
            ))
        })?;
        std::io::stdout().flush().unwrap();

        Ok(())
    }
}

pub fn pipeline_secret(key: &str) -> String {
    format!("pipeline_secret{{{{{key}}}}}")
}

pub fn global_secret(key: &str) -> String {
    format!("global_secret{{{{{key}}}}}")
}

/// A convenience function for retrieving objects from the pipeline object store.
///
/// Pipeline objects are part of a ring buffer that pushes out the oldest pipeline object once it becomes full. This
/// means that pipeline objects are kept forever until they are too many of them.
///
/// When pulling objects from the object store Gofer will attempt to stringify the object (utf-8).
/// If you need the raw bytes for an object use the Gofer cli.
pub fn pipeline_object(key: &str) -> String {
    format!("pipeline_object{{{{{key}}}}}")
}

/// A convenience function for retrieving objects from the run object store.
///
/// Run objects are scoped to a specific run. It is meant as a way for a task within a run to pass data to other tasks.
///
/// When pulling objects from the object store Gofer will attempt to stringify the object (utf-8).
/// If you need the raw bytes for an object use the Gofer cli.
pub fn run_object(key: &str) -> String {
    format!("run_object{{{{{key}}}}}")
}

/// Represents a single task within a [`Pipeline`]. A task is a unit of work that operates within its own container.
/// Each task defines the operations to be performed and the container environment in which these operations will run.
///
///   - The ID must be between 3 and 32 characters long and only alphanumeric, hyphens are the only allowed
///     alphanumeric character.
///     Ex. `simple-pipeline`
///   - The name is a human friendly name to represent the pipeline.
///     Ex. `Simple Pipeline`
///
/// # Example Usage
/// ```ignore
/// // Define a new task within a pipeline.
/// let task = Task {
///     id: "example_task".to_string(),
///     description: Some("This task executes a simple print command in an Ubuntu container.".to_string()),
///     image: "ubuntu:latest".to_string(),
///     registry_auth: None,
///     depends_on: HashMap::new(), // No dependencies, so it starts immediately when the pipeline runs.
///     variables: HashMap::from([("KEY", "value".to_string())]),
///     entrypoint: None, // Use the image's default entrypoint.
///     command: Some(vec!["echo".to_string(), "Hello World!".to_string()]),
///     inject_api_token: false,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

    /// Always attempt to pull the newest container image.
    pub always_pull_newest_image: bool,
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
            always_pull_newest_image: true,
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

    /// Sets environment variables to be injected into the task container at runtime.
    ///
    /// The provided variables will be added to the task's environment, with their keys automatically
    /// converted to uppercase. This ensures that the environment variables follow common conventions.
    ///
    /// You can use helper functions such as [`pipeline_secret`], [`global_secret`], [`pipeline_object`],
    /// and [`run_object`] to insert dynamically generated secrets or object strings into the values
    /// of these variables. These helper functions ensure that your secrets and objects are correctly
    /// formatted for retrieval within the task.
    ///
    /// # Examples
    ///
    /// ## Using a `Vec` of Tuples
    ///
    /// ```
    /// # use gofer_sdk::config::Task;
    /// # use std::collections::HashMap;
    /// let task = Task::new("example-task", "ghcr.io/example/image:latest")
    ///     .variables(vec![
    ///         ("WAIT_DURATION", "20s"),
    ///         ("RETRY_COUNT", "3"),
    ///     ]);
    /// ```
    ///
    /// ## Using a `HashMap`
    ///
    /// ```
    /// # use gofer_sdk::config::{Task, pipeline_object};
    /// # use std::collections::HashMap;
    /// let mut env_vars = HashMap::new();
    /// env_vars.insert("API_KEY", "12345");
    /// env_vars.insert("TIMEOUT", "30s");
    ///
    /// let task = Task::new("example-task", "ghcr.io/example/image:latest")
    ///     .variables(env_vars);
    ///
    /// let task = Task::new("example-task", "ghcr.io/example/image:latest")
    ///        .variables(HashMap::from([
    ///             ("SOME_VARIABLE", "something here"),
    ///             ("LOGS_HEADER", &pipeline_object("logs_header")),
    ///             ("ALTERNATE_LOGS_HEADER", "pipeline_object{{alternate_logs_header}}")
    ///            ])
    ///        );
    /// ```
    ///
    /// ## Getting secrets and objects dynamically
    ///
    /// ```
    /// # use gofer_sdk::config::{Task, pipeline_secret, pipeline_object};
    /// # use std::collections::HashMap;
    /// let mut env_vars = HashMap::new();
    /// env_vars.insert("API_KEY", pipeline_secret("some_secret_key"));
    /// env_vars.insert("TIMEOUT", pipeline_object("some_object_key"));
    ///
    /// let task = Task::new("example-task", "ghcr.io/example/image:latest")
    ///     .variables(env_vars);
    /// ```
    ///
    pub fn variables<K, V, I>(mut self, variables: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        let vars: HashMap<String, String> = variables
            .into_iter()
            .map(|(key, value)| (key.into().to_uppercase(), value.into()))
            .collect();

        self.variables.extend(vars);
        self
    }

    /// Set/Replace the Entrypoint of the task container.
    /// https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
    pub fn entrypoint(mut self, entrypoint: Vec<String>) -> Self {
        self.entrypoint = Some(entrypoint);
        self
    }

    /// Set/Replace the command(CMD) of the task container.
    /// https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    /// Provide an inline multi-line shell script to be run in the container as `bash -c "<script>"`.
    ///
    /// The script will be trimmed of leading and trailing whitespace. Under the hood, it
    /// becomes the equivalent of:
    ///
    /// ```bash
    /// sh -c "<your multiline script here>"
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// # use gofer_sdk::config::Task;
    /// let task = Task::new("run-cargo-test", "ghcr.io/clintjedwards/gofer/tools:rust")
    ///     .description("Run cargo test command for workspace")
    ///     .always_pull_newest_image(true)
    ///     .script(r#"
    ///         cargo test
    ///         wget https://example.com/somefile
    ///         curl https://google.com
    ///     "#);
    /// ```
    ///
    /// In this example, all three commands (cargo test, wget, and curl) will run
    /// sequentially inside a single container session.
    ///
    /// Should not be used with ['command'].
    pub fn script(mut self, script: impl AsRef<str>) -> Self {
        let trimmed_script = script.as_ref().trim();

        self.command = Some(vec!["bash".into(), "-c".into(), trimmed_script.into()]);
        self
    }

    /// Gofer will auto-generate and inject a short-lived Gofer API token as `GOFER_API_TOKEN`. This allows you to
    /// easily have tasks communicate with Gofer by either embedding Gofer's CLI or just simply using the token to
    /// authenticate to the REST API.
    ///
    /// This auto-generated token is stored in this pipeline's secret store and automatically cleaned up when the run
    /// objects get cleaned up.
    pub fn inject_api_token(mut self, inject_token: bool) -> Self {
        self.inject_api_token = inject_token;
        self
    }

    /// Always attempt to pull the newest container image for a given tag.
    pub fn always_pull_newest_image(mut self, pull: bool) -> Self {
        self.always_pull_newest_image = pull;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_pipeline_cyclical() {
        let task_a = Task::new("task-a", "").depends_on("task-b", RequiredParentStatus::Any);
        let task_b = Task::new("task-b", "").depends_on("task-c", RequiredParentStatus::Any);
        let task_c = Task::new("task-c", "").depends_on("task-a", RequiredParentStatus::Any);

        let result = Pipeline::new("invalid-pipeline", "")
            .tasks(vec![task_a, task_b, task_c])
            .finish();

        assert_eq!(
            std::mem::discriminant(&result.unwrap_err()),
            std::mem::discriminant(&ConfigError::TaskCycle("".to_string(), "".to_string())),
        )
    }
}
