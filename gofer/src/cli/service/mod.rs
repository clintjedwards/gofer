use crate::{api::start_web_services, cli::Cli};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Args, Clone)]
pub struct ServiceSubcommands {
    #[clap(subcommand)]
    pub command: ServiceCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ServiceCommands {
    /// Start the Gofer API server.
    Start,
}

impl Cli {
    pub async fn handle_service_subcommands(&self, command: ServiceSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            ServiceCommands::Start => start_web_services().await,
        }
    }
}
