mod create;
mod delete;
mod get;
mod list;
mod run;
mod update;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct PipelineSubcommands {
    /// Set namespace for command to act upon.
    #[clap(long)]
    pub namespace: Option<String>,

    #[clap(subcommand)]
    pub command: PipelineCommands,
}

#[derive(Debug, Subcommand)]
pub enum PipelineCommands {
    /// List pipelines.
    List,

    /// Create a new pipeline.
    ///
    /// Creating a pipeline requires a pipeline configuration file. You can find documentation on
    /// how to create a pipeline configuration file
    /// [here](https://clintjedwards.com/gofer/docs/getting-started/first-steps/generate-pipeline-config).
    Create {
        /// Path to a pipeline configuration file.
        path: String,
    },

    /// Detail pipeline by id.
    Get { id: String },

    /// Start executing a pipeline.
    Run { id: String, variables: Vec<String> },

    /// Update a new pipeline.
    ///
    /// Updating a pipeline requires a pipeline configuration file. You can find documentation on
    /// how to manage your pipeline configuration file
    /// [here](https://clintjedwards.com/gofer/docs/getting-started/first-steps/generate-pipeline-config).
    Update {
        /// Path to a pipeline configuration file.
        path: String,
    },

    /// Delete pipeline by id.
    Delete { id: String },
}
