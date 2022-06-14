use super::super::CliHarness;
use crate::cli::{Spinner, DEFAULT_NAMESPACE};
use colored::Colorize;
use indicatif::ProgressBar;
use std::io::{BufRead, BufReader, Read};
use std::{path::PathBuf, process};

impl CliHarness {
    /// The ability to create and manage pipelines is a huge selling point for Gofer.
    /// In the pursuit of making this as easy as possible we allow the user to use rust
    /// as a way to generate and manage their pipeline configurations. For that to work
    /// though we have to be able to compile and run programs which implement the sdk and
    /// then collect the output.
    pub async fn trigger_uninstall(&self, path: &str) {
        let spinner: ProgressBar = Spinner::new();
        spinner.set_message("Creating pipeline");

        // Figure out absolute path for any given path string.
        let path = PathBuf::from(path);
        let full_path = match path.canonicalize() {
            Ok(path) => path,
            Err(e) => {
                spinner.finish_and_error(&format!(
                    "Could not determine full path for '{}'; {}",
                    path.to_string_lossy(),
                    e
                ));
            }
        };
        let full_path = full_path.to_string_lossy();

        // Spawn the relevant binary to build the configuration and collect
        // the output.
        // The stderr we use as status markers since they mostly stem from
        // the build tool's debug output.
        // The stdout we use as the final output and attempt to parse that.
        let mut cmd = match process::Command::new("cargo")
            .args(["run", &format!("--manifest-path={full_path}/Cargo.toml")])
            .stderr(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .spawn()
        {
            Ok(cmd) => cmd,
            Err(e) => {
                spinner.finish_and_error(&format!(
                    "Could not run build command for target config '{}'; {}",
                    full_path, e
                ));
            }
        };

        // Print out the stderr as status markers
        let stderr = cmd.stderr.take().unwrap();
        let stderr_reader = BufReader::new(stderr).lines();

        for line in stderr_reader {
            let line = line.unwrap();
            spinner.set_message({
                let mut status_line = format!("Building pipeline config: {}", line.trim());
                status_line.truncate(80);
                status_line
            });
        }

        let exit_status = match cmd.wait() {
            Ok(status) => status,
            Err(e) => {
                spinner.finish_and_error(&format!(
                    "Could not run build command for target config; {}",
                    e
                ));
            }
        };

        if !exit_status.success() {
            let mut output = String::from("");
            cmd.stderr.unwrap().read_to_string(&mut output).unwrap();

            spinner.finish_and_error(&format!(
                "Could not run build command for target config; {}",
                output
            ));
        }

        spinner.set_message("Parsing pipeline config");

        let mut output = "".to_string();
        cmd.stdout.unwrap().read_to_string(&mut output).unwrap();

        let config: gofer_sdk::config::Pipeline = match serde_json::from_str(&output) {
            Ok(config) => config,
            Err(e) => {
                spinner.finish_and_error(&format!("Could not parse pipeline config; {}", e));
            }
        };

        spinner.set_message("Creating pipeline config");

        let mut client = match self.connect().await {
            Ok(client) => client,
            Err(e) => {
                spinner.finish_and_error(&format!("Could not create pipeline; {}", e));
            }
        };

        let request = tonic::Request::new(gofer_proto::CreatePipelineRequest {
            namespace_id: self
                .config
                .namespace
                .clone()
                .unwrap_or_else(|| DEFAULT_NAMESPACE.to_string()),
            pipeline_config: Some(config.into()),
        });
        let response = match client.create_pipeline(request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                spinner.finish_and_error(&format!("Could not create pipeline; {}", e));
            }
        };

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
