use super::super::CliHarness;
use crate::cli::DEFAULT_NAMESPACE;
use colored::Colorize;
use std::{collections::HashMap, process};

impl CliHarness {
    pub async fn trigger_install(
        &self,
        name: &str,
        image: &str,
        host: Option<String>,
        user: Option<String>,
        pass: Option<String>,
    ) {
        unimplemented!()
        // let mut client = match self.connect().await {
        //     Ok(client) => client,
        //     Err(e) => {
        //         eprintln!("Command failed; {}", e);
        //         process::exit(1);
        //     }
        // };

        // let request = tonic::Request::new(gofer_proto::RunPipelineRequest {
        //     namespace_id: self
        //         .config
        //         .namespace
        //         .clone()
        //         .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
        //     id: id.to_string(),
        //     variables: vars,
        // });
        // let response = match client.run_pipeline(request).await {
        //     Ok(response) => response.into_inner(),
        //     Err(e) => {
        //         eprintln!("Command failed; {}", e.message());
        //         process::exit(1);
        //     }
        // };

        // let new_run = response.run.unwrap();

        // println!(
        //     "{} Started new run ({}) for pipeline '{}'",
        //     "✓".green(),
        //     new_run.id,
        //     id,
        // );

        // println!(
        //     "  View details of your new run: {}",
        //     format!("gofer run get {} {}", id, new_run.id).cyan()
        // );

        // println!(
        //     "  List all task runs: {}",
        //     format!("gofer taskrun list {} {}", id, new_run.id).cyan()
        // );
    }
}
