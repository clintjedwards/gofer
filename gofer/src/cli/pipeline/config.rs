use crate::cli::{colorize_status_text, colorize_status_text_comfy, Cli};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};

#[derive(Debug, Args, Clone)]
pub struct ConfigSubcommands {
    #[clap(subcommand)]
    pub command: ConfigCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ConfigCommands {
    /// List pipeline configurations.
    List {
        /// Pipeline Identifier.
        id: String,
    },

    /// Get pipeline configuration details.
    Get {
        /// Pipeline Identifier.
        id: String,

        /// Config version.
        version: u64,
    },

    /// Delete a pipeline config
    ///
    /// You cannot remove currently live versions or the last pipeline configuration.
    Delete {
        /// Pipeline Identifier.
        id: String,

        /// Config version.
        version: u64,
    },
}

impl Cli {
    pub async fn handle_pipeline_config_subcommands(
        &self,
        command: ConfigSubcommands,
    ) -> Result<()> {
        let cmds = command.command;
        match cmds {
            ConfigCommands::List { id } => self.pipeline_config_list(command.namespace, &id).await,
            ConfigCommands::Get { id, version } => {
                self.pipeline_config_get(command.namespace, &id, version)
                    .await
            }
            ConfigCommands::Delete { id, version } => {
                self.pipeline_config_delete(command.namespace, &id, version)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn pipeline_config_list(&self, namespace_id: Option<String>, id: &str) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let configs = self
            .client
            .list_configs(&namespace, id)
            .await
            .context("Could not successfully retrieve pipeline configs from Gofer api")?
            .into_inner()
            .configs;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("version")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("registered")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("deprecated")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for config in configs {
            table.add_row(vec![
                Cell::new(config.version).fg(Color::Green),
                Cell::new(config.state).fg(colorize_status_text_comfy(config.state)),
                Cell::new(
                    self.format_time(config.registered)
                        .unwrap_or("Unknown".to_string()),
                ),
                Cell::new(
                    self.format_time(config.deprecated)
                        .unwrap_or("Never".into()),
                ),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn pipeline_config_get(
        &self,
        namespace_id: Option<String>,
        id: &str,
        version: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let config = self
            .client
            .get_config(&namespace, id, version as i64)
            .await
            .context("Could not successfully retrieve config from Gofer api")?;

        const TEMPLATE: &str = r#"
  {%- if tasks %}
  🗒 Tasks:
    {%- for task_id, task in tasks %}
    • {{ task_id -}}
    {%- if task.depends_on -%}
        {%- for dependant in task.depends_on %}
        - {{ dependant -}}
        {%- endfor %}
    {%- endif %}
    {%- endfor %}
  {%- endif %}

  Registered {{ registered }} | Deprecated {{ deprecated }}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("version", &config.config.version);
        context.insert("parallelism", &config.config.parallelism);
        context.insert("description", &config.config.description);
        context.insert("tasks", &config.config.tasks);
        context.insert(
            "registered",
            &self
                .format_time(config.config.registered)
                .unwrap_or("Unknown".to_string()),
        );
        context.insert(
            "deprecated",
            &self
                .format_time(config.config.deprecated)
                .unwrap_or("Never".to_string()),
        );

        let content = tera.render("main", &context)?;
        println!(
            "[{}] {} :: {}",
            config.config.pipeline_id.blue(),
            config.config.name,
            colorize_status_text(config.config.state)
        );
        print!("\n");
        println!("  Version: {}", &config.config.version);
        println!("  Parallelism: {}", &config.config.parallelism);
        print!("\n");
        println!("  {}", &config.config.description);

        print!("{}", content);
        Ok(())
    }

    pub async fn pipeline_config_delete(
        &self,
        namespace_id: Option<String>,
        id: &str,
        version: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .delete_config(&namespace, id, version as i64)
            .await
            .context("Could not delete config")?;

        success!("Successfully removed config version '{}'", version);

        Ok(())
    }
}
