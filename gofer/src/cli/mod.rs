mod event;
mod extension;
mod namespace;
mod pipeline;
mod run;
mod secret;
mod service;
mod task;
mod token;
mod up;

use crate::conf::{cli::CliConfig, Configuration};
use anyhow::{bail, Context, Result};
use chrono::{Duration, LocalResult, TimeZone, Utc};
use chrono_humanize::HumanTime;
use clap::{Parser, Subcommand};
use colored::Colorize;
use lazy_regex::regex;
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
/// For longer, more complete documentation visit: https://clintjedwards.com/gofer
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
    /// [here](https://clintjedwards.com/gofer/ref/pipeline_configuration/index.html).
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
            Commands::Service(service) => self.handle_service_subcommands(service).await,
            Commands::Namespace(namespace) => self.handle_namespace_subcommands(namespace).await,
            Commands::Pipeline(pipeline) => self.handle_pipeline_subcommands(pipeline).await,
            Commands::Run(run) => self.handle_run_subcommands(run).await,
            Commands::Secret(secret) => self.handle_secret_subcommands(secret).await,
            Commands::Task(task) => self.handle_task_subcommands(task).await,
            Commands::Extension(extension) => self.handle_extension_subcommands(extension).await,
            Commands::Event(event) => self.handle_event_subcommands(event).await,
            Commands::Token(token) => self.handle_token_subcommands(token).await,
        }
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
fn humanize_relative_duration(time: u64) -> Option<String> {
    if time == 0 {
        return None;
    }

    let time_diff = epoch_milli() - time;
    let time_diff_duration = chrono::Duration::milliseconds(-(time_diff as i64));
    Some(HumanTime::from(time_diff_duration).to_string())
}

/// Transforms the given time into a humanized duration string from the current time.
///  or if time is not valid returns None.
/// (i.e. 'in an hour ago' )
fn humanize_future_time(time: u64) -> Option<String> {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let duration_until_expires = time.saturating_sub(now);
    let chrono_duration = chrono::Duration::milliseconds(duration_until_expires as i64);
    let duration_string = HumanTime::from(chrono_duration).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Future,
    );

    Some(duration_string)
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
        gofer_sdk::api::ApiVersion::V0.to_header_value()?,
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

/// Duration returns a humanized duration time for two epoch milli second times.
fn duration(start: i64, end: i64) -> String {
    if start == 0 {
        return "0s".to_string();
    }

    let start_time = match Utc.timestamp_millis_opt(start) {
        LocalResult::Single(time) => time,
        _ => Utc::now(), // fallback to current time if no valid time
    };
    let mut end_time = Utc::now();

    if end != 0 {
        end_time = match Utc.timestamp_millis_opt(end) {
            LocalResult::Single(time) => time,
            _ => Utc::now(), // fallback to current time if no valid time
        };
    }

    let duration = end_time.signed_duration_since(start_time);

    if duration > Duration::seconds(1) {
        return format!("~{}s", duration.num_seconds());
    }

    format!("~{}ms", duration.num_milliseconds())
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
/// * Only alphanumeric characters or underscores
fn validate_identifier(value: &str) -> Result<()> {
    let alphanumeric_w_underscores = regex!("^[a-zA-Z0-9_]*$");

    if value.len() > 32 {
        bail!("length cannot be greater than 32")
    }

    if value.len() < 3 {
        bail!("length cannot be less than 3")
    }

    if !alphanumeric_w_underscores.is_match(value) {
        bail!("can only be made up of alphanumeric and underscore characters")
    }

    Ok(())
}
