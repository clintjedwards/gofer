use clap::{Args, Subcommand};

use super::CliHarness;

#[derive(Debug, Args)]
pub struct EventSubcommands {
    #[clap(subcommand)]
    pub command: EventCommands,
}

#[derive(Debug, Subcommand)]
pub enum EventCommands {
    /// Detail event by id.
    Get {
        /// Event Identifier.
        id: String,
    },

    /// List all events; default from oldest event to newest.
    List {
        /// Sort events from newest to oldest.
        #[clap(short, long)]
        reverse: bool,

        /// Continuously wait for more events; does not work with reverse.
        #[clap(short, long)]
        follow: bool,
    },
}

impl CliHarness {
    pub async fn event_get(&self) {
        todo!()
    }

    pub async fn event_list(&self) {
        todo!()
    }
}
