use crate::cli::Cli;
use anyhow::bail;
use anyhow::{Context, Result};
use colored::Colorize;
use polyfmt::{error, println, success, Spinner};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;

#[derive(Debug, Default)]
enum ConfigLanguage {
    #[default]
    Unknown,

    Golang,

    Rust,
}

impl Cli {
    /// The ability to create and manage pipelines is a huge selling point for Gofer.
    /// In the pursuit of making this as easy as possible we allow the use of Rust
    /// and Go as a way to generate and manage their pipeline configurations. For that to work
    /// though we have to be able to compile and run programs which implement the sdk and
    /// then collect the output.
    pub async fn pipeline_create(
        &self,
        namespace_id: Option<String>,
        path: PathBuf,
        deploy: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let spinner = Spinner::create("Creating pipeline");

        // Figure out absolute path for any given path string.
        let full_path = path.canonicalize().with_context(|| {
            format!(
                "Could not determine full path for '{}'",
                path.to_string_lossy()
            )
        })?;

        let full_path = full_path.to_string_lossy();

        // We need to detect which compiler we need to use by examining
        // the file extensions within the path given.
        let language = detect_language(&full_path)
            .with_context(|| format!("Could not determine config language for '{}'", full_path))?;

        // Spawn the relevant binary to build the configuration and collect
        // the output.
        // The stderr we use as status markers since they mostly stem from
        // the build tool's debug output.
        // The stdout we use as the final output and attempt to parse that.
        let cmd = match language {
            ConfigLanguage::Golang => go_build_cmd(&full_path),
            ConfigLanguage::Rust => rust_build_cmd(&full_path),
            ConfigLanguage::Unknown => bail!("Could not determine config language"),
        };

        let mut cmd = cmd.with_context(|| {
            format!(
                "Could not run build command for target config '{}'",
                full_path
            )
        })?;

        let max_line_length = terminal_width().unwrap_or(80);

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
                status_line.truncate(max_line_length.into());
                status_line
            });
        }

        let exit_status = cmd
            .wait()
            .context("Could not run build command for target pipeline config")?;

        if !exit_status.success() {
            if last_lines.is_empty() {
                last_lines = vec!["No output found for this pipeline build".to_string()];
            }

            let last_few_lines: Vec<String> = last_lines.into_iter().rev().take(15).collect();

            spinner.suspend(||
                error!("Could not successfully build target pipeline; Examine partial error output below:\n..."));

            for line in last_few_lines {
                spinner.suspend(|| println!("  {}", line));
            }

            match language {
                ConfigLanguage::Rust => spinner.suspend(|| {
                    println!(
                        "...\nView full error output: {}",
                        rust_helper_cmd(&path.to_string_lossy()).cyan()
                    )
                }),
                ConfigLanguage::Golang => spinner.suspend(|| {
                    println!(
                        "...\nView full error output: {}",
                        go_helper_cmd(&path.to_string_lossy()).cyan()
                    )
                }),
                ConfigLanguage::Unknown => {}
            }
            bail!("");
        }

        spinner.set_message("Parsing pipeline config".into());

        let mut output = "".to_string();
        cmd.stdout.unwrap().read_to_string(&mut output).unwrap();

        let config: gofer_sdk::api::types::Pipeline =
            serde_json::from_str(&output).context("Could not parse pipeline config")?;

        spinner.set_message("Creating pipeline config".into());

        let config_req = gofer_sdk::api::types::RegisterPipelineConfigRequest {
            config: config.clone(),
        };

        let response = self
            .client
            .register_config(&namespace, &config.id, &config_req)
            .await
            .context("Could not successfully create pipeline config")?
            .into_inner()
            .pipeline;

        drop(spinner);

        success!(
            "Registered pipeline: [{}] '{}' {}",
            response.config.pipeline_id.blue(),
            response.config.name,
            format!("v{}", response.config.version).magenta()
        );

        if deploy {
            self.client
                .deploy_config(&namespace, &config.id, response.config.version as i64)
                .await
                .context("Could not successfully deploy pipeline config")?
                .into_inner();
        }

        println!(
            "  View details of your pipeline: {}",
            format!("gofer fetch {}", response.metadata.pipeline_id).cyan()
        );
        println!(
            "  Start a new run: {}",
            format!("gofer pipeline run {}", response.metadata.pipeline_id).cyan()
        );

        Ok(())
    }
}

fn terminal_width() -> Option<u16> {
    let mut ws = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    unsafe {
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
            Some(ws.ws_col)
        } else {
            None
        }
    }
}

fn detect_language(path: &str) -> Result<ConfigLanguage> {
    let path = std::path::Path::new(path);
    if !path.is_dir() {
        bail!("path must be a directory");
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_details = entry.file_type()?;

        if file_details.is_dir() {
            continue;
        }

        match entry.file_name().to_string_lossy().to_string().as_str() {
            "Cargo.toml" => return Ok(ConfigLanguage::Rust),
            "go.mod" => return Ok(ConfigLanguage::Golang),
            _ => continue,
        }
    }

    bail!("no 'Cargo.toml' or 'go.mod' found");
}

fn rust_build_cmd(path: &str) -> std::io::Result<std::process::Child> {
    std::process::Command::new("cargo")
        .args(["run", &format!("--manifest-path={path}/Cargo.toml")])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
}

fn go_build_cmd(path: &str) -> std::io::Result<std::process::Child> {
    std::process::Command::new("/bin/sh")
        .current_dir(path)
        .args([
            "-c",
            "go build -o /tmp/gofer_go_pipeline && /tmp/gofer_go_pipeline",
        ])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
}

fn rust_helper_cmd(path: &str) -> String {
    format!("cargo run --manifest-path {path}/Cargo.toml")
}

fn go_helper_cmd(path: &str) -> String {
    format!("cd {path} && go build -o /tmp/gofer_go_pipeline && /tmp/gofer_go_pipeline")
}
