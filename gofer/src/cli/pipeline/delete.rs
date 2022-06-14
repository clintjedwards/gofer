use super::super::CliHarness;
use crate::cli::DEFAULT_NAMESPACE;
use colored::Colorize;
use std::process;

impl CliHarness {
    pub async fn pipeline_delete(&self, id: &str) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
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
            eprintln!("Command failed; {}", e.message());
            process::exit(1);
        });

        println!("{} Deleted pipeline: {}", "✓".green(), id);
    }
}
