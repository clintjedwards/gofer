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
mod web;

use crate::conf::{
    Configuration,
    cli::{CliConfig, OutputFormat},
};
use anyhow::{Context, Result, bail};
use chrono::{LocalResult, TimeZone, Utc};
use chrono_humanize::HumanTime;
use clap::{Parser, Subcommand};
use colored::Colorize;
use gofer_sdk::api::ClientInfo;
use lazy_regex::regex;
use polyfmt::{finish, println};
use reqwest::{Client, header};
use std::collections::HashMap;
use std::{
    fmt::Debug,
    io::IsTerminal,
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
/// Settings are loaded from defaults, then a configuration file, then environment variables. Settings from later
/// sources replace the same settings from earlier ones.
///
/// ### Config file
///
/// Gofer reads the first configuration file it finds out of: [~/.gofer.toml, ~/.config/gofer.toml]
///
/// ```toml
/// api_base_url = 'https://gofer.example.com'
/// token = 'mysupersecrettoken'
/// namespace = 'default'
/// ```
///
/// ### Env vars
///
/// Every config file key can also be set as an environment variable by upper casing it and adding a 'GOFER_' prefix.
/// For example, the file above is the same as:
///
/// GOFER_API_BASE_URL = https://gofer.example.com
///
/// GOFER_TOKEN = mysupersecrettoken
///
/// GOFER_NAMESPACE = default
///
/// The full list of options is at https://gofer.clintjedwards.com/docs/cli/configuration.html
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

        /// Deploy the new version after registering it, making it the one new runs use. Pass `--deploy false` to
        /// only register it.
        #[arg(short, long, default_value_t = true, action = clap::ArgAction::Set)]
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

    /// Open the web UI, already signed in with your CLI token.
    ///
    /// With no arguments this opens the front page. Pass a pipeline to open its latest run, or a pipeline and run to
    /// open that run's details page.
    ///
    /// Ex. `gofer web dag 3`
    Web {
        /// Pipeline Identifier.
        pipeline_id: Option<String>,

        /// Run Identifier.
        run_id: Option<u64>,

        /// Namespace Identifier.
        #[arg(long)]
        namespace: Option<String>,

        /// Print a sign-in link instead of opening a browser. Handy over SSH or to use a different browser.
        #[arg(long, default_value = "false")]
        print: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Cli {
    args: Args,
    conf: CliConfig,
    client: gofer_sdk::api::Client,
}

// So we never forget to call [`polyfmt::Formatter::finish`]
impl Drop for Cli {
    fn drop(&mut self) {
        finish!();
    }
}

impl From<OutputFormat> for polyfmt::Format {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Spinner => polyfmt::Format::Spinner,
            OutputFormat::Plain => polyfmt::Format::Plain,
            OutputFormat::Silent => polyfmt::Format::Silent,
            OutputFormat::Json => polyfmt::Format::Json,
        }
    }
}

impl Cli {
    pub fn new() -> Result<Self> {
        let args = Args::parse();

        // Set configuration
        let conf = Configuration::<CliConfig>::load(None).unwrap();

        let client = new_api_client(&conf.api_base_url, &conf.token)
            .context("Could not initiate gofer api client")?;

        // Starting the service is just a stream of logs, so a spinner would only get in the way.
        let is_service_start = matches!(
            &args.command,
            Commands::Service(service::ServiceSubcommands {
                command: service::ServiceCommands::Start { .. }
            })
        );

        // Spinners only make sense on a terminal, when output is redirected we fall back to plain.
        let output_format = match conf.output_format {
            OutputFormat::Spinner if is_service_start || !std::io::stdout().is_terminal() => {
                polyfmt::Format::Plain
            }
            _ => polyfmt::Format::from(conf.output_format.clone()),
        };

        let fmtter_options = polyfmt::Options {
            debug: conf.debug,
            padding: 1,
            // Tables and templates are already laid out for the terminal, so we don't want polyfmt re-wrapping them.
            max_line_length: usize::MAX,
            ..Default::default()
        };

        polyfmt::set_global_formatter(polyfmt::new(output_format, fmtter_options));

        Ok(Cli { args, conf, client })
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
            Commands::Web {
                pipeline_id,
                run_id,
                namespace,
                print,
            } => self.web(namespace, pipeline_id, run_id, print).await,
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
            .context("Could not retrieve system preferences from Gofer api")?
            .into_inner();

        let current_token = self
            .client
            .whoami()
            .await
            .context("Could not retrieve current token from Gofer api")?
            .into_inner()
            .token;

        println!("Whoami?");
        println!("  id: {}", current_token.id);
        println!("  user: {}", current_token.user);
        println!("  roles: {:#?}", current_token.roles);

        println!("CLI configuration:");
        println!("  {:#?}", self.conf);

        println!(
            "Server status (version: {}):",
            gofer_sdk::api::Client::api_version()
        );
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

    // HumanTime calls very recent times "now" but still appends "ago"/"in" for the tense.
    let present = HumanTime::from(time_diff_duration).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Present,
    );
    if present == "now" {
        return Some("just now".into());
    }

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

