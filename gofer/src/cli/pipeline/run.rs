use super::super::CliHarness;
use crate::cli::{parse_variables, DEFAULT_NAMESPACE};
use colored::Colorize;
use std::process;

impl CliHarness {
    pub async fn pipeline_run(&self, id: &str, variables: Vec<String>) {
        let vars = parse_variables(variables);

        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::StartRunRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_id: id.to_string(),
            variables: vars,
        });
        let response = client
            .start_run(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            })
            .into_inner();

        let new_run = response.run.unwrap();

        println!(
            "{} Started new run ({}) for pipeline '{}'",
            "✓".green(),
            new_run.id,
            id,
        );

        println!(
            "  View details of your new run: {}",
            format!("gofer run get {} {}", id, new_run.id).cyan()
        );

        println!(
            "  List all task runs: {}",
            format!("gofer taskrun list {} {}", id, new_run.id).cyan()
        );
    }
}
