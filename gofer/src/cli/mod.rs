mod event;
mod extension;
mod fetch;
mod namespace;
mod pipeline;
mod role;
mod run;
mod secret;
mod service;
mod task;
mod token;
mod up;

use crate::conf::{cli::CliConfig, Configuration};
use anyhow::{bail, Context, Result};
use chrono::{LocalResult, TimeZone, Utc};
use chrono_humanize::HumanTime;
use clap::{Parser, Subcommand};
use colored::Colorize;
use lazy_regex::regex;
use polyfmt::println;
use reqwest::{header, Client};
use std::collections::HashMap;
use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};

/// Gofer is a distributed, continuous thing do-er.
///
/// It uses a similar model to [concourse](https://concourse-ci.org/), leveraging the docker container as a key
/// mechanism to run short-lived workloads. The benefits of this is simplicity. No foreign agents, no cluster setup,
/// just run containers.
///
/// For longer, more complete documentation visit: https://gofer.clintjedwards.com/docs
///
/// ## Configuration
/// This program retrieves it's settings from multiple sources in a specific sequence. First it loads default settings,
/// then it looks for a configuration file, then environment variables, and lastly CLI flags. Settings from later sources
/// will supersede identical settings from earlier ones.
///
/// ### Config file
///
/// Gofer will automatically attempt to load a configuration file from the following locations:
/// [~/.gofer.toml, ~/.config/gofer.toml]
///
/// Gofer will automatically create a configuration file for you at `~/.gofer.toml` with default settings
/// if it doesn't find one on command run. You will need to update the configuration file to include your Gofer API key
/// that has been shared with you.
///
/// ### Env vars
///
/// The environment variables that the program accepts are 1:1 with the config file keys it accepts. The key for the value
/// is specifically formatted in a particular way though. Firstly, all env vars have a prefix of 'GOFER_', and they
/// respect any nesting by using double underscores to differentiate when they're now in a new nested level.
///
/// Let's look at some examples:
///
/// The following config toml:
///
/// ```toml
///
/// log_level = 'debug'
///
/// [server]
/// admin_key = 'test'
///
/// ```
///
/// Would result in the corresponding environment variables:
///
/// GOFER_LOG_LEVEL = debug
///
/// GOFER_SERVER__ADMIN_KEY = test
#[derive(Debug, Parser, Clone)]
#[command(name = "gofer")]
#[command(bin_name = "gofer")]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, Clone)]
enum Commands {
    /// Register and deploy a new pipeline config.
    ///
    /// If pipeline does not exist this will create a new one.
    ///
    /// Requires a pipeline configuration file. You can find documentation on how to
    /// create/manage your pipeline configuration file
    /// [here](https://gofer.clintjedwards.com/docs/ref/pipeline_configuration/index.html).
    Up {
        path: std::path::PathBuf,

        /// Namespace Identifier.
        #[arg(long)]
        namespace: Option<String>,

        /// Performs a registration and deployment of the pipeline. If this is set to false, the pipeline config
        /// will be registered but not deployed.
        #[arg(short, long, default_value = "true")]
        deploy: bool,
    },

    /// Lookup pipeline specific information.
    ///
    /// Shortcut for commands like `pipeline get` or `run list`. Use `+` to
    /// differentiate between getting a specific item and listing the next tier of items.
    ///
    /// Ex. `gofer fetch my-pipeline`: Will return details for "my-pipeline"i
    ///
    /// Ex. `gofer fetch my-pipeline +`: Will return a list of all runs for "my-pipeline"
    Fetch {
        /// Namespace Identifier.
        #[arg(long)]
        namespace: Option<String>,

        /// Pipeline Identifier.
        pipeline_id: Option<String>,

        /// Run Identifier.
        run_id: Option<String>,

        /// Task Identifier.
        task_id: Option<String>,

        /// Limit the amount of results returned
        #[arg(short, long, default_value = "10")]
        limit: u64,

        /// How many runs to skip, useful for paging through results.
        #[arg(short, long, default_value = "0")]
        offset: u64,

        /// Reverse the return order back to ascending order. By default lists runs in descending order.
        #[arg(short, long, default_value = "false")]
        no_reverse: bool,
    },

    /// Prints information about the current context of the Gofer CLI.
    ///
    /// Helpful for things like figuring out which environment you're currently communicating with, the server status,
    /// CLI configuration options and more.
    Context,

