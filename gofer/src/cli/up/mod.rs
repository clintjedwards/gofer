mod images;

use crate::cli::{Cli, api_error_message, ask_yes_no, rail};
use crate::conf::cli::OutputFormat;
use anyhow::bail;
use anyhow::{Context, Result};
use colored::Colorize;
use gofer_sdk::api::types::{self, ConfigState};
use polyfmt::{Format, error, print, println, success};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufRead, BufReader, IsTerminal, Read};
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
        yes: bool,
        rebuild: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        print!("Creating pipeline"; vec![Format::Spinner]);

        // Figure out absolute path for any given path string.
        let full_path = path.canonicalize().with_context(|| {
            format!(
                "Could not determine full path for '{}'",
                path.to_string_lossy()
            )
        })?;

        let config_folder = full_path.clone();
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
            let mut status_line = format!("Building pipeline config: {}", read_line);
            status_line.truncate(max_line_length.into());
            print!("{}", status_line; vec![Format::Spinner]);
        }

        let exit_status = cmd
            .wait()
            .context("Could not run build command for target pipeline config")?;

        if !exit_status.success() {
            if last_lines.is_empty() {
                last_lines = vec!["No output found for this pipeline build".to_string()];
            }

            // Oldest first, the way they were printed.
            let skip = last_lines.len().saturating_sub(15);
            let last_few_lines: Vec<String> = last_lines.into_iter().skip(skip).collect();

            error!(
                "Could not successfully build target pipeline; Examine partial error output below:\n..."
            );

            for line in last_few_lines {
                println!("  {}", line);
            }

            match language {
                ConfigLanguage::Rust => println!(
                    "...\nView full error output: {}",
                    rust_helper_cmd(&path.to_string_lossy()).cyan()
                ),
                ConfigLanguage::Golang => println!(
                    "...\nView full error output: {}",
                    go_helper_cmd(&path.to_string_lossy()).cyan()
                ),
                ConfigLanguage::Unknown => {}
            }
            bail!("");
        }

        print!("Parsing pipeline config"; vec![Format::Spinner]);

        let mut output = "".to_string();
        cmd.stdout.unwrap().read_to_string(&mut output).unwrap();

        let mut config: gofer_sdk::api::types::Pipeline =
            serde_json::from_str(&output).context("Could not parse pipeline config")?;

        print!("Hashing the files each task copies in"; vec![Format::Spinner]);
        let prepared = images::prepare(&config_folder, &config)?;
        let hashes: HashMap<&str, &str> = (prepared.iter())
            .map(|(id, prepared)| (id.as_str(), prepared.source_hash.as_str()))
            .collect();

        print!("Comparing with the current pipeline config"; vec![Format::Spinner]);

        let current = match self.client.get_config(&namespace, &config.id, 0).await {
            Ok(response) => Some(response.into_inner().config),
            Err(e) if e.status() == Some(reqwest::StatusCode::NOT_FOUND) => None,
            Err(e) => bail!(
                "Could not get the current pipeline config; {}",
                api_error_message(e)
            ),
        };

        // With --rebuild nothing is kept, so a base image that changed upstream gets downloaded again.
        let reusable = match rebuild {
            true => images::Reusable::default(),
            false => {
                self.reusable_images(&namespace, &config, &prepared, current.as_ref())
                    .await?
            }
        };

        match &current {
            None => {
                let task_ids: Vec<&str> =
                    config.tasks.iter().map(|task| task.id.as_str()).collect();
                println!(
                    "New pipeline '{}' with {} task(s): {}",
                    config.id,
                    task_ids.len(),
                    task_ids.join(", ")
                );
            }
            Some(current) => {
                let changes = diff(&current_fields(current), &new_fields(&config, &hashes));

                if changes.is_empty() && reusable.gone.is_empty() && !rebuild {
                    return self.unchanged(&namespace, current, deploy).await;
                }

                println!(
                    "Changes since {}:",
                    format!("v{}", current.version).magenta()
                );
                for change in &changes {
                    print_change(change);
                }
                for id in &reusable.gone {
                    println!(
                        "  {} {} tasks.{id}: its image is gone from Gofer, so it's built again",
                        rail(),
                        "~".yellow()
                    );
                }
                if rebuild {
                    println!(
                        "  {} {} every task's image is built again from scratch",
                        rail(),
                        "~".yellow()
                    );
                }

                // Prompts can't be answered when nobody's at a terminal or the output is JSON or silent, so in those
                // cases we go ahead like before; --yes does the same on purpose.
                let can_prompt = std::io::stdin().is_terminal()
                    && matches!(
                        self.conf.output_format,
                        OutputFormat::Spinner | OutputFormat::Plain
                    );
                let action = if deploy {
                    "Register and deploy"
                } else {
                    "Register"
                };
                if !yes && can_prompt && !ask_yes_no(&format!("{action} these changes?")) {
                    println!("Nothing was changed");
                    return Ok(());
                }
            }
        }

        let to_build: Vec<&types::Task2> = (config.tasks.iter())
            .filter(|task| !reusable.images.contains_key(&task.id))
            .collect();

        let mut images = reusable.images;
        if !to_build.is_empty() {
            let built = self
                .build_images(&namespace, &config, &to_build, &prepared, rebuild)
                .await?;
            self.upload_images(&namespace, &config.id, &built).await?;

            for (ids, built) in &built {
                for id in ids {
                    images.insert(
                        id.clone(),
                        types::BuiltImage {
                            digest: built.image.manifest.to_string(),
                            source_hash: prepared[id].source_hash.clone(),
                        },
                    );
                }
            }
        }

        for task in &mut config.tasks {
            task.image = images.remove(&task.id);
        }

        print!("Creating pipeline config"; vec![Format::Spinner]);

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

        print_next_steps(&response.metadata.pipeline_id);

        Ok(())
    }

    /// The config we built matches the latest version, so registering it would only add a duplicate version. If
    /// that version isn't the live one we still deploy it, since that's what `--deploy` asked for.
    async fn unchanged(
        &self,
        namespace: &str,
        current: &gofer_sdk::api::types::Config,
        deploy: bool,
    ) -> Result<()> {
        let version = format!("v{}", current.version).magenta();

        if deploy && current.state != ConfigState::Live {
            self.client
                .deploy_config(namespace, &current.pipeline_id, current.version as i64)
                .await
                .context("Could not successfully deploy pipeline config")?;

            success!("No changes since {version}; deployed it since it wasn't live");
        } else {
            success!("No changes since {version}; nothing to register");
        }

        print_next_steps(&current.pipeline_id);

        Ok(())
    }
}

