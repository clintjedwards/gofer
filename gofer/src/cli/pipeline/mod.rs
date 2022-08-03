mod create;
mod delete;
mod get;
mod list;
mod run;
mod update;

use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use std::{path, process};

#[derive(Debug, Args)]
pub struct PipelineSubcommands {
    /// Set namespace for command to act upon.
    #[clap(long)]
    pub namespace: Option<String>,

    #[clap(subcommand)]
    pub command: PipelineCommands,
}

#[derive(Debug, Subcommand)]
pub enum PipelineCommands {
    /// List pipelines.
    List,

    /// Create a new pipeline.
    ///
    /// Creating a pipeline requires a pipeline configuration file. You can find documentation on
    /// how to create a pipeline configuration file
    /// [here](https://clintjedwards.com/gofer/docs/getting-started/first-steps/generate-pipeline-config).
    Create {
        /// Path to a pipeline configuration file.
        path: String,
    },

    /// Detail pipeline by id.
    Get {
        /// Pipeline Identifier.
        id: String,
    },

    /// Start executing a pipeline.
    Run {
        /// Pipeline Identifier.
        id: String,

        /// Optional environment variables to pass to your run.
        #[clap(short, long, name = "KEY=VALUE")]
        variables: Vec<String>,
    },

    /// Update to a new version of your pipeline.
    ///
    /// Updating a pipeline requires a pipeline configuration file. You can find documentation on
    /// how to manage your pipeline configuration file
    /// [here](https://clintjedwards.com/gofer/docs/getting-started/first-steps/generate-pipeline-config).
    Update {
        /// Path to a pipeline configuration file.
        path: String,
    },

    /// Delete pipeline by id.
    Delete {
        /// Pipeline Identifier.
        id: String,
    },
}

#[derive(Debug)]
enum PipelineLanguage {
    Rust,
    Golang,
}

fn rust_build_cmd(path: &str) -> std::io::Result<std::process::Child> {
    process::Command::new("cargo")
        .args(["run", &format!("--manifest-path={path}/Cargo.toml")])
        .stderr(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()
}

fn go_build_cmd(path: &str) -> std::io::Result<std::process::Child> {
    process::Command::new("/bin/sh")
        .current_dir(path)
        .args([
            "-c",
            "go build -o /tmp/gofer_go_pipeline && /tmp/gofer_go_pipeline",
        ])
        .stderr(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()
}

fn rust_helper_cmd(path: &str) -> String {
    format!("cargo run --manifest-path {path}/Cargo.toml")
}

fn go_helper_cmd(path: &str) -> String {
    format!("cd {path} && go build -o /tmp/gofer_go_pipeline && /tmp/gofer_go_pipeline")
}

fn detect_language(path: &str) -> Result<PipelineLanguage> {
    let path = path::Path::new(path);
    if !path.is_dir() {
        return Err(anyhow!("path must be a directory"));
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_details = entry.file_type()?;

        if file_details.is_dir() {
            continue;
        }

        match entry.file_name().to_string_lossy().to_string().as_str() {
            "Cargo.toml" => return Ok(PipelineLanguage::Rust),
            "go.mod" => return Ok(PipelineLanguage::Golang),
            _ => continue,
        }
    }

    return Err(anyhow!("no 'Cargo.toml' or 'go.mod' found"));
}
