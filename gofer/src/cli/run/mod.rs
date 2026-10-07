mod debug;
mod object;

use crate::cli::{
    Cli, colorize_status_text, colorize_status_text_comfy, dependencies, duration, rail, rail_table,
};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{println, success};
use std::collections::HashMap;

#[derive(Debug, Args, Clone)]
pub struct RunSubcommands {
    #[clap(subcommand)]
    pub command: RunCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum RunCommands {
    /// List all runs.
    List {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Limit the amount of results returned
        #[arg(short, long, default_value = "10")]
        limit: u64,

        /// How many runs to skip, useful for paging through results.
        #[arg(short, long, default_value = "0")]
        offset: u64,

        /// Reverse the return order back to ascending order. By default lists runs in descending order.
        #[arg(short, long, default_value = "false")]
        no_reverse: bool,
    },

    /// Get details on a single run.
    Get {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,
    },

    /// Show what went wrong in a run.
    ///
    /// Prints a timeline of every task in the run, then details on each task that failed, was cancelled, or was
    /// skipped. Failed tasks include the last few lines of their output.
    Debug {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// How many lines of output to show for each failed task.
        #[arg(short, long, default_value = "10")]
        lines: usize,
    },

    /// Start a run.
    Start {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Optional environment variables to pass to your run. Format: Key=Value
        #[arg(short, long)]
        variable: Vec<String>,
    },
    Cancel {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,
    },

    /// Manage run object store.
    Object(object::ObjectSubcommands),
}

impl Cli {
    pub async fn handle_run_subcommands(&self, command: RunSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            RunCommands::List {
                pipeline_id,
                limit,
                offset,
                no_reverse,
            } => {
                self.run_list(command.namespace, &pipeline_id, limit, offset, no_reverse)
                    .await
            }
            RunCommands::Get {
                pipeline_id,
                run_id,
            } => self.run_get(command.namespace, &pipeline_id, run_id).await,
            RunCommands::Debug {
                pipeline_id,
                run_id,
                lines,
            } => {
                self.run_debug(command.namespace, &pipeline_id, run_id, lines)
                    .await
            }
            RunCommands::Start {
                pipeline_id,
                variable,
            } => {
                self.run_start(command.namespace, &pipeline_id, variable)
                    .await
            }
            RunCommands::Cancel {
                pipeline_id,
                run_id,
            } => {
                self.run_cancel(command.namespace, &pipeline_id, run_id)
                    .await
            }
            RunCommands::Object(object) => self.handle_run_object_subcommands(object).await,
        }
    }
}

/// The first line of `run get` and `run debug`, so both commands start out looking the same.
fn run_title(run: &gofer_sdk::api::types::Run) -> String {
    format!(
        "  Run {} for pipeline {} (v{}) :: {} :: {}",
        format!("#{}", run.run_id).cyan(),
        run.pipeline_id.cyan(),
        run.pipeline_config_version,
        colorize_status_text(run.state),
        colorize_status_text(run.status)
    )
}

