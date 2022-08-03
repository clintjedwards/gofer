use clap::{Args, Subcommand};
use colored::Colorize;
use futures::{stream::Stream, TryStreamExt};
use std::process;
use tokio_stream::StreamExt;

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

    /// List all events; defaults from oldest event to newest.
    List {
        /// Change the order of events to newest first instead of oldest first (reverse chronological order).
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

        println!("{:#?}", response.event.unwrap()); //TODO(clintjedwards): Make this pretty
    }

    pub async fn event_list(&self, reverse: bool, follow: bool) {
        if reverse && follow {
            eprintln!(
                "{} Command failed; flags 'reverse' and 'follow' cannot both be true",
                "x".red(),
            );
            process::exit(1);
        }

        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::ListEventsRequest { reverse, follow });
        let mut response = client
            .list_events(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("{} Command failed; {}", "x".red(), e.message());
                process::exit(1);
            })
            .into_inner();

        loop {
            let msg = match response.message().await {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("{} Command failed; {}", "x".red(), e.message());
                    process::exit(1);
                }
            };

            let msg = match msg {
                Some(msg) => msg,
                None => return,
            };

            println!("{:?}", msg.event.unwrap());
        }
    }
}
