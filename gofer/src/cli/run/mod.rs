mod object;

use crate::cli::{colorize_status_text, colorize_status_text_comfy, dependencies, duration, Cli};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};
use serde::Serialize;
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
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
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

        task_executions.sort_by(|a, b| a.task.depends_on.len().cmp(&b.task.depends_on.len()));

        let mut task_table = comfy_table::Table::new();
        task_table
            .load_preset(comfy_table::presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_style(comfy_table::TableComponent::VerticalLines, ':');

        for task in task_executions.iter() {
            let state_prefix: &str = match task.state {
                gofer_sdk::api::types::TaskExecutionState::Running => "Running for",
                gofer_sdk::api::types::TaskExecutionState::Waiting
                | gofer_sdk::api::types::TaskExecutionState::Processing => "Waiting for",
                _ => "Lasted",
            };

            task_table.add_row(vec![
                Cell::new(format!("• {}", task.task_id.clone())).fg(Color::Blue),
                Cell::new(format!(
                    "Started {}",
                    self.format_time(task.started)
                        .unwrap_or("Not yet".to_string())
                )),
                Cell::new(format!(
                    "{} {}",
                    state_prefix,
                    duration(task.started as i64, task.ended as i64)
                )),
                Cell::new(task.state).fg(colorize_status_text_comfy(task.state)),
                Cell::new(task.status).fg(colorize_status_text_comfy(task.status)),
            ]);
        }

        #[derive(Serialize)]
        struct TaskData {
            line: String,
            depends_on: Vec<String>,
        }

        let mut task_data = vec![];
        let task_table_lines = task_table
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<String>>();

        for (index, task) in task_executions.into_iter().enumerate() {
            task_data.push(TaskData {
                line: task_table_lines[index].clone(),
                depends_on: dependencies(&task.task.depends_on),
            })
        }

        const TEMPLATE: &str = r#"
  Initiated by {{ initiator_name }} {{ started }} and ran for {{ duration }}
  {%- if task_executions is defined and task_executions | length > 0 %}

  {%- if status_reason %}
  
  {{status_message}}: {{ status_reason.reason }}: {{ status_reason.description }}
  {%- endif %}

  🗒 Task Executions
    {%- for task in task_executions %}
    {{ task.line }}
    {%- if task.depends_on is defined and task.depends_on | length > 0 %}
      {%- for dependant in task.depends_on %}
      - {{ dependant }}
      {%- endfor -%}
    {%- endif -%}
    {%- endfor %}
  {%- endif %}

  Objects Expired: {{ objects_expired }}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("initiator_name", &run.initiator.user.cyan().to_string());
        context.insert(
            "started",
            &self
                .format_time(run.started)
                .unwrap_or_else(|| "Not yet".to_string()),
        );
        context.insert("duration", &duration(run.started as i64, run.ended as i64));
        context.insert("objects_expired", &run.store_objects_expired);
        context.insert("task_executions", &task_data);
        context.insert("status_reason", &run.status_reason);
        context.insert("status_message", &"Failure".red().to_string());

        let content = tera.render("main", &context)?;
        println!(
            "Run {} for Pipeline {} ({}) :: {} :: {}",
            format!("#{}", run.run_id).blue(),
            run.pipeline_id.blue(),
            format!("v{}", run.pipeline_config_version),
            colorize_status_text(run.state),
            colorize_status_text(run.status)
        );
        print!("{}", content);
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
                    "gofer run get {} {}",
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
                    "gofer task list {} {}",
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
