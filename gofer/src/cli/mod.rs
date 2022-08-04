mod event;
mod namespace;
mod pipeline;
mod run;
mod service;
mod spinner;
mod trigger;
mod utils;

pub use self::spinner::*;
pub use self::utils::*;

use crate::conf::{self, cli::Config};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use clap::{Parser, Subcommand};
use gofer_proto::gofer_client::GoferClient;
use slog::o;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;
use std::{
    error::Error,
    fmt::Debug,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tonic::transport::channel::Channel;

#[derive(Debug, Parser)]
#[clap(name = "gofer")]
#[clap(about = "Gofer is a distributed, continuous thing do-er.")]
#[clap(
    long_about = "Gofer is a distributed, continous thing do-er.\n\n It uses a similar model to concourse
    (https://concourse-ci.org/), leveraging the docker container as a key mechanism to run short-lived workloads.
    This results in simplicity; No foreign agents, no cluster setup, just run containers.\n\n
    Read more at https://clintjedwards.com/gofer"
)]
#[clap(version)]
struct Cli {
    /// Set configuration path; if empty default paths are used
    #[clap(long, value_name = "PATH")]
    config_path: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

const DEFAULT_NAMESPACE: &str = "default";

struct CliHarness {
    config: Config,
}

impl CliHarness {
    /// Allows for injecting the default namespace into the configuration,
    /// so that there is still a single place to look up CLI state but we
    /// properly overwrite what the default namespace should be.
    /// Flags always get final priority over all other configuration types.
    fn default_namespace(&mut self, namespace: &str) {
        self.config.namespace = Some(namespace.to_string());
    }

    async fn connect(&self) -> Result<GoferClient<Channel>, Box<dyn Error>> {
        let tls_config = get_tls_config(&self.config.server, self.config.tls_ca.clone())?;

        let channel = Channel::from_shared(self.config.server.to_string())?
            .tls_config(tls_config)?
            .connect()
            .await
            .map_err(|e| {
                if let Some(source_err) = e.source() {
                    source_err.to_string()
                } else {
                    e.to_string()
                }
            })?;

        Ok(GoferClient::new(channel))
    }
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manages service related commands pertaining to administration.
    Service(service::ServiceSubcommands),

    /// Manages namespace related commands. Most commands are admin only.
    Namespace(namespace::NamespaceSubcommands),

    /// Manages pipeline related commands.
    Pipeline(pipeline::PipelineSubcommands),

    /// Managers run related commands.
    Run(run::RunSubcommands),

    /// Manages trigger related commands.
    Trigger(trigger::TriggerSubcommands),