fn print_next_steps(pipeline_id: &str) {
    println!(
        "  View details of your pipeline: {}",
        format!("gofer fetch {pipeline_id}").cyan()
    );
    println!(
        "  Start a new run: {}",
        format!("gofer pipeline run {pipeline_id}").cyan()
    );
}

/// A pipeline config flattened into `field -> value`, like `tasks.build.base -> alpine`. The config we just built
/// and the one Gofer has come back in different shapes (a task list with variables as a map versus a task map with
/// variables as a list), so we flatten both and compare those instead.
type Fields = BTreeMap<String, String>;

/// One task, in the shape both sides get converted to before flattening.
struct FlatTask {
    id: String,
    description: String,
    build: Vec<(String, String)>,

    /// The hash of the files the task copies in. Only whether it changed is shown.
    files: String,

    depends_on: BTreeMap<String, String>,
    variables: BTreeMap<String, String>,
    entrypoint: Option<Vec<String>>,
    command: Option<Vec<String>>,
    inject_api_token: bool,
    timeout_seconds: u64,
}

impl FlatTask {
    fn insert_into(self, fields: &mut Fields) {
        let prefix = format!("tasks.{}", self.id);

        // Empty means unset, the same as on the server, so a task without a description doesn't show up as a change.
        let mut insert = |field: &str, value: String| {
            if !value.is_empty() {
                fields.insert(format!("{prefix}.{field}"), value);
            }
        };

        insert("description", self.description);
        for (field, value) in self.build {
            insert(&field, value);
        }
        insert("files", self.files);
        for (parent, status) in self.depends_on {
            insert(&format!("depends_on.{parent}"), status);
        }
        for (key, value) in self.variables {
            insert(&format!("variables.{key}"), value);
        }
        insert(
            "entrypoint",
            self.entrypoint
                .map(|args| join_args(&args))
                .unwrap_or_default(),
        );
        insert(
            "command",
            self.command
                .map(|args| join_args(&args))
                .unwrap_or_default(),
        );
        insert("inject_api_token", self.inject_api_token.to_string());
        // 0 means unset, so it's left out like the other empty values.
        if self.timeout_seconds > 0 {
            insert("timeout_seconds", self.timeout_seconds.to_string());
        }
    }
}