    // Log and event streaming use websockets, which upgrade from an HTTP/1.1 request. Over HTTPS a proxy like Caddy
    // will happily negotiate HTTP/2, which has no way to carry that upgrade, so the server just sees a normal request.
    let client = Client::builder()
        .default_headers(headers)
        .http1_only()
        .build()?;

    // The generated client appends paths starting with "/", so a trailing slash would produce "//" in every URL.
    Ok(gofer_sdk::api::Client::new_with_client(
        url.trim_end_matches('/'),
        client,
    ))
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

/// The magenta bar that runs down the left side of each section in `get` output.
fn rail() -> String {
    "│".magenta().to_string()
}

/// Renders rows as a borderless table for a `$ Section:` in `get` output, returning one string per line with the rail
/// in front. The rail is added after rendering, instead of being its own column, so it continues down rows that wrap.
fn rail_table(header: &[&str], rows: Vec<Vec<comfy_table::Cell>>) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("{} None", rail())];
    }

    let mut table = comfy_table::Table::new();
    table
        .load_style(comfy_table::presets::NOTHING)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    if !header.is_empty() {
        table.set_header(
            header
                .iter()
                .map(|title| comfy_table::Cell::new(title).fg(comfy_table::Color::AnsiValue(245))),
        );
    }

    // Leave room for the indent and rail we put in front of each line so wrapped rows don't overflow.
    if let Some(width) = table.width() {
        table.set_width(width.saturating_sub(4));
    }

    for row in rows {
        table.add_row(row);
    }

    table
        .lines()
        .map(|line| format!("{}{line}", rail()))
        .collect()
}

/// Splits an event kind into its name and fields. Kinds serialize as either a bare name or a single-key object like
/// {"started_run": {..fields..}}, so going through JSON lets us show any kind without listing every variant.
fn event_kind_parts(kind: &gofer_sdk::api::types::Kind) -> Result<(String, Vec<(String, String)>)> {
    let parts = match serde_json::to_value(kind).context("Could not serialize event kind")? {
        serde_json::Value::Object(map) if map.len() == 1 => {
            let (name, inner) = map.into_iter().next().unwrap();
            let fields = match inner {
                serde_json::Value::Object(fields) => fields
                    .into_iter()
                    .map(|(key, value)| match value {
                        serde_json::Value::String(value) => (key, value),
                        value => (key, value.to_string()),
                    })
                    .collect(),
                _ => vec![],
            };
            (name, fields)
        }
        serde_json::Value::String(name) => (name, vec![]),
        other => (other.to_string(), vec![]),
    };

    Ok(parts)
}

/// A pipeline's tasks as a rail table, tasks with fewer dependencies first so it reads roughly in run order.
fn tasks_table(tasks: &HashMap<String, gofer_sdk::api::types::Task>) -> Vec<String> {
    let mut tasks: Vec<_> = tasks.values().collect();
    tasks.sort_by(|a, b| {
        a.depends_on
            .len()
            .cmp(&b.depends_on.len())
            .then_with(|| a.id.cmp(&b.id))
    });

    rail_table(
        &["TASK", "STARTS"],
        tasks
            .into_iter()
            .map(|task| {
                let depends_on = dependencies(&task.depends_on);
                let starts = if depends_on.is_empty() {
                    "Immediately".to_string()
                } else {
                    depends_on.join("\n")
                };
                vec![
                    comfy_table::Cell::new(&task.id).fg(comfy_table::Color::Blue),
                    comfy_table::Cell::new(starts),
                ]
            })
            .collect(),
    )
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

#[allow(dead_code)]
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
