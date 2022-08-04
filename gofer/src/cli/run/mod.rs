mod get;
mod list;

use super::CliHarness;
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct RunSubcommands {
    #[clap(subcommand)]
    pub command: RunCommands,
}

#[derive(Debug, Subcommand)]
pub enum RunCommands {
    /// Detail run by id.
    Get {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        id: u64,
    },

    /// List all runs; defaults from oldest run to newest.
    List {
        /// Pipeline Identifier.
        pipeline_id: String,
    },

    /// Start a new run.
    Start {
        /// Pipeline Identifier.
        pipeline_id: String,
    },

    /// Cancel a run in progress.
    Cancel {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        id: u64,
    },

    /// Cancels all runs for a given pipeline.
    CancelAll {
        /// Pipeline Identifier.
        pipeline_id: String,
    },
}