impl Cli {
    pub async fn run_list(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        limit: u64,
        offset: u64,
        no_reverse: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let reverse = !no_reverse;

        let runs = self
            .client
            .list_runs(
                &namespace,
                pipeline_id,
                Some(limit),
                Some(offset),
                Some(reverse),
            )
            .await
            .context("Could not successfully retrieve runs from Gofer api")?
            .into_inner()
            .runs;

        let mut table = comfy_table::Table::new();
        table
            .load_style(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("started")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("ended")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("duration")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("status")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("started by")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for run in runs {
            table.add_row(vec![
                Cell::new(run.run_id).fg(Color::Green),
                Cell::new(
                    self.format_time(run.started)
                        .unwrap_or("Unknown".to_string()),
                ),
                Cell::new(self.format_time(run.ended).unwrap_or("Unknown".to_string())),
                Cell::new(duration(run.started as i64, run.ended as i64)),
                Cell::new(run.state).fg(colorize_status_text_comfy(run.state)),
                Cell::new(run.status).fg(colorize_status_text_comfy(run.status)),
                Cell::new(run.initiator.user),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn run_get(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let run = self
            .client
            .get_run(&namespace, pipeline_id, run_id)
            .await
            .context("Could not successfully retrieve run from Gofer api")?
            .into_inner()
            .run;

        let mut task_executions = self
            .client
            .list_task_executions(&namespace, pipeline_id, run_id)
            .await
            .context("Could not successfully retrieve task executions from Gofer api")?
            .into_inner()
            .task_executions;

        task_executions.sort_by_key(|a| a.task.depends_on.len());

        let mut task_rows = vec![];

        for task in task_executions.iter() {
            let state_prefix: &str = match task.state {
                gofer_sdk::api::types::TaskExecutionState::Running => "Running for",
                gofer_sdk::api::types::TaskExecutionState::Waiting
                | gofer_sdk::api::types::TaskExecutionState::Processing => "Waiting for",
                _ => "Lasted",
            };

            let depends_on = dependencies(&task.task.depends_on);

            task_rows.push(vec![
                Cell::new(&task.task_id).fg(Color::Blue),
                Cell::new(
                    self.format_time(task.started)
                        .unwrap_or("Not yet".to_string()),
                ),
                Cell::new(format!(
                    "{} {}",
                    state_prefix,
                    duration(task.started as i64, task.ended as i64)
                )),
                Cell::new(task.state).fg(colorize_status_text_comfy(task.state)),
                Cell::new(task.status).fg(colorize_status_text_comfy(task.status)),
                Cell::new(if depends_on.is_empty() {
                    "Immediately".to_string()
                } else {
                    depends_on.join("\n")
                }),
            ]);
        }

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Initiated by {{ initiator_name }}
  {{ vertical_line }} Started {{ started }} and {{ duration }}
  {{ vertical_line }} Objects Expired: {{ objects_expired }}
  {%- if token_id %}
  {{ vertical_line }} Injected Token ID: {{ token_id }}
  {%- endif %}
  {%- if status_reason %}
  {{ vertical_line }} {{ status_message }}: {{ status_reason.reason }}: {{ status_reason.description }}
  {%- endif %}

  $ Task Executions:
  {%- for line in task_executions %}
  {{ line }}
  {%- endfor %}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &rail());
        context.insert("initiator_name", &run.initiator.user.blue().to_string());
        context.insert(
            "started",
            &self
                .format_time(run.started)
                .unwrap_or_else(|| "Not yet".to_string()),
        );
        context.insert("duration", &debug::run_duration(&run));
        context.insert("objects_expired", &run.store_objects_expired);
        context.insert("token_id", &run.token_id);
        context.insert(
            "task_executions",
            &rail_table(
                &["TASK", "STARTED", "DURATION", "STATE", "STATUS", "STARTS"],
                task_rows,
            ),
        );
        context.insert("status_reason", &run.status_reason);
        context.insert("status_message", &"Failure".red().to_string());

        let content = tera.render("main", &context)?;
        println!("{}", run_title(&run));
        println!("{}", content.trim_end());

        let mut run_ordered = task_executions.clone();
        debug::sort_by_run_order(&mut run_ordered);

        let mut problems: Vec<_> = run_ordered
            .iter()
            .filter(|task| {
                task.state == gofer_sdk::api::types::TaskExecutionState::Complete
                    && task.status != gofer_sdk::api::types::TaskExecutionStatus::Successful
            })
            .collect();
        problems.sort_by_key(|task| debug::problem_rank(task));

        if !problems.is_empty() {
            let task_width = problems
                .iter()
                .map(|task| task.task_id.chars().count())
                .max()
                .unwrap_or(0);

            let mut output = vec![String::new(), "  $ Problems:".to_string()];

            for task in problems {
                let mut line = format!(
                    "  {} {}  {}",
                    rail(),
                    format!("{:<task_width$}", task.task_id).blue(),
                    debug::status_text(task)
                );

                if task.status != gofer_sdk::api::types::TaskExecutionStatus::Skipped
                    && let Some(reason) = &task.status_reason
                {
                    line.push_str(&format!(": {}", reason.reason).dimmed().to_string());
                }

                output.push(line);
            }

            output.push(String::new());
            output.push(format!(
                "* Use '{}' for details.",
                format!("gofer run debug {} {}", run.pipeline_id, run.run_id).cyan()
            ));

            println!("{}", output.join("\n"));
        }

        Ok(())
    }

    pub async fn run_start(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        variables: Vec<String>,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let mut variable_map = HashMap::new();

        for var in variables {
            let split_var = var.split_once('=');
            match split_var {
                Some((key, value)) => {
                    variable_map.insert(key.into(), value.into());
                }
                None => {
                    bail!(
                        "malformed variable '{}'; must be in format <KEY>=<VALUE>",
                        var
                    );
                }
            }
        }

        let response = self
            .client
            .start_run(
                &namespace,
                pipeline_id,
                &gofer_sdk::api::types::StartRunRequest {
                    variables: variable_map,
                },
            )
            .await
            .context("Could not successfully start the pipeline run")?;

        success!(
            "Started new run {}",
            format!("#{}", response.run.run_id).blue()
        );
        println!(
            "{}",
            format!(
                "\n  View details of your new run: {}",
                format!(
                    "gofer fetch {} {}",
                    response.run.pipeline_id, response.run.run_id
                )
                .yellow()
            )
        );
        println!(
            "{}",
            format!(
                "  List all task executions: {}",
                format!(
                    "gofer fetch {} {} +",
                    response.run.pipeline_id, response.run.run_id
                )
                .yellow()
            )
        );
        Ok(())
    }

    pub async fn run_cancel(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .cancel_run(&namespace, pipeline_id, run_id)
            .await
            .context("Could not successfully cancel run")?;

        success!("run '{}' cancelled", run_id);
        Ok(())
    }
}
