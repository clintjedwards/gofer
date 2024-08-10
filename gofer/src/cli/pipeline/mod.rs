mod config;
mod object;

use crate::cli::{
    colorize_status_text, colorize_status_text_comfy, dependencies, duration,
    humanize_relative_duration, Cli,
};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};
use serde::Serialize;

#[derive(Debug, Args, Clone)]
pub struct PipelineSubcommands {
    #[clap(subcommand)]
    pub command: PipelineCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum PipelineCommands {
    /// List all pipelines.
    List,

    /// Get details on a single pipeline.
    Get {
        /// Pipeline Identifier.
        id: String,
    },

    /// Start a new run.
    Run {
        /// Pipeline Identifier.
        id: String,

        /// Optional environment variables to pass to your run. Format: Key=Value
        #[arg(short, long)]
        variable: Vec<String>,
    },

    /// Move a pipeline from a disabled state to a enabled.
    Enable {
        /// Pipeline Identifier.
        id: String,
    },

    /// Stop a pipeline from being able to run.
    Disable {
        /// Pipeline Identifier.
        id: String,
    },

    /// Delete a pipeline permanently.
    Delete {
        /// Pipeline Identifier.
        id: String,
    },

    /// Subscribe a pipeline to an extension.
    ///
    /// Extensions extend the functionality of your pipeline, allowing it to do many things like automatically run based on
    /// some event or post to Slack. To take advantage of this first your pipeline has to "subscribe" to a particular extension.
    ///
    /// You can find the current extensions your Gofer instance supports by using the 'gofer extension list' command.
    ///
    /// Usually extensions will require some type of configuration for each pipeline subscribed. You can pass this configuration
    /// by using the '--setting' flag. You can find a list of the settings for your extension needs by reading the documentation.
    ///
    /// For example, the "interval" extension requires the subscribing pipeline to specify which time interval it would like to
    ///  be run on. The setting is called "every". So one might subscribe to the interval extension like so:
    ///
    /// ex. gofer pipeline subscribe simple interval every_5_seconds -s every="5s"
    Subscribe {
        /// Pipeline Identifier.
        id: String,

        /// The extension id to subscribe to.
        extension_id: String,

        /// A name for this subscription.
        label: String,

        /// Input extension setting
        #[arg(short, long)]
        setting: Vec<String>,
    },

    /// Remove a subscription from an extension.
    Unsubscribe {
        /// Pipeline Identifier.
        id: String,

        /// The extension id to unsubscribe from.
        extension_id: String,

        /// A name for this subscription.
        label: String,
    },

    /// Manage pipeline object store.
    Object(object::ObjectSubcommands),

    /// Manage pipeline configs/manifests.
    Config(config::ConfigSubcommands),
}

#[derive(Serialize)]
struct TaskData {
    name: String,
    depends_on: Vec<String>,
    num_items: usize,
}

impl Cli {
    pub async fn handle_pipeline_subcommands(&self, command: PipelineSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            PipelineCommands::List => self.pipeline_list(command.namespace).await,
            PipelineCommands::Get { id } => self.pipeline_get(command.namespace, &id).await,
            PipelineCommands::Run { id, variable } => {
                self.pipeline_run(command.namespace, &id, variable).await
            }
            PipelineCommands::Enable { id } => self.pipeline_enable(command.namespace, &id).await,
            PipelineCommands::Disable { id } => self.pipeline_disable(command.namespace, &id).await,
            PipelineCommands::Delete { id } => self.pipeline_delete(command.namespace, &id).await,
            PipelineCommands::Subscribe {
                id,
                extension_id,
                label,
                setting,
            } => {
                self.pipeline_subscribe(command.namespace, &id, &extension_id, &label, setting)
                    .await
            }
            PipelineCommands::Unsubscribe {
                id,
                extension_id,
                label,
            } => {
                self.pipeline_unsubscribe(command.namespace, &id, &extension_id, &label)
                    .await
            }
            PipelineCommands::Object(object) => {
                self.handle_pipeline_object_subcommands(object).await
            }
            PipelineCommands::Config(config) => {
                self.handle_pipeline_config_subcommands(config).await
            }
        }
    }
}