/// How a task's image is built, as fields. Every task has either `base` or `dockerfile`. Steps are numbered from 1,
/// in order, so adding one in the middle shows up as every step after it changing.
fn build_fields(build: &types::Build) -> Vec<(String, String)> {
    match build {
        types::Build::Steps(steps) => {
            let mut fields = vec![("base".to_string(), steps.base.clone())];

            for (number, step) in steps.steps.iter().enumerate() {
                let step = match step {
                    types::Step::Copy { from, to } => format!("copy {from} to {to}"),
                    types::Step::Run { command } => format!("run {command}"),
                };
                fields.push((format!("steps.{}", number + 1), step));
            }

            if let Some(keep) = &steps.keep {
                fields.push((
                    "keep".into(),
                    format!("{} on {}", keep.paths.join(", "), keep.base),
                ));
            }

            fields
        }

        types::Build::Dockerfile { path, context } => vec![
            ("dockerfile".into(), path.clone()),
            ("context".into(), context.clone()),
        ],
    }
}

/// Shows args the way you'd type them in a shell, so a `Script()` reads like the script it is.
fn join_args(args: &[String]) -> String {
    shlex::try_join(args.iter().map(String::as_str)).unwrap_or_else(|_| args.join(" "))
}

fn new_fields(config: &gofer_sdk::api::types::Pipeline, hashes: &HashMap<&str, &str>) -> Fields {
    let mut fields = Fields::new();
    fields.insert("name".into(), config.name.clone());
    fields.insert(
        "description".into(),
        config.description.clone().unwrap_or_default(),
    );
    fields.insert("parallelism".into(), config.parallelism.to_string());
    fields.insert("allow_runs_from".into(), config.allow_runs_from.join(", "));
    fields.insert("skip_dates".into(), config.skip_dates.join(", "));

    for task in &config.tasks {
        FlatTask {
            id: task.id.clone(),
            description: task.description.clone().unwrap_or_default(),
            build: build_fields(&task.build),
            files: hashes.get(task.id.as_str()).unwrap_or(&"").to_string(),
            depends_on: task
                .depends_on
                .iter()
                .map(|(parent, status)| (parent.clone(), status.to_string().to_lowercase()))
                .collect(),
            variables: task.variables.clone().into_iter().collect(),
            entrypoint: task.entrypoint.clone(),
            command: task.command.clone(),
            inject_api_token: task.inject_api_token,
            timeout_seconds: task.timeout_seconds,
        }
        .insert_into(&mut fields);
    }

    fields.retain(|_, value| !value.is_empty());
    fields
}

fn current_fields(config: &gofer_sdk::api::types::Config) -> Fields {
    let mut fields = Fields::new();
    fields.insert("name".into(), config.name.clone());
    fields.insert("description".into(), config.description.clone());
    fields.insert("parallelism".into(), config.parallelism.to_string());
    fields.insert("allow_runs_from".into(), config.allow_runs_from.join(", "));
    fields.insert("skip_dates".into(), config.skip_dates.join(", "));

    for task in config.tasks.values() {
        FlatTask {
            id: task.id.clone(),
            description: task.description.clone(),
            build: build_fields(&task.build),
            files: task.image.source_hash.clone(),
            depends_on: task
                .depends_on
                .iter()
                .map(|(parent, status)| (parent.clone(), status.to_string().to_lowercase()))
                .collect(),
            variables: task
                .variables
                .iter()
                .map(|variable| (variable.key.clone(), variable.value.clone()))
                .collect(),
            entrypoint: task.entrypoint.clone(),
            command: task.command.clone(),
            inject_api_token: task.inject_api_token,
            timeout_seconds: task.timeout_seconds,
        }
        .insert_into(&mut fields);
    }

    fields.retain(|_, value| !value.is_empty());
    fields
}

#[derive(Debug, PartialEq, Eq)]
struct Change {
    field: String,
    old: Option<String>,
    new: Option<String>,
}

/// Every task has a base image or a Dockerfile, and task ids can't contain dots, so `tasks.<id>.base` and
/// `tasks.<id>.dockerfile` tell us which tasks exist, and what each is built from.
fn task_builds(fields: &Fields) -> BTreeMap<&str, &str> {
    fields
        .iter()
        .filter_map(|(field, built_from)| {
            let rest = field.strip_prefix("tasks.")?;
            let id = (rest.strip_suffix(".base")).or_else(|| rest.strip_suffix(".dockerfile"))?;
            (!id.contains('.')).then_some((id, built_from.as_str()))
        })
        .collect()
}

