mod global;
mod pipeline;

use crate::cli::Cli;
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Args, Clone)]
pub struct SecretSubcommands {
    #[clap(subcommand)]
    pub command: SecretCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum SecretCommands {
    /// Manage global secrets.
    ///
    /// Gofer allows you to store global secrets. These secrets are then used to populate all the place where
    /// Gofer needs to use shared secrets. Only accessible to admins.
    Global(global::GlobalSecretSubcommands),

    /// Manage pipeline secrets.
    ///
    /// Gofer allows you to store pipeline secrets. These secrets are then used to populate wherever variables are used.
    Pipeline(pipeline::PipelineSecretSubcommands),
}

impl Cli {
    pub async fn handle_secret_subcommands(&self, command: SecretSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            SecretCommands::Global(secret) => self.handle_global_secret_subcommands(secret).await,
            SecretCommands::Pipeline(secret) => {
                self.handle_pipeline_secret_subcommands(secret).await
            }
        }
    }
}
