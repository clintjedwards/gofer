use crate::api;
use crate::conf;
use gofer_proto::GetSystemInfoRequest;

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
        api::Api::start(config).await;
    }

    pub async fn service_info(&self) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {:?}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(GetSystemInfoRequest {});
        let response = client.get_system_info(request).await.unwrap_or_else(|e| {
            eprintln!("Command failed; {:?}", e.message());
            process::exit(1);
        });

        println!("{:?}", response.into_inner());
    }
}