/// A task that was added or removed as a whole is one change, `tasks.<id>`, with what it's built from as the value. Listing all
/// of its fields would mostly be defaults like `inject_api_token = false`.
fn diff(old: &Fields, new: &Fields) -> Vec<Change> {
    let old_tasks = task_builds(old);
    let new_tasks = task_builds(new);
    let in_whole_task = |field: &str| {
        field
            .strip_prefix("tasks.")
            .and_then(|rest| rest.split('.').next())
            .is_some_and(|id| old_tasks.contains_key(id) != new_tasks.contains_key(id))
    };

    let mut changes: Vec<Change> = old
        .keys()
        .chain(new.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|field| !in_whole_task(field) && old.get(*field) != new.get(*field))
        .map(|field| Change {
            field: field.clone(),
            old: old.get(field).cloned(),
            new: new.get(field).cloned(),
        })
        .collect();

    for id in old_tasks
        .keys()
        .chain(new_tasks.keys())
        .collect::<BTreeSet<_>>()
    {
        if old_tasks.contains_key(id) != new_tasks.contains_key(id) {
            changes.push(Change {
                field: format!("tasks.{id}"),
                old: old_tasks.get(id).map(|built_from| built_from.to_string()),
                new: new_tasks.get(id).map(|built_from| built_from.to_string()),
            });
        }
    }

    changes.sort_by(|a, b| a.field.cmp(&b.field));
    changes
}

/// Prints a change in the same `+`/`-`/`~` style as `gofer extension reload`. Values that span lines, which is
/// usually a `Script()`, get a line diff instead so a one line edit doesn't print the whole script twice.
fn print_change(change: &Change) {
    let field = &change.field;
    let multiline = |value: &Option<String>| value.as_deref().is_some_and(|v| v.contains('\n'));

    if multiline(&change.old) || multiline(&change.new) {
        let (sign, header) = match (&change.old, &change.new) {
            (None, _) => ("+".green(), format!("{field}:")),
            (_, None) => ("-".red(), format!("{field} (removed):")),
            _ => ("~".yellow(), format!("{field}:")),
        };
        println!("  {} {sign} {header}", rail());
        print_line_diff(
            change.old.as_deref().unwrap_or_default(),
            change.new.as_deref().unwrap_or_default(),
        );
        return;
    }

    // A whole task; see [`diff`].
    let is_task = field.matches('.').count() == 1 && field.starts_with("tasks.");

    let line = match (&change.old, &change.new) {
        // Hashes mean nothing to read, so only say that the files changed.
        (Some(_), Some(_)) if field.ends_with(".files") => {
            format!("{} {field}: changed", "~".yellow())
        }
        (None, Some(built_from)) if is_task => {
            format!("{} {field}: new task built from {built_from}", "+".green())
        }
        (Some(built_from), None) if is_task => {
            format!(
                "{} {field}: removed (was built from {built_from})",
                "-".red()
            )
        }
        (None, Some(new)) => format!("{} {field} = {new}", "+".green()),
        (Some(old), None) => format!("{} {field} (was {old})", "-".red()),
        (Some(old), Some(new)) => format!("{} {field}: {old} -> {new}", "~".yellow()),
        (None, None) => return,
    };
    println!("  {} {line}", rail());
}