impl Cli {
    pub async fn pipeline_list(&self, namespace_id: Option<String>) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let pipelines = self
            .client
            .list_pipelines(&namespace)
            .await
            .context("Could not successfully retrieve pipelines from Gofer api")?
            .into_inner()
            .pipelines;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("created")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("last run")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for pipeline in pipelines {
            let last_run = self
                .client
                .list_runs(&namespace, &pipeline.pipeline_id, Some(1), None, Some(true))
                .await
                .context("Could not successfully retrieve last run from Gofer api")?
                .into_inner()
                .runs;

            let last_run_time = match last_run.first() {
                Some(last_run) => last_run.started,
                None => 0,
            };

            table.add_row(vec![
                Cell::new(pipeline.pipeline_id).fg(Color::Green),
                Cell::new(pipeline.state).fg(colorize_status_text_comfy(pipeline.state)),
                Cell::new(
                    humanize_relative_duration(pipeline.created).unwrap_or("Unknown".to_string()),
                ),
                Cell::new(humanize_relative_duration(last_run_time).unwrap_or("Never".to_string())),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn pipeline_get(&self, namespace_id: Option<String>, id: &str) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let pipeline_metadata = self
            .client
            .get_pipeline(&namespace, id)
            .await
            .context("Could not successfully retrieve pipeline from Gofer api")?
            .into_inner()
            .pipeline;

        let pipeline_config = self
            .client
            .get_config(&namespace, id, 0)
            .await
            .context("Could not successfully retrieve pipeline config from Gofer api")?
            .into_inner()
            .config;

        let subscriptions = self
            .client
            .list_subscriptions(&namespace, id)
            .await
            .context("Could not successfully retrieve subscriptions for pipeline from Gofer api")?
            .into_inner()
            .subscriptions;

        let mut recent_runs = self
            .client
            .list_runs(&namespace, id, Some(5), None, Some(true))
            .await
            .context("Could not successfully retrieve recent runs for pipeline from Gofer api")?
            .into_inner()
            .runs;

        let last_run_time = match recent_runs.first() {
            Some(last_run) => last_run.started,
            None => 0,
        };

        let mut run_table = comfy_table::Table::new();

        recent_runs.truncate(10);
        for run in recent_runs {
            let state_prefix = if run.state == gofer_sdk::api::types::RunState::Running {
                "Running for"
            } else {
                "Lasted"
            };

            run_table
                .load_preset(comfy_table::presets::NOTHING)
                .set_content_arrangement(ContentArrangement::Dynamic);

            run_table.add_row(vec![
                Cell::new(format!("{}:", run.run_id)).fg(Color::Blue),
                Cell::new(format!(
                    "{} by {}",
                    humanize_relative_duration(run.started).unwrap_or("Never".into()),
                    run.initiator.id
                )),
                Cell::new(format!(
                    "{} {}",
                    state_prefix,
                    duration(run.started as i64, run.ended as i64)
                )),
                Cell::new(run.state).fg(colorize_status_text_comfy(run.state)),
                Cell::new(run.status).fg(colorize_status_text_comfy(run.status)),
            ]);
        }

        let mut subscription_table_data = vec![];

        for subscription in subscriptions {
            subscription_table_data.push(vec![
                "⟳".into(),
                subscription.subscription_id,
                subscription.extension_id,
            ]);
        }

        subscription_table_data.sort();

        let mut subscription_table = comfy_table::Table::new();

        for subscription in subscription_table_data {
            subscription_table
                .load_preset(comfy_table::presets::NOTHING)
                .set_content_arrangement(ContentArrangement::Dynamic);

            subscription_table.add_row(vec![
                Cell::new(subscription[0].clone()),
                Cell::new(subscription[1].clone()).fg(Color::Blue),
                Cell::new(subscription[2].clone()),
            ]);
        }

        let mut tasks = vec![];

        for task in pipeline_config.tasks.values() {
            tasks.push(TaskData {
                name: task.id.blue().to_string(),
                depends_on: dependencies(&task.depends_on),
                num_items: task.depends_on.len(), // We use this for sorting purposes.
            });
        }

        tasks.sort_by_key(|task| task.num_items);

        const TEMPLATE: &str = r#"{%- if has_recent_runs %}
  📦 Recent Runs
    {%- for line in recent_runs %}
    {{ line }}
    {%- endfor %}
  {% endif %}
  {%- if tasks %}
  🗒 Tasks:
    {%- for task in tasks %}
    • {{ task.name }}
    {%- if task.depends_on -%}
    {%- for dependant in task.depends_on %}
      - {{ dependant }}
    {%- endfor -%}
    {%- endif -%}
    {%- endfor -%}
  {%- endif %}

  {%- if has_subscriptions %}
    🗘 Extension Subscriptions:
      {{ subscriptions }}
  {%- endif %}

  Created {{ created }} | Last Run {{ last_run }}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("has_recent_runs", &!run_table.is_empty());
        context.insert(
            "recent_runs",
            &run_table
                .lines()
                .map(|line| line.to_string())
                .collect::<Vec<String>>(),
        );
        context.insert("tasks", &tasks);
        context.insert("has_subscriptions", &!subscription_table.is_empty());
        context.insert("subscriptions", &subscription_table.to_string());
        context.insert(
            "created",
            &humanize_relative_duration(pipeline_metadata.created)
                .unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert(
            "last_run",
            &humanize_relative_duration(last_run_time).unwrap_or("Never".into()),
        );

        let content = tera.render("main", &context)?;
        println!(
            "[{}] {} :: {}",
            &pipeline_metadata.pipeline_id,
            &pipeline_config.name,
            colorize_status_text(pipeline_metadata.state)
        );
        print!("\n");
        print!("{}", &pipeline_config.description);
        print!("{}", content);
        Ok(())
    }

    pub async fn pipeline_run(
        &self,
        namespace_id: Option<String>,
        id: &str,
        variables: Vec<String>,
    ) -> Result<()> {
        self.run_start(namespace_id, id, variables).await
    }

    pub async fn pipeline_enable(&self, namespace_id: Option<String>, id: &str) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .update_pipeline(
                &namespace,
                id,
                &gofer_sdk::api::types::UpdatePipelineRequest {
                    state: Some(gofer_sdk::api::types::PipelineState::Active),
                },
            )
            .await
            .context("Could not successfully enable pipeline from Gofer api")?;

        success!("pipeline '{}' enabled!", id);
        Ok(())
    }

    pub async fn pipeline_disable(&self, namespace_id: Option<String>, id: &str) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .update_pipeline(
                &namespace,
                id,
                &gofer_sdk::api::types::UpdatePipelineRequest {
                    state: Some(gofer_sdk::api::types::PipelineState::Disabled),
                },
            )
            .await
            .context("Could not successfully enable pipeline from Gofer api")?;

        success!("pipeline '{}' disabled!", id);
        Ok(())
    }

    pub async fn pipeline_subscribe(
        &self,
        namespace_id: Option<String>,
        id: &str,
        extension_id: &str,
        label: &str,
        settings: Vec<String>,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let mut settings_map = std::collections::HashMap::new();

        for key_value_pair_str in settings {
            let (key, value) = match key_value_pair_str.split_once('=') {
                Some((key, value)) => (key.to_string(), value.to_string()),
                None => {
                    bail!("Malformed setting string '{key_value_pair_str}'; Must be in format: <KEY>=<VALUE>");
                }
            };

            settings_map.insert(key, value);
        }

        self.client
            .create_subscription(
                &namespace,
                id,
                &gofer_sdk::api::types::CreateSubscriptionRequest {
                    extension_id: extension_id.into(),
                    subscription_id: label.into(),
                    settings: settings_map,
                },
            )
            .await
            .context("Could not successfully subscribe pipeline to extension")?;

        success!(
            "pipeline '{}' subscribed to extension '{}'!",
            id,
            extension_id
        );
        Ok(())
    }

    pub async fn pipeline_unsubscribe(
        &self,
        namespace_id: Option<String>,
        id: &str,
        extension_id: &str,
        label: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .delete_subscription(&namespace, id, extension_id, label)
            .await
            .context("Could not successfully unsubscribe pipeline from extension")?;

        success!(
            "pipeline '{}' unsubscribed from extension '{}'!",
            id,
            extension_id
        );
        Ok(())
    }

    pub async fn pipeline_delete(&self, namespace_id: Option<String>, id: &str) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .delete_pipeline(&namespace, id)
            .await
            .context("Could not successfully remove pipeline from Gofer api")?;

        success!("pipeline '{}' deleted!", id);
        Ok(())
    }
}
