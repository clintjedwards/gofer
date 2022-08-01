use super::super::CliHarness;
use crate::cli::DEFAULT_NAMESPACE;
use colored::Colorize;
use std::io::{stdin, stdout, Write};
use std::process;

impl CliHarness {
    pub async fn pipeline_delete(&self, id: &str) {
        let namespace_id = self
            .config
            .namespace
            .clone()
            .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string());

        // Check to make sure user actually wanted to delete this pipeline.
        print!(
            "Confirm deletion of pipeline '{}/{}' [y/N]: ",
            namespace_id, id
        );
        stdout().flush().unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let mut input_string = String::new();
        stdin().read_line(&mut input_string).unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        if input_string.trim().to_lowercase() != "y" {
            eprintln!("User aborted pipeline deletion");
            process::exit(1);
        }

        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e);
            process::exit(1);
        });

        let request = tonic::Request::new(gofer_proto::DeletePipelineRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            id: id.to_string(),
        });

        client.delete_pipeline(request).await.unwrap_or_else(|e| {
            eprintln!("{} Command failed; {}", "x".red(), e.message());
            process::exit(1);
        });

        println!("{} Deleted pipeline '{}'", "✓".green(), id);
    }
}
