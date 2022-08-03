use super::super::CliHarness;
use super::*;
use crate::cli::{Spinner, DEFAULT_NAMESPACE};
use colored::Colorize;
use indicatif::ProgressBar;
use std::io::{BufRead, BufReader, Read};
use std::{path::PathBuf, process};

impl CliHarness {
    /// The ability to create and manage pipelines is a huge selling point for Gofer.
    /// In the pursuit of making this as easy as possible we allow the use of Rust
    /// and Go as a way to generate and manage their pipeline configurations. For that to work
    /// though we have to be able to compile and run programs which implement the sdk and
    /// then collect the output.
    pub async fn pipeline_create(&self, path: &str) {
        let spinner: ProgressBar = Spinner::new();
        spinner.set_message("Creating pipeline");

        // Figure out absolute path for any given path string.
        let parsed_path = PathBuf::from(path);
        let full_path = parsed_path.canonicalize().unwrap_or_else(|e| {
            spinner.finish_and_error(&format!(
                "Could not determine full path for '{}'; {:?}",
                parsed_path.to_string_lossy(),
                e
            ));
        });
        let full_path = full_path.to_string_lossy();

        // We need to detect which compiler we need to use by examining
        // the file extensions within the path given.
        let language = detect_language(&full_path).unwrap_or_else(|e| {
            spinner.finish_and_error(&format!(
                "Could not determine pipeline language for '{}'; {:?}",
                parsed_path.to_string_lossy(),
                e
            ));
        });

        // Spawn the relevant binary to build the configuration and collect
        // the output.
        // The stderr we use as status markers since they mostly stem from
        // the build tool's debug output.
        // The stdout we use as the final output and attempt to parse that.
        let cmd = match language {
            PipelineLanguage::Golang => go_build_cmd(&full_path),
            PipelineLanguage::Rust => rust_build_cmd(&full_path),
        };
        let mut cmd = cmd.unwrap_or_else(|e| {
            spinner.finish_and_error(&format!(
                "Could not run build command for target config '{}'; {:?}",
                full_path, e
            ));
        });

        // Print out the stderr as status markers
        let stderr = cmd.stderr.take().unwrap();
        let stderr_reader = BufReader::new(stderr).lines();

        let mut last_lines = vec![];
        for line in stderr_reader {
            let read_line = line.unwrap();
            last_lines.push(read_line.to_string());
            let read_line = read_line.trim();
            spinner.set_message({
                let mut status_line = format!("Building pipeline config: {}", read_line);
                status_line.truncate(80);
                status_line
            });
        }

        let exit_status = cmd.wait().unwrap_or_else(|e| {
            spinner.finish_and_error(&format!(
                "Could not run build command for target pipeline config; {:?}",
                e
            ));
        });

        if !exit_status.success() {
            if last_lines.is_empty() {
                last_lines = vec!["No output found for this pipeline build".to_string()];
            }

            let last_few_lines: Vec<String> = last_lines.into_iter().rev().take(15).collect();

            spinner.println_error(
                "Could not successfully build target pipeline; Examine partial error output below:\n...",
            );

            for line in last_few_lines {
                spinner.println(format!("  {}", line))
            }

            match language {
                PipelineLanguage::Rust => spinner.println(format!(
                    "...\nView full error output: {}",
                    rust_helper_cmd(&parsed_path.to_string_lossy()).cyan()
                )),
                PipelineLanguage::Golang => spinner.println(format!(
                    "...\nView full error output: {}",
                    go_helper_cmd(&parsed_path.to_string_lossy()).cyan()
                )),
            }
            spinner.finish_and_clear();
            process::exit(1);
        }

        spinner.set_message("Parsing pipeline config");

        let mut output = "".to_string();
        cmd.stdout.unwrap().read_to_string(&mut output).unwrap();

        let config: gofer_sdk::config::Pipeline =
            serde_json::from_str(&output).unwrap_or_else(|e| {
                spinner.finish_and_error(&format!("Could not parse pipeline config; {}", e));
            });

        spinner.set_message("Creating pipeline config");

        let mut client = self.connect().await.unwrap_or_else(|e| {
            spinner.finish_and_error(&format!("Could not create pipeline; {}", e));
        });

        let request = tonic::Request::new(gofer_proto::CreatePipelineRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_config: Some(config.into()),
        });
        let response = client
            .create_pipeline(request)
            .await
            .unwrap_or_else(|e| {
                spinner.finish_and_error(&format!("Could not create pipeline; {}", e));
            })
            .into_inner();

        let created_pipeline = response.pipeline.unwrap();

        spinner.finish_and_clear();

        println!(
            "{} Created pipeline: [{}] {}",
            "✓".green(),
            created_pipeline.id.green(),
            created_pipeline.name
        );
        println!(
            "  View details of your new pipeline: {}",
            format!("gofer pipeline get {}", created_pipeline.id).cyan()
        );
        println!(
            "  Start a new run: {}",
            format!("gofer pipeline run {}", created_pipeline.id).cyan()
        );
    }
}
