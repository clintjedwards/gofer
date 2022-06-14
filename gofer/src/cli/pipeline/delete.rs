use super::super::CliHarness;
use crate::cli::DEFAULT_NAMESPACE;
use colored::Colorize;
use std::process;

impl CliHarness {
    pub async fn pipeline_delete(&self, id: &str) {
        let mut client = match self.connect().await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            }
        };

        let request = tonic::Request::new(gofer_proto::DeletePipelineRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            id: id.to_string(),
        });

        match client.delete_pipeline(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                eprintln!("Command failed; {}", e.message());
                process::exit(1);
            }
        };

        println!("{} Deleted pipeline: {}", "✓".green(), id);
    }
}
