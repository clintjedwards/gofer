use crate::{
    dag::{self, DagError},
    validate_identifier,
};
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

/// How a task's image is made. `gofer up` builds it on the user's machine and uploads it to Gofer, so a version of a
/// pipeline always runs exactly the image it was registered with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Build {
    /// Start from a base image, then copy files in and run commands, in the order they were written.
    Steps(Steps),

    /// Build from a Dockerfile, for anything the steps can't do. Both paths are relative to the folder of the
    /// pipeline's config.
    Dockerfile {
        path: String,

        /// The folder whose files the Dockerfile can use.
        context: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Steps {
    /// The image to start from, like `rust:1-trixie`.
    pub base: String,

    pub steps: Vec<Step>,

    /// Moves what it lists onto another base image once every step has run, leaving everything else behind.
    pub keep: Option<Keep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// Copies a file, or what's inside a folder, into the image. `from` is relative to the folder of the pipeline's
    /// config, and leaving it is fine, like `../shared`.
    Copy { from: String, to: String },

    /// Runs a command with `/bin/sh -c`.
    Run { command: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Keep {
    /// Full paths in the image. Each lands at the same path on the new base.
    pub paths: Vec<String>,

    /// The image to move them onto, like `debian:13-slim`. It needs whatever the kept files need to run, like the
    /// system libraries a program uses.
    pub base: String,
}

/// The image `gofer up` built for a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuiltImage {
    /// The digest of the image's manifest, like `sha256:4a5c...`.
    pub digest: String,

    /// A hash of the files that went into the image. It's how `gofer up` knows whether they changed since.
    pub source_hash: String,
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

    #[error(transparent)]
    Dag(#[from] DagError),

    /// Every problem found in a config, so they can all be fixed at once.
    #[error("pipeline config has problems:\n{}", .0.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    Invalid(Vec<ConfigError>),
}

/// Variable names become environment variable names in the task's container, which can't be empty or contain `=`
/// or NUL. Anything else is allowed so users can name them however their program expects.
pub fn validate_variable_key(arg: &str, key: &str) -> Result<(), ConfigError> {
    let description = if key.is_empty() {
        "cannot be empty"
    } else if key.contains('=') {
        "cannot contain '='"
    } else if key.contains('\0') {
        "cannot contain NUL characters"
    } else {
        return Ok(());
    };

    Err(ConfigError::InvalidArgument {
        argument: arg.to_string(),
        value: key.to_string(),
        description: description.to_string(),
    })
}

/// Splits an `allow_runs_from` pattern into its namespace and pipeline regexes, each anchored to match a whole id.
fn parse_run_source(pattern: &str) -> Option<(lazy_regex::Regex, lazy_regex::Regex)> {
    let (namespace, pipeline) = pattern.split_once('/')?;
    if namespace.is_empty() || pipeline.is_empty() {
        return None;
    }

    let namespace = lazy_regex::Regex::new(&format!("^(?:{namespace})$")).ok()?;
    let pipeline = lazy_regex::Regex::new(&format!("^(?:{pipeline})$")).ok()?;
    Some((namespace, pipeline))
}

fn validate_run_source(pattern: &str) -> Result<(), ConfigError> {
    if parse_run_source(pattern).is_some() {
        return Ok(());
    }

    Err(ConfigError::InvalidArgument {
        argument: "allow_runs_from".into(),
        value: pattern.into(),
        description: "must be 'namespace/pipeline' where both halves are non-empty regexes; use '.*' to match \
            everything"
            .into(),
    })
}

/// Whether an `allow_runs_from` pattern covers the given pipeline. Invalid patterns match nothing.
pub fn run_source_matches(pattern: &str, namespace_id: &str, pipeline_id: &str) -> bool {
    parse_run_source(pattern).is_some_and(|(namespace, pipeline)| {
        namespace.is_match(namespace_id) && pipeline.is_match(pipeline_id)
    })
}

/// In the same order as cron's day of week field.
const WEEKDAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// Reads a `MM/DD` date. There's no year, so 02/29 is always allowed.
fn parse_month_day(text: &str) -> Option<(u32, u32)> {
    let (month, day) = text.split_once('/')?;
    let two_digits = |text: &str| text.len() == 2 && text.bytes().all(|b| b.is_ascii_digit());
    if !two_digits(month) || !two_digits(day) {
        return None;
    }

    let month = month.parse().ok()?;
    let day = day.parse().ok()?;
    let days_in_month = match month {
        2 => 29,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return None,
    };

    (1..=days_in_month).contains(&day).then_some((month, day))
}

/// Splits a `skip_dates` entry into its first date, last date, and weekday. A single date is its own first and last.
#[allow(clippy::type_complexity)]
fn parse_skip_date(skip_date: &str) -> Option<((u32, u32), (u32, u32), Option<u32>)> {
    let (dates, weekday) = match skip_date.split_once(':') {
        Some((dates, weekday)) => {
            let weekday = WEEKDAYS.iter().position(|name| *name == weekday)?;
            (dates, Some(weekday as u32))
        }
        None => (skip_date, None),
    };

    let (first, last) = dates.split_once('-').unwrap_or((dates, dates));
    Some((parse_month_day(first)?, parse_month_day(last)?, weekday))
}

fn validate_skip_date(skip_date: &str) -> Result<(), ConfigError> {
    if parse_skip_date(skip_date).is_some() {
        return Ok(());
    }

    Err(ConfigError::InvalidArgument {
        argument: "skip_dates".into(),
        value: skip_date.into(),
        description:
            "must be 'MM/DD' or 'MM/DD-MM/DD', optionally followed by a weekday like ':thu'".into(),
    })
}

/// Whether a `skip_dates` entry covers the given day. `weekday` counts from Sunday, which is 0. Invalid entries match
/// nothing.
pub fn skip_date_matches(skip_date: &str, month: u32, day: u32, weekday: u32) -> bool {
    let Some((first, last, only_weekday)) = parse_skip_date(skip_date) else {
        return false;
    };

    let date = (month, day);
    let in_range = if first <= last {
        (first..=last).contains(&date)
    } else {
        // The range goes over the new year.
        date >= first || date <= last
    };

    in_range && only_weekday.is_none_or(|only| only == weekday)
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
///          Task::new("simple-task", "ubuntu:latest")
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

    /// Other pipelines whose runs may start runs of this one, as `namespace/pipeline` patterns. Each half is a regex
    /// that has to match the whole id, so `team-a-.*/.*` allows every pipeline in namespaces starting with `team-a-`.
    /// Defaults to empty so configs built with older SDKs still parse.
    #[serde(default)]
    pub allow_runs_from: Vec<String>,

    /// Days the pipeline doesn't run, as `MM/DD` or `MM/DD-MM/DD`. Either can be limited to a weekday, so
    /// `11/22-11/28:thu` is Thanksgiving. Dates repeat every year and are in UTC. Defaults to empty so configs built with
    /// older SDKs still parse.
    #[serde(default)]
    pub skip_dates: Vec<String>,

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
            allow_runs_from: Vec::new(),
            skip_dates: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// Checks the whole config and returns [`ConfigError::Invalid`] listing every problem found. The Gofer API runs
    /// this same check when a config is registered, so configs that skip the SDK are held to the same rules.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut problems = vec![];

        if let Err(e) = validate_identifier("id", &self.id) {
            problems.push(e);
        }

        if self.parallelism < 0 {
            problems.push(ConfigError::InvalidArgument {
                argument: "parallelism".into(),
                value: self.parallelism.to_string(),
                description: "cannot be negative; use 0 for the server's default".into(),
            });
        }

        for pattern in &self.allow_runs_from {
            if let Err(e) = validate_run_source(pattern) {
                problems.push(e);
            }
        }

        for skip_date in &self.skip_dates {
            if let Err(e) = validate_skip_date(skip_date) {
                problems.push(e);
            }
        }

        for task in &self.tasks {
            if let Err(e) = validate_identifier("task id", &task.id) {
                problems.push(e);
            }

            problems.extend(task.misused.iter().cloned());
            validate_build(&task.id, &task.build, &mut problems);

            for (parent, status) in &task.depends_on {
                if *status == RequiredParentStatus::Unknown {
                    problems.push(ConfigError::InvalidArgument {
                        argument: format!("depends_on for task '{}'", task.id),
                        value: parent.clone(),
                        description:
                            "required status can't be Unknown; use Any, Success, or Failure".into(),
                    });
                }
            }

            for key in task.variables.keys() {
                if let Err(e) =
                    validate_variable_key(&format!("variable name for task '{}'", task.id), key)
                {
                    problems.push(e);
                }
            }
        }

        let dag_errors = dag::validate(
            self.tasks
                .iter()
                .map(|task| (task.id.as_str(), task.depends_on.keys().map(String::as_str))),
        );
        problems.extend(dag_errors.into_iter().map(ConfigError::Dag));

        if problems.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Invalid(problems))
        }
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn parallelism(mut self, parallelism: i64) -> Self {
        self.parallelism = parallelism;
        self
    }

    /// Lets runs of other pipelines start runs of this one. Each entry is a `namespace/pipeline` pattern where both
    /// halves are regexes matched against the whole id. For example `team-a-.*/.*` allows any pipeline in a namespace
    /// starting with `team-a-`, and `deploys/release` allows just that one pipeline.
    ///
    /// Anyone who can start runs of an allowed pipeline can use its injected token to start this one, so only list
    /// pipelines whose users you'd trust to run this pipeline.
    pub fn allow_runs_from<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allow_runs_from = patterns.into_iter().map(Into::into).collect();
        self
    }

    /// Days the pipeline shouldn't run, as `MM/DD` or `MM/DD-MM/DD`. Ranges include both ends and can go over the new
    /// year. Add a weekday (`sun` to `sat`) to only skip that day of the range, so `11/22-11/28:thu` is Thanksgiving.
    ///
    /// Dates repeat every year and are in UTC. A run started on one of them is recorded as skipped, unless it's forced.
    pub fn skip_dates<I, S>(mut self, dates: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.skip_dates = dates.into_iter().map(Into::into).collect();
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

/// Checks what can be checked about a build without its files. `gofer up` finds the rest, like a path that doesn't
/// exist, when it builds.
fn validate_build(task_id: &str, build: &Build, problems: &mut Vec<ConfigError>) {
    let mut not_empty = |argument: &str, value: &str| {
        if value.trim().is_empty() {
            problems.push(ConfigError::InvalidArgument {
                argument: format!("{argument} for task '{task_id}'"),
                value: value.into(),
                description: "cannot be empty".into(),
            });
        }
    };

    match build {
        Build::Steps(steps) => {
            not_empty("base image", &steps.base);

            for step in &steps.steps {
                match step {
                    Step::Copy { from, to } => {
                        not_empty("copy source", from);
                        not_empty("copy destination", to);
                    }
                    Step::Run { command } => not_empty("run command", command),
                }
            }

            if let Some(keep) = &steps.keep {
                not_empty("keep base image", &keep.base);

                if keep.paths.is_empty() {
                    problems.push(ConfigError::InvalidArgument {
                        argument: format!("keep for task '{task_id}'"),
                        value: String::new(),
                        description: "needs at least one path to keep".into(),
                    });
                }

                // A relative path would be relative to whatever folder each image starts in, which can differ.
                for path in &keep.paths {
                    if !path.starts_with('/') {
                        problems.push(ConfigError::InvalidArgument {
                            argument: format!("keep path for task '{task_id}'"),
                            value: path.clone(),
                            description: "has to be a full path, starting with '/'".into(),
                        });
                    }
                }
            }
        }

        Build::Dockerfile { path, context } => {
            not_empty("Dockerfile path", path);
            not_empty("Dockerfile context", context);
        }
    }
}

/// Runs [`Task::script`]'s script with bash when the image has it and sh otherwise. The script is passed as `$1`
/// rather than pasted into this string so it never needs quoting.
const SCRIPT_LAUNCHER: &str = r#"exec "$(command -v bash || echo sh)" -c "$1""#;

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

/// Represents a single task within a [`Pipeline`]. A task is a unit of work that operates within its own container.
/// Each task defines the operations to be performed and the container environment in which these operations will run.
///
///   - The ID must be between 3 and 32 characters long and only alphanumeric, hyphens are the only allowed
///     alphanumeric character.
///     Ex. `simple-pipeline`
///   - The name is a human friendly name to represent the pipeline.
///     Ex. `Simple Pipeline`
///
/// Gofer builds every task's image itself, when you run `gofer up`. A task starts from a base image, copies in your
/// files and runs commands, or it points at a Dockerfile.
///
/// # Example
/// ```
/// # use gofer_sdk::config::Task;
/// // Compile a folder, then keep only the binary on a small base.
/// let report = Task::new("report", "rust:1-trixie")
///     .copy("./report", "/src")
///     .run("cd /src && cargo build --release")
///     .keep(["/src/target/release/report"], "debian:13-slim")
///     .command(vec!["/src/target/release/report".into()]);
///
/// // Anything the steps can't do is a Dockerfile. The last argument is the folder it builds from.
/// let scan = Task::dockerfile("scan", "./scan/Dockerfile", "./scan");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub id: String,
    pub description: Option<String>,

    /// How the task's image is made.
    pub build: Build,

    /// The image `gofer up` built. A config straight from the SDK doesn't have one; `gofer up` fills it in before it
    /// registers the config.
    #[serde(default)]
    pub image: Option<BuiltImage>,

    pub depends_on: HashMap<String, RequiredParentStatus>,
    pub variables: HashMap<String, String>,

    /// Replaces the image's entrypoint. It's built into the image.
    pub entrypoint: Option<Vec<String>>,

    /// Replaces the image's command. It's built into the image.
    pub command: Option<Vec<String>>,

    pub inject_api_token: bool,

    /// How long the task's container can run before Gofer stops it and fails the task. 0 means the server's max
    /// task execution timeout applies. Defaults to 0 so configs built with older SDKs still parse.
    #[serde(default)]
    pub timeout_seconds: u64,

    /// Builder methods that were used where they can't work, like [`Task::copy`] on a Dockerfile task. They can't
    /// return an error without breaking the chain, so [`Pipeline::validate`] reports these.
    #[serde(skip)]
    misused: Vec<ConfigError>,
}

impl Task {
    /// A task whose image starts from `base`, like `rust:1-trixie` or `python:3`. Add to it with [`Task::copy`],
    /// [`Task::run`] and [`Task::keep`], or leave it as is to run the base image with a command of your own.
    pub fn new(id: &str, base: &str) -> Self {
        Self::with_build(
            id,
            Build::Steps(Steps {
                base: base.to_string(),
                steps: vec![],
                keep: None,
            }),
        )
    }

    /// A task whose image is built from a Dockerfile, for anything [`Task::copy`] and [`Task::run`] can't do. Both
    /// paths are relative to the folder of the pipeline's config. The context is the folder the Dockerfile can use
    /// files from, the same as the one you'd pass to `docker build`, and its `.dockerignore` is used the same way.
    pub fn dockerfile(id: &str, path: &str, context: &str) -> Self {
        Self::with_build(
            id,
            Build::Dockerfile {
                path: path.to_string(),
                context: context.to_string(),
            },
        )
    }

    fn with_build(id: &str, build: Build) -> Self {
        Task {
            id: id.to_string(),
            description: None,
            build,
            image: None,
            depends_on: HashMap::new(),
            variables: HashMap::new(),
            entrypoint: None,
            command: None,
            inject_api_token: false,
            timeout_seconds: 0,
            misused: vec![],
        }
    }

    /// Copies a file, or what's inside a folder, into the image at `to`. `from` is relative to the folder of the
    /// pipeline's config, and leaving it is fine, like `../shared`.
    ///
    /// The folder's `.dockerignore` decides what to leave out. Folders that only hold a cache, like Cargo's `target/`,
    /// are left out too, so copying a project doesn't copy what a local build left behind.
    pub fn copy(self, from: &str, to: &str) -> Self {
        self.add_step(
            "copy",
            Step::Copy {
                from: from.to_string(),
                to: to.to_string(),
            },
        )
    }

    /// Runs a command in the image with `/bin/sh -c`, after the steps before it. Gofer keeps caches for Cargo and Go
    /// between builds on its own, so a plain `cargo build --release` is only slow the first time.
    pub fn run(self, command: &str) -> Self {
        self.add_step(
            "run",
            Step::Run {
                command: command.to_string(),
            },
        )
    }

    /// Once every step has run, moves only `paths` onto `base` and leaves everything else behind, including the
    /// compiler. A smaller image uploads and starts faster. Paths have to be full paths, and each one lands at the
    /// same path on the new base.
    ///
    /// Nothing else carries over from the first base: no environment variables, working directory or entrypoint.
    /// The new base needs whatever the kept files need to run; `rust:1-trixie` builds programs that run on
    /// `debian:13-slim`, for example. Calling it again replaces what was kept before.
    pub fn keep<I, S>(mut self, paths: I, base: &str) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let keep = Keep {
            paths: paths.into_iter().map(Into::into).collect(),
            base: base.to_string(),
        };

        match &mut self.build {
            Build::Steps(steps) => steps.keep = Some(keep),
            Build::Dockerfile { .. } => self.misuse("keep", &keep.paths.join(", ")),
        }
        self
    }

    fn add_step(mut self, name: &str, step: Step) -> Self {
        match &mut self.build {
            Build::Steps(steps) => steps.steps.push(step),
            Build::Dockerfile { .. } => {
                let value = match &step {
                    Step::Copy { from, .. } => from.clone(),
                    Step::Run { command } => command.clone(),
                };
                self.misuse(name, &value);
            }
        }
        self
    }

    fn misuse(&mut self, name: &str, value: &str) {
        self.misused.push(ConfigError::InvalidArgument {
            argument: format!("{name} for task '{}'", self.id),
            value: value.to_string(),
            description:
                "can't be used on a task built from a Dockerfile; do it in the Dockerfile instead"
                    .into(),
        });
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

    /// Sets environment variables to be injected into the task container at runtime.
    ///
    /// The provided variables will be added to the task's environment, with their keys automatically
    /// converted to uppercase. This ensures that the environment variables follow common conventions.
    ///
    /// You can use helper functions such as [`pipeline_secret`], [`global_secret`] and [`pipeline_object`] to insert
    /// dynamically generated secrets or object strings into the values of these variables. These helper functions
    /// ensure that your secrets and objects are correctly formatted for retrieval within the task.
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

    /// Replaces the image's entrypoint, the same way `docker run --entrypoint` would, so the image's command goes
    /// too unless you set one. It's built into the image.
    /// https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
    pub fn entrypoint(mut self, entrypoint: Vec<String>) -> Self {
        self.entrypoint = Some(entrypoint);
        self
    }

    /// Replaces the image's command (CMD). It's built into the image.
    /// https://docs.docker.com/engine/reference/builder/#understand-how-cmd-and-entrypoint-interact
    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    /// Provide an inline multi-line shell script to be run in the container.
    ///
    /// The script is run with `bash -c "<script>"` if the image has bash, and `sh -c "<script>"` if it doesn't, so
    /// it works on small images like alpine. If you use bash features, make sure your image has bash. Leading and
    /// trailing whitespace is trimmed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gofer_sdk::config::Task;
    /// let task = Task::new("run-cargo-test", "ghcr.io/clintjedwards/gofer/tools:rust")
    ///     .description("Run cargo test command for workspace")
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
    /// Should not be used with [`Task::command`].
    pub fn script(mut self, script: impl AsRef<str>) -> Self {
        let trimmed_script = script.as_ref().trim();

        self.command = Some(vec![
            "sh".into(),
            "-c".into(),
            SCRIPT_LAUNCHER.into(),
            "gofer-script".into(),
            trimmed_script.into(),
        ]);
        self
    }

    /// Gofer will auto-generate and inject a short-lived Gofer API token as `GOFER_TOKEN`. This allows you to
    /// easily have tasks communicate with Gofer by either embedding Gofer's CLI or just simply using the token to
    /// authenticate to the REST API.
    ///
    /// The token belongs to the run, and Gofer deletes it when the run ends.
    pub fn inject_api_token(mut self, inject_token: bool) -> Self {
        self.inject_api_token = inject_token;
        self
    }

    /// How long the task's container can run before Gofer stops it and marks the task as failed. Time spent loading
    /// the image doesn't count. Without a timeout, or with one longer than the server allows, the server's max task
    /// execution timeout applies instead.
    ///
    /// Timeouts are in whole seconds; anything shorter is rounded up.
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout_seconds = timeout.as_millis().div_ceil(1000) as u64;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_source_patterns_match_whole_ids() {
        assert!(run_source_matches(
            "team-a-.*/.*",
            "team-a-build",
            "anything"
        ));
        assert!(run_source_matches("deploys/release", "deploys", "release"));
        assert!(!run_source_matches(
            "deploys/release",
            "deploys",
            "release-two"
        ));
        assert!(!run_source_matches("team-a/.*", "not-team-a", "build"));
        assert!(!run_source_matches("team-a", "team-a", "build"));
    }

    #[test]
    fn invalid_run_sources_are_rejected() {
        for pattern in ["no-slash", "/build", "team-a/", "team-a/(", ""] {
            let result = Pipeline::new("valid", "")
                .allow_runs_from([pattern])
                .validate();
            assert!(result.is_err(), "expected '{pattern}' to be rejected");
        }

        assert!(
            Pipeline::new("valid", "")
                .allow_runs_from(["team-a-.*/.*"])
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn skip_dates_cover_the_right_days() {
        // Weekday 3 is a Wednesday.
        assert!(skip_date_matches("12/25", 12, 25, 3));
        assert!(!skip_date_matches("12/25", 12, 26, 3));
        assert!(skip_date_matches("02/29", 2, 29, 3));

        // Ranges include both ends.
        assert!(skip_date_matches("03/30-04/02", 3, 30, 3));
        assert!(skip_date_matches("03/30-04/02", 4, 1, 3));
        assert!(skip_date_matches("03/30-04/02", 4, 2, 3));
        assert!(!skip_date_matches("03/30-04/02", 3, 29, 3));
        assert!(!skip_date_matches("03/30-04/02", 4, 3, 3));

        // Ranges can go over the new year.
        assert!(skip_date_matches("12/24-01/02", 12, 24, 3));
        assert!(skip_date_matches("12/24-01/02", 12, 31, 3));
        assert!(skip_date_matches("12/24-01/02", 1, 2, 3));
        assert!(!skip_date_matches("12/24-01/02", 1, 3, 3));
        assert!(!skip_date_matches("12/24-01/02", 12, 23, 3));

        // Both the date and the weekday have to match.
        assert!(skip_date_matches("11/22-11/28:thu", 11, 26, 4));
        assert!(!skip_date_matches("11/22-11/28:thu", 11, 25, 3));
        assert!(!skip_date_matches("11/22-11/28:thu", 11, 19, 4));
        assert!(skip_date_matches("07/05:mon", 7, 5, 1));
        assert!(!skip_date_matches("07/05:mon", 7, 5, 2));
        assert!(skip_date_matches("01/01-12/31:sun", 6, 7, 0));
    }

    #[test]
    fn invalid_skip_dates_are_rejected() {
        for skip_date in [
            "",
            "12",
            "12/",
            "1/1",
            "13/01",
            "00/10",
            "12/00",
            "12/32",
            "02/30",
            "04/31",
            "+1/01",
            "2026/12/25",
            "12/25-",
            "12/25-13/01",
            "12/24-12/25-12/26",
            "12/25:",
            "12/25:thursday",
            "12/25:thu:fri",
            "12/25 ",
        ] {
            let result = Pipeline::new("valid", "")
                .skip_dates([skip_date])
                .validate();
            assert!(result.is_err(), "expected '{skip_date}' to be rejected");
            assert!(!skip_date_matches(skip_date, 12, 25, 4));
        }

        assert!(
            Pipeline::new("valid", "")
                .skip_dates(["12/25", "02/29", "12/24-01/02", "11/22-11/28:thu"])
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn test_invalid_pipeline_cyclical() {
        let task_a = Task::new("task-a", "ubuntu").depends_on("task-b", RequiredParentStatus::Any);
        let task_b = Task::new("task-b", "ubuntu").depends_on("task-c", RequiredParentStatus::Any);
        let task_c = Task::new("task-c", "ubuntu").depends_on("task-a", RequiredParentStatus::Any);

        let result = Pipeline::new("invalid-pipeline", "")
            .tasks(vec![task_a, task_b, task_c])
            .validate();

        assert_eq!(
            result,
            Err(ConfigError::Invalid(vec![ConfigError::Dag(
                DagError::Cycle(vec![
                    "task-a".into(),
                    "task-b".into(),
                    "task-c".into(),
                    "task-a".into()
                ])
            )]))
        )
    }

    /// The Go SDK writes the same JSON, and the server reads both, so the shape can't change by accident.
    #[test]
    fn test_builds_serialize_like_the_go_sdk() {
        let task = Task::new("report", "rust:1")
            .copy("./report", "/src")
            .run("cargo build --release")
            .keep(["/src/target/release/report"], "debian:13-slim");

        assert_eq!(
            serde_json::to_value(&task.build).unwrap(),
            serde_json::json!({"steps": {
                "base": "rust:1",
                "steps": [
                    {"copy": {"from": "./report", "to": "/src"}},
                    {"run": {"command": "cargo build --release"}},
                ],
                "keep": {"paths": ["/src/target/release/report"], "base": "debian:13-slim"},
            }})
        );

        assert_eq!(
            serde_json::to_value(Task::dockerfile("scan", "scan/Dockerfile", "scan").build)
                .unwrap(),
            serde_json::json!({"dockerfile": {"path": "scan/Dockerfile", "context": "scan"}})
        );

        // A config straight from the SDK has no image yet, and the problems found while building it aren't part of it.
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["image"], serde_json::Value::Null);
        assert!(json.get("misused").is_none());
    }

    #[test]
    fn test_steps_cant_be_used_on_dockerfile_tasks() {
        let task = Task::dockerfile("scan", "Dockerfile", ".")
            .copy("./app", "/app")
            .run("make")
            .keep(["/app"], "alpine:3");

        let result = Pipeline::new("pipeline", "").tasks(vec![task]).validate();
        let Err(ConfigError::Invalid(problems)) = result else {
            panic!("expected an invalid config, got {result:?}");
        };
        assert_eq!(problems.len(), 3, "{problems:#?}");
        assert!(problems[0].to_string().contains("built from a Dockerfile"));
    }

    #[test]
    fn test_kept_paths_have_to_be_full_paths() {
        let task = Task::new("report", "rust:1").keep(["target/release/report"], "debian:13-slim");

        let result = Pipeline::new("pipeline", "").tasks(vec![task]).validate();
        let Err(ConfigError::Invalid(problems)) = result else {
            panic!("expected an invalid config, got {result:?}");
        };
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].to_string().contains("full path"));
    }

    #[test]
    fn test_invalid_pipeline_reports_every_problem() {
        let task = Task::new("ok", "")
            .depends_on("missing", RequiredParentStatus::Unknown)
            .variables([("BAD=NAME", "value")]);

        let result = Pipeline::new("x", "")
            .parallelism(-1)
            .tasks(vec![task])
            .validate();

        let Err(ConfigError::Invalid(problems)) = result else {
            panic!("expected an invalid config, got {result:?}");
        };

        // Bad id, negative parallelism, bad task id, empty image, unknown status, bad variable, missing dependency.
        assert_eq!(problems.len(), 7, "{problems:#?}");
    }
}