fn print_line_diff(old: &str, new: &str) {
    let diff = similar::TextDiff::from_lines(old, new);

    for (index, group) in diff.grouped_ops(2).iter().enumerate() {
        if index > 0 {
            println!("  {}       {}", rail(), "...".dimmed());
        }

        for op in group {
            for change in diff.iter_changes(op) {
                let text = change.value().trim_end_matches('\n');
                let line = match change.tag() {
                    similar::ChangeTag::Delete => format!("- {text}").red().to_string(),
                    similar::ChangeTag::Insert => format!("+ {text}").green().to_string(),
                    similar::ChangeTag::Equal => format!("  {text}").dimmed().to_string(),
                };
                println!("  {}     {line}", rail());
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // The same pipeline as the SDK writes it out...
    fn built() -> gofer_sdk::api::types::Pipeline {
        serde_json::from_value(serde_json::json!({
            "id": "simple",
            "name": "Simple",
            "parallelism": 0,
            "allow_runs_from": [],
            "skip_dates": [],
            "tasks": [{
                "id": "build",
                "build": {"steps": {"base": "alpine:latest", "steps": [{"run": {"command": "make"}}], "keep": null}},
                "depends_on": {"test": "Success"},
                "variables": {"LOG_LEVEL": "debug"},
                "command": ["sh", "-c", "echo one\necho two\necho three"],
                "inject_api_token": false
            }]
        }))
        .unwrap()
    }

    // ...and as the API hands it back.
    fn stored() -> gofer_sdk::api::types::Config {
        serde_json::from_value(serde_json::json!({
            "namespace_id": "default",
            "pipeline_id": "simple",
            "version": 1,
            "parallelism": 0,
            "allow_runs_from": [],
            "skip_dates": [],
            "name": "Simple",
            "description": "",
            "state": "live",
            "registered": 0,
            "deprecated": 0,
            "tasks": {"build": {
                "id": "build",
                "description": "",
                "build": {"steps": {"base": "alpine:latest", "steps": [{"run": {"command": "make"}}], "keep": null}},
                "image": {"digest": "sha256:aaaa", "source_hash": "bbbb"},
                "depends_on": {"test": "success"},
                "variables": [{"key": "LOG_LEVEL", "value": "debug", "source": "pipeline_config"}],
                "command": ["sh", "-c", "echo one\necho two\necho three"],
                "inject_api_token": false,
                "timeout_seconds": 0
            }}
        }))
        .unwrap()
    }

    /// The hash of the files the stored task was built from.
    fn same_files() -> HashMap<&'static str, &'static str> {
        HashMap::from([("build", "bbbb"), ("compile", "bbbb")])
    }

    #[test]
    fn test_changed_files_are_one_change() {
        let changes = diff(
            &current_fields(&stored()),
            &new_fields(&built(), &HashMap::from([("build", "cccc")])),
        );

        assert_eq!(
            changes,
            vec![Change {
                field: "tasks.build.files".into(),
                old: Some("bbbb".into()),
                new: Some("cccc".into()),
            }]
        );
    }

    #[test]
    fn test_added_and_removed_tasks_are_one_change_each() {
        let mut config = built();
        config.tasks[0].id = "compile".into();

        let changes = diff(
            &current_fields(&stored()),
            &new_fields(&config, &same_files()),
        );

        assert_eq!(
            changes,
            vec![
                Change {
                    field: "tasks.build".into(),
                    old: Some("alpine:latest".into()),
                    new: None,
                },
                Change {
                    field: "tasks.compile".into(),
                    old: None,
                    new: Some("alpine:latest".into()),
                },
            ]
        );
    }

    #[test]
    fn test_same_config_in_both_shapes_has_no_changes() {
        assert_eq!(
            diff(
                &current_fields(&stored()),
                &new_fields(&built(), &same_files())
            ),
            vec![]
        );
    }

    #[test]
    fn test_changes_are_per_field() {
        let mut config = built();
        let types::Build::Steps(steps) = &mut config.tasks[0].build else {
            panic!("the test config is built from steps");
        };
        steps.base = "alpine:3.20".into();
        steps.steps.push(types::Step::Copy {
            from: "./app".into(),
            to: "/app".into(),
        });
        config.tasks[0].variables.remove("LOG_LEVEL");
        config.tasks[0].command = Some(vec![
            "sh".into(),
            "-c".into(),
            "echo one\necho 2\necho three".into(),
        ]);

        let changes = diff(
            &current_fields(&stored()),
            &new_fields(&config, &same_files()),
        );

        assert_eq!(
            changes,
            vec![
                Change {
                    field: "tasks.build.base".into(),
                    old: Some("alpine:latest".into()),
                    new: Some("alpine:3.20".into()),
                },
                Change {
                    field: "tasks.build.command".into(),
                    old: Some("sh -c 'echo one\necho two\necho three'".into()),
                    new: Some("sh -c 'echo one\necho 2\necho three'".into()),
                },
                Change {
                    field: "tasks.build.steps.2".into(),
                    old: None,
                    new: Some("copy ./app to /app".into()),
                },
                Change {
                    field: "tasks.build.variables.LOG_LEVEL".into(),
                    old: Some("debug".into()),
                    new: None,
                },
            ]
        );
    }
}
