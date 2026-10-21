use crate::cli::{
    Cli, api_error_message, colorize_status_text, colorize_status_text_comfy, rail, tasks_table,
};
use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use gofer_sdk::api::types::ConfigState;
use polyfmt::{println, success};

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

    /// Deploy an existing pipeline config version.
    ///
    /// Makes that version the one new runs use, replacing the live one. Mostly useful for rolling back to an
    /// earlier version; `gofer up` already deploys new versions as it registers them.
    Deploy {
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
            ConfigCommands::Deploy { id, version } => {
                self.pipeline_config_deploy(command.namespace, &id, version)
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
            .list_configs(&namespace, id, None)
            .await
            .context("Could not successfully retrieve pipeline configs from Gofer api")?
            .into_inner()
            .configs;

        let mut table = comfy_table::Table::new();
        table
            .load_style(comfy_table::presets::ASCII_MARKDOWN)
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
            .get_config(&namespace, id, version as i64, None)
            .await
            .context("Could not successfully retrieve config from Gofer api")?;

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Version: {{ version }}
  {{ vertical_line }} Parallelism: {{ parallelism }}
  {{ vertical_line }} Registered {{ registered }}
  {{ vertical_line }} Deprecated {{ deprecated }}

  $ Tasks:
  {%- for line in tasks %}
  {{ line }}
  {%- endfor %}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &rail());
        context.insert("version", &format!("v{}", config.config.version));
        context.insert("parallelism", &config.config.parallelism);
        context.insert("tasks", &tasks_table(&config.config.tasks));
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
            "  Pipeline Config {} ({}) :: {}",
            config.config.pipeline_id.cyan(),
            config.config.name,
            colorize_status_text(config.config.state)
        );
        if !config.config.description.is_empty() {
            println!("");
            println!("  {}", config.config.description);
        }
        println!("{}", content.trim_end());
        Ok(())
    }

    pub async fn pipeline_config_deploy(
        &self,
        namespace_id: Option<String>,
        id: &str,
        version: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        // Version 0 means "latest" when getting a config, which isn't something you'd mean to deploy.
        if version == 0 {
            bail!("Config versions start at 1");
        }

        let config = self
            .client
            .get_config(&namespace, id, version as i64, None)
            .await
            .map_err(|e| anyhow!("Could not get config v{version}; {}", api_error_message(e)))?
            .into_inner()
            .config;

        // Deploying the live version again would only add an empty deployment to the history.
        if config.state == ConfigState::Live {
            success!(
                "{} is already live; nothing to deploy",
                format!("v{version}").magenta()
            );
            return Ok(());
        }

        let deployment = self
            .client
            .deploy_config(&namespace, id, version as i64)
            .await
            .map_err(|e| {
                anyhow!(
                    "Could not deploy config v{version}; {}",
                    api_error_message(e)
                )
            })?
            .into_inner()
            .deployment;

        success!(
            "Deployed {} -> {}",
            format!("v{}", deployment.start_version).magenta(),
            format!("v{}", deployment.end_version).magenta()
        );
        println!(
            "  View the deployment: {}",
            format!(
                "gofer pipeline deployment get {id} {}",
                deployment.deployment_id
            )
            .cyan()
        );

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