    /// List and get information about Gofer events.
    Event(event::EventSubcommands),
}

fn init_logging(severity: Severity) -> slog_scope::GlobalLoggerGuard {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(severity);
    builder.destination(Destination::Stderr);

    let root_logger = builder.build().unwrap();
    let log = slog::Logger::root(root_logger, o!());

    slog_scope::set_global_logger(log)
}

/// Return the current epoch time
fn epoch() -> u64 {
    let current_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    u64::try_from(current_epoch).unwrap()
}

/// Transforms the given time into a humanized duration string from the current time.
///  or if time is not valid returns None.
/// (i.e. 'about an hour ago' )
fn humanize_relative_duration(time: u64) -> Option<String> {
    if time == 0 {
        return None;
    }

    let time_diff = epoch() - time;
    let time_diff_duration = chrono::Duration::milliseconds(-(time_diff as i64));
    Some(HumanTime::from(time_diff_duration).to_string())
}

/// Transforms the given two time intervals into a humanized duration string.
/// Subtracts time two(end time) from time one(start time).
fn humanize_absolute_duration(time_one: u64, time_two: u64) -> String {
    let mut time_two = time_two;

    // If time_two is just zero the thing we're trying to calculate
    // the duration of probably isn't finished. So we'll sub in a current
    // running time by entering current epoch.
    if time_two == 0 {
        time_two = epoch()
    }

    // If we'll get a negative number by subtracting the times then we should just
    // return zero.
    if time_two < time_one {
        return "0s".to_string();
    }

    let time_diff = time_two - time_one;
    let time_diff_duration = chrono::Duration::milliseconds(time_diff as i64);
    chrono_humanize::HumanTime::from(time_diff_duration)
        .to_text_en(Accuracy::Precise, Tense::Present)
}

/// init the CLI and appropriately run the correct command.
pub async fn init() {
    let args = Cli::parse();

    let config = match conf::Kind::new_cli_config()
        .parse(&args.config_path)
        .unwrap()
    {
        conf::Kind::Cli(parsed_config) => parsed_config,
        _ => {
            panic!("Incorrect configuration file received")
        }
    };

    let mut cli = CliHarness { config };

    match args.command {
        Commands::Service(service) => {
            let service_cmds = service.command;
            match service_cmds {
                service::ServiceCommands::Start => {
                    if let conf::Kind::Api(parsed_config) = conf::Kind::new_api_config()
                        .parse(&args.config_path)
                        .unwrap()
                    {
                        let severity =
                            sloggers::types::Severity::from_str(&parsed_config.general.log_level)
                                .expect(
                                    "could not parse log_level; must be one of
                                ['trace', 'debug', 'info', 'warning', 'error', 'critical']",
                                );
                        let _guard = init_logging(severity);
                        cli.service_start(*parsed_config).await;
                    } else {
                        panic!("Incorrect configuration file received trying to start api")
                    }
                }
                service::ServiceCommands::Info => {
                    cli.service_info().await;
                }
            }
        }
        Commands::Namespace(namespace) => {
            let namespace_cmds = namespace.command;
            match namespace_cmds {
                namespace::NamespaceCommands::List => cli.namespace_list().await,
                namespace::NamespaceCommands::Create {
                    id,
                    name,
                    description,
                } => cli.namespace_create(&id, name, description).await,

                namespace::NamespaceCommands::Get { id } => cli.namespace_get(&id).await,
                namespace::NamespaceCommands::Update {
                    id,
                    name,
                    description,
                } => cli.namespace_update(&id, name, description).await,
                namespace::NamespaceCommands::Delete { id } => cli.namespace_delete(&id).await,
            }
        }
        Commands::Pipeline(pipeline) => {
            let pipeline_cmds = pipeline.command;

            if let Some(namespace) = pipeline.namespace {
                cli.default_namespace(&namespace);
            }

            match pipeline_cmds {
                pipeline::PipelineCommands::List => cli.pipeline_list().await,
                pipeline::PipelineCommands::Create { path } => cli.pipeline_create(&path).await,
                pipeline::PipelineCommands::Get { id } => cli.pipeline_get(&id).await,
                pipeline::PipelineCommands::Run { id, variables } => {
                    cli.pipeline_run(&id, variables).await
                }
                pipeline::PipelineCommands::Update { path } => cli.pipeline_update(&path).await,
                pipeline::PipelineCommands::Delete { id } => cli.pipeline_delete(&id).await,
            }
        }
        Commands::Run(run) => match run.command {
            run::RunCommands::Get { pipeline_id, id } => cli.run_get(pipeline_id, id).await,
            run::RunCommands::List { pipeline_id } => cli.run_list(pipeline_id).await,
            _ => todo!(),
        },
        Commands::Trigger(trigger) => {
            let trigger_cmds = trigger.command;

            match trigger_cmds {
                trigger::TriggerCommands::Install {
                    name,
                    image,
                    user,
                    pass,
                    installer,
                    variables,
                } => {
                    cli.trigger_install(&name, &image, user, pass, installer, variables)
                        .await
                }
                trigger::TriggerCommands::List => cli.trigger_list().await,

                _ => todo!(),
            }
        }
        Commands::Event(event) => {
            let event_cmds = event.command;

            match event_cmds {
                event::EventCommands::Get { id } => cli.event_get(id).await,
                event::EventCommands::List { reverse, follow } => {
                    cli.event_list(reverse, follow).await
                }
            }
        }
    }
}
