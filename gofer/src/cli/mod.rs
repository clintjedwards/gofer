mod namespace;
mod pipeline;
mod service;
mod spinner;

pub use self::spinner::*;

use crate::conf::{self, cli::Config};
use clap::{Parser, Subcommand};
use gofer_proto::gofer_client::GoferClient;
use slog::o;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;
use std::{
    error::Error,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tonic::transport::channel::Channel;
use url::Url;

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
        let parsed_url = Url::parse(&self.config.server).unwrap();
        let domain_name = parsed_url.host_str().unwrap();

        let mut conn =
            tonic::transport::Channel::from_shared(self.config.server.to_string()).unwrap();

        if !&self.config.dev_mode {
            conn =
                conn.tls_config(tonic::transport::ClientTlsConfig::new().domain_name(domain_name))?;
        }

        Ok(GoferClient::new(conn.connect().await?))
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

/// humanize_duration transforms a given time into a humanized duration string from the current time
/// (i.e. 'about an hour ago' )
fn humanize_duration(time: i64) -> String {
    let time_diff = time - epoch() as i64;
    let time_diff_duration = chrono::Duration::milliseconds(time_diff);
    chrono_humanize::HumanTime::from(time_diff_duration).to_string()
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
                        cli.service_start(parsed_config).await;
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
    }
}
