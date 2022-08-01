use clap::{Args, Subcommand};
use colored::Colorize;
use std::process;

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
        id: u64,
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
    pub async fn event_get(&self, id: u64) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::GetEventRequest { id });
        let response = client
            .get_event(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner();

        let event = response.event.unwrap();
        dbg!(event);
    }

    pub async fn event_list(&self) {
        todo!()
    }
}