    /// Manage the Gofer api service.
    Service(service::ServiceSubcommands),

    /// Manage namespaces.
    ///
    /// A namespace represents a grouping of pipelines. Normally it is used to divide teams or logically different
    /// sections of workloads. It is the highest level unit as it sits above pipelines in the hierarchy of Gofer.
    Namespace(namespace::NamespaceSubcommands),

    /// Manage pipelines.
    ///
    /// A pipeline is a graph of containers that accomplish some goal. Pipelines are
    /// created via a Pipeline configuration file and can be set to be run automatically via attached
    /// extensions
    Pipeline(pipeline::PipelineSubcommands),

    /// Manage runs.
    ///
    /// A run is a specific execution of a pipeline at a specific point in time. A run is made up of multiple tasks
    /// that all execute according to their dependency on each other.
    Run(run::RunSubcommands),

    /// Manage permissions and roles.
    ///
    /// A role is a group of permissions assigned to tokens to give user's access.
    Role(role::RoleSubcommands),

    /// Manage task executions.
    ///
    /// A task is the lowest unit of execution for a pipeline. A task execution is the
    /// tracking of a task, which is to say a task execution is simply the tracking of the container that
    /// is in the act of being executed.
    Task(task::TaskSubcommands),

    /// Manage secrets.
    ///
    /// Gofer allows user to enter secrets on both a global and pipeline scope. This
    /// is useful for workloads that need access to secret values and want a quick, convenient way to
    /// access those secrets. Global secrets are managed by admins and can grant pipelines access to secrets
    /// shared amongst many namespaces. Pipeline secrets on the other hand are only accessible from within
    /// that specific pipeline
    Secret(secret::SecretSubcommands),

    /// Manage gofer extensions.
    ///
    /// Extensions act as plugins for Gofer that can do a multitude of things.
    ///
    /// An example of a extension might be the simply the passing of time for the "interval" extension. A user will
    /// _subscribe_ to this extension in their pipeline configuration file and based on settings used in that file
    /// interval will alert Gofer when the user's intended interval of time has passed. This automatically then
    /// kicks off a new instance of a run for that specific pipeline.
    Extension(extension::ExtensionSubcommands),

    /// Get details about Gofer's event system.
    Event(event::EventSubcommands),

    /// Manage Gofer API Tokens.
    Token(token::TokenSubcommands),
}

#[derive(Debug, Clone)]
pub struct Cli {
    args: Args,
    conf: CliConfig,
    client: gofer_sdk::api::Client,
}

impl Cli {
    pub fn new() -> Result<Self> {
        let args = Args::parse();

        // Set configuration
        let conf = Configuration::<CliConfig>::load(None).unwrap();

        let client = new_api_client(&conf.api_base_url, &conf.token)
            .context("Could not initiate gofer api client")?;

        Ok(Cli { args, conf, client })
    }

    #[allow(dead_code)]
    pub fn init_formatter(&self, format: polyfmt::Format) -> Box<dyn polyfmt::Formatter> {
        let fmtter_options = polyfmt::Options {
            debug: self.conf.debug,
            padding: 1,
            ..Default::default()
        };

        polyfmt::new(format, fmtter_options)
    }

    pub async fn run(&mut self) -> Result<()> {
        match self.args.clone().command {
            Commands::Up {
                namespace,
                path,
                deploy,
            } => self.pipeline_create(namespace, path, deploy).await,
            Commands::Fetch {
                namespace,
                pipeline_id,
                run_id,
                task_id,
                limit,
                offset,
                no_reverse,
            } => {
                self.fetch(
                    namespace,
                    pipeline_id,
                    run_id,
                    task_id,
                    limit,
                    offset,
                    no_reverse,
                )
                .await
            }
            Commands::Context => self.get_context().await,
            Commands::Service(service) => self.handle_service_subcommands(service).await,
            Commands::Namespace(namespace) => self.handle_namespace_subcommands(namespace).await,
            Commands::Pipeline(pipeline) => self.handle_pipeline_subcommands(pipeline).await,
            Commands::Run(run) => self.handle_run_subcommands(run).await,
            Commands::Role(role) => self.handle_role_subcommands(role).await,
            Commands::Secret(secret) => self.handle_secret_subcommands(secret).await,
            Commands::Task(task) => self.handle_task_subcommands(task).await,
            Commands::Extension(extension) => self.handle_extension_subcommands(extension).await,
            Commands::Event(event) => self.handle_event_subcommands(event).await,
            Commands::Token(token) => self.handle_token_subcommands(token).await,
        }
    }

