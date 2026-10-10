use crate::{api::start_web_services, cli::Cli};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args, Clone)]
pub struct ServiceSubcommands {
    #[clap(subcommand)]
    pub command: ServiceCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ServiceCommands {
    /// Start the Gofer API server.
    Start {
        /// Read the server config from this file instead of /etc/gofer/gofer_web.toml. 'gofer extension reload'
        /// rereads the same file.
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

impl Cli {
    pub async fn handle_service_subcommands(&self, command: ServiceSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            ServiceCommands::Start { config } => start_web_services(config).await,
        }
    }
}
