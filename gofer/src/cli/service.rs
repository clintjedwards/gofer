use crate::api;
use crate::conf;
use gofer_proto::{gofer_client::GoferClient, GetSystemInfoRequest};

use clap::{Args, Subcommand};
use std::process;

use super::CliHarness;

#[derive(Debug, Args)]
pub struct ServiceSubcommands {
    #[clap(subcommand)]
    pub command: ServiceCommands,
}

#[derive(Debug, Subcommand)]
pub enum ServiceCommands {
    /// Start the Gofer GRPC service.
    #[clap(
        long_about = "Gofer runs a a GRPC backend combined with GRPC-WEB/HTTP1.
    Running this command attempts to start the long running service. This command will block and only
    gracefully stop on SIGINT or SIGTERM signals."
    )]
    Start,

    /// Retrieve general information about Gofer's systems
    Info,
}

impl CliHarness {
    pub async fn service_start(&self, config: conf::api::Config) {
        let api = api::Api::new(config).await;
        api.start_service().await;
    }

    pub async fn service_info(&self) {
        let channel = match tonic::transport::Channel::from_shared(self.config.server.to_string()) {
            Ok(channel) => channel,
            Err(e) => {
                eprintln!("Could not open transport channel; {}", e);
                process::exit(1);
            }
        };

        let conn = match channel.connect().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Could not connect to server; {}", e);
                process::exit(1);
            }
        };

        let mut client = GoferClient::new(conn);
        let request = tonic::Request::new(GetSystemInfoRequest {});
        let response = match client.get_system_info(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                eprintln!("Could not get info; {}", e.message());
                process::exit(1);
            }
        };

        println!("{:?}", response);
    }
}