    /// Uses the 'detail' flag for the CLI to either print a friendly duration if detail = false
    /// or the exact timestamp if detail = true.
    /// Expects to be given unix milliseconds.
    pub fn format_time(&self, time: u64) -> Option<String> {
        if time == 0 {
            return None;
        };

        if self.conf.detail {
            Some(
                chrono::DateTime::from_timestamp_millis(time as i64)
                    .unwrap()
                    .to_rfc2822(),
            )
        } else {
            format_duration(time)
        }
    }

    pub async fn get_context(&self) -> Result<()> {
        let preferences = self
            .client
            .get_system_preferences()
            .await
            .unwrap()
            .into_inner();

        let current_token = self.client.whoami().await.unwrap().into_inner().token;

        println!("Whoami?");
        println!("  id: {}", current_token.id);
        println!("  user: {}", current_token.user);
        println!("  roles: {:#?}", current_token.roles);

        println!("CLI configuration:");
        println!("  {:#?}", self.conf);

        println!("Server status (version: {}):", self.client.api_version());
        println!("  {:#?}", preferences);

        Ok(())
    }
}

/// Return the current epoch time in milliseconds.
pub fn epoch_milli() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Transforms the given time into a humanized duration string from the current time.
///  or if time is not valid returns None.
/// (i.e. 'about an hour ago' )
/// (i.e. 'in about an hour')
fn format_duration(time: u64) -> Option<String> {
    if time == 0 {
        return None;
    }

    let time_diff = epoch_milli() as i64 - time as i64;
    let time_diff_duration = chrono::Duration::milliseconds(time_diff);

    if time_diff.is_positive() {
        Some(HumanTime::from(time_diff_duration).to_text_en(
            chrono_humanize::Accuracy::Rough,
            chrono_humanize::Tense::Past,
        ))
    } else {
        Some(HumanTime::from(time_diff_duration).to_text_en(
            chrono_humanize::Accuracy::Rough,
            chrono_humanize::Tense::Future,
        ))
    }
}

/// Creates a new HTTP client that is set up to talk to Gofer.
pub fn new_api_client(url: &str, token: &str) -> Result<gofer_sdk::api::Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        gofer_sdk::api::API_VERSION_HEADER,
        header::HeaderValue::from_str(&gofer_sdk::api::ApiVersion::V0.to_string())?,
    );

    let client = Client::builder().default_headers(headers).build()?;

    Ok(gofer_sdk::api::Client::new_with_client(url, client))
}

/// This is a bit of generic function to figure out what color to make state and status text specifically for use in
/// comfy table.
/// We typically have some state string that comes from an object and we want to make it pretty colors
/// for downstream users. Previously to do this we made a special function for each type but then
/// notices that most of the states are colored in the exact same ways, so maybe a function that simply
/// compares strings is what we need.
///
/// Additionally, we don't simply pass back the colorized string because our table generator does not
/// handle the padding of colorized strings well.
fn colorize_status_text_comfy<T: ToString>(input: T) -> comfy_table::Color {
    match input.to_string().to_ascii_lowercase().as_str() {
        "active" | "complete" | "successful" | "success" | "live" | "true" => {
            comfy_table::Color::Green
        }
        "disabled" | "pending" | "running" | "unreleased" => comfy_table::Color::Yellow,
        "failed" | "fail" | "deprecated" | "false" => comfy_table::Color::Red,
        "cancel" | "cancelled" => comfy_table::Color::AnsiValue(245),
        _ => comfy_table::Color::Reset,
    }
}

/// This is a bit of generic function to figure out what color to make state and status text.
/// We typically have some state string that comes from an object and we want to make it pretty colors
/// for downstream users. Previously to do this we made a special function for each type but then
/// notices that most of the states are colored in the exact same ways, so maybe a function that simply
/// compares strings is what we need.
///
/// Additionally, we don't simply pass back the colorized string because our table generator does not
/// handle the padding of colorized strings well.
fn colorize_status_text<T: ToString>(input: T) -> String {
    let input = input.to_string().to_ascii_lowercase();

    match input.as_str() {
        "active" | "complete" | "successful" | "success" | "live" | "true" => {
            input.green().to_string()
        }
        "disabled" | "pending" | "running" | "unreleased" => input.yellow().to_string(),
        "failed" | "fail" | "deprecated" | "false" => input.red().to_string(),
        "cancel" | "cancelled" => input.dimmed().to_string(),
        _ => input,
    }
}

/// Formats a duration between two epoch millis into a readable string
/// like "1 hour, 25 mins, 12 secs" or "12 secs, 4 ms"
fn duration(start: i64, end: i64) -> String {
    if start == 0 {
        return "0 secs".to_string();
    }

    let start_time = match Utc.timestamp_millis_opt(start) {
        LocalResult::Single(t) => t,
        _ => Utc::now(), // fallback to current time if no valid time.
    };

    let end_time = if end != 0 {
        match Utc.timestamp_millis_opt(end) {
            LocalResult::Single(t) => t,
            _ => Utc::now(),
        }
    } else {
        Utc::now()
    };

    let duration = end_time.signed_duration_since(start_time);
    let total_millis = duration.num_milliseconds();

    if total_millis <= 0 {
        return "0 secs".to_string();
    }

    let hours = duration.num_hours();
    let minutes = (duration.num_minutes() % 60).abs();
    let seconds = (duration.num_seconds() % 60).abs();
    let millis = (duration.num_milliseconds() % 1000).abs();

    let mut parts = vec![];

    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        ));
    }
    if minutes > 0 {
        parts.push(format!(
            "{} min{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!(
            "{} sec{}",
            seconds,
            if seconds == 1 { "" } else { "s" }
        ));
    }

    if hours == 0 && minutes == 0 && millis > 0 {
        parts.push(format!("{} ms", millis));
    }

    parts.join(", ")
}

fn dependencies(
    dependencies: &HashMap<String, gofer_sdk::api::types::RequiredParentStatus>,
) -> Vec<String> {
    let mut result = vec![];
    let mut any = vec![];
    let mut successful = vec![];
    let mut failure = vec![];

    for (name, state) in dependencies {
        match state {
            gofer_sdk::api::types::RequiredParentStatus::Unknown => {}
            gofer_sdk::api::types::RequiredParentStatus::Any => any.push(name.to_string()),
            gofer_sdk::api::types::RequiredParentStatus::Success => {
                successful.push(name.to_string())
            }
            gofer_sdk::api::types::RequiredParentStatus::Failure => failure.push(name.to_string()),
        }
    }

    if !any.is_empty() {
        if any.len() == 1 {
            result.push(format!("After task {} has finished.", any.first().unwrap()));
        } else {
            result.push(format!("After tasks {} have finished.", any.join(", ")));
        }
    }

    if !successful.is_empty() {
        if successful.len() == 1 {
            result.push(format!(
                "Only after task {} has finished successfully.",
                successful.first().unwrap()
            ));
        } else {
            result.push(format!(
                "Only after tasks {} have finished successfully.",
                successful.join(", ")
            ));
        }
    }

    if !failure.is_empty() {
        if failure.len() == 1 {
            result.push(format!(
                "Only after task {} has finished with an error.",
                failure.first().unwrap()
            ));
        } else {
            result.push(format!(
                "Only after tasks {} have finished with an error.",
                failure.join(", ")
            ));
        }
    }

    result
}

/// Identifiers are used as the primary key in most of gofer's resources.
/// They're defined by the user and therefore should have some sane bounds.
/// For all ids we'll want the following:
/// * 32 > characters < 3
/// * Only alphanumeric characters or hyphens
///
/// We don't allow underscores to conform with common practices for url safe strings.
fn validate_identifier(value: &str) -> Result<()> {
    let alphanumeric_w_hyphen = regex!("^[a-zA-Z0-9-]*$");

    if value.len() > 32 {
        bail!("length cannot be greater than 32")
    }

    if value.len() < 3 {
        bail!("length cannot be less than 3")
    }

    if !alphanumeric_w_hyphen.is_match(value) {
        bail!("can only be made up of alphanumeric and hyphen characters")
    }

    Ok(())
}

trait TitleCase {
    fn title(&self) -> String;
}

impl TitleCase for str {
    /// Convert string to title case.
    fn title(&self) -> String {
        self.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first_char) => {
                        first_char.to_uppercase().collect::<String>()
                            + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
