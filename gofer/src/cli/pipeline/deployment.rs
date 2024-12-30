use crate::cli::{colorize_status_text, colorize_status_text_comfy, duration, Cli};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println};

#[derive(Debug, Args, Clone)]
pub struct DeploymentSubcommands {
    #[clap(subcommand)]
    pub command: DeploymentCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DeploymentCommands {
    /// List pipeline Deploymenturations.
    List {
        /// Pipeline Identifier.
        id: String,
    },

    /// Get pipeline Deploymenturation details.
    Get {
        /// Pipeline Identifier.
        id: String,

        /// Deployment Identifier.
        deployment_id: u64,
    },
}

impl Cli {
    pub async fn handle_pipeline_deployment_subcommands(
        &self,
        command: DeploymentSubcommands,
    ) -> Result<()> {
        let cmds = command.command;
        match cmds {
            DeploymentCommands::List { id } => {
                self.pipeline_deployment_list(command.namespace, &id).await
            }
            DeploymentCommands::Get { id, deployment_id } => {
                self.pipeline_deployment_get(command.namespace, &id, deployment_id)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn pipeline_deployment_list(
        &self,
        namespace_id: Option<String>,
        id: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let deployments = self
            .client
            .list_deployments(&namespace, id)
            .await
            .context("Could not successfully retrieve pipeline Deployments from Gofer api")?
            .into_inner()
            .deployments;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("versions")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("started")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("ended")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("status")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for deployment in deployments {
            table.add_row(vec![
                Cell::new(deployment.deployment_id).fg(Color::Green),
                Cell::new(format!(
                    "{} -> {}",
                    deployment.start_version, deployment.end_version
                )),
                Cell::new(
                    self.format_time(deployment.started)
                        .unwrap_or("Not Yet".to_string()),
                ),
                Cell::new(self.format_time(deployment.ended).unwrap_or("Never".into())),
                Cell::new(deployment.state).fg(colorize_status_text_comfy(deployment.state)),
                Cell::new(deployment.status).fg(colorize_status_text_comfy(deployment.status)),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn pipeline_deployment_get(
        &self,
        namespace_id: Option<String>,
        id: &str,
        deployment_id: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let deployment = self
            .client
            .get_deployment(&namespace, id, deployment_id)
            .await
            .context("Could not successfully retrieve deployment from Gofer api")?;

        const TEMPLATE: &str = r#"
    {{ vertical_line }} Start Version: v{{ start_version }}
    {{ vertical_line }} End Version: v{{ end_version }}
    {{ vertical_line }} Started: {{ started }} and ran for {{ duration }}
    {{ vertical_line }} Ended: {{ ended }}
    {{ vertical_line }} State: {{ state }}
    {{ vertical_line }} Status: {{ status }}

    {%- if status_reason %}
        Status Details:
        {{ vertical_line }} Reason: {{ status_reason.reason }}
        {{ vertical_line }} Description: {{ status_reason.description }}
    {%- endif %}
    {%- if logs is defined %}

    $ Logs:
    {%- for line in logs %}
    {{ line }}
    {%- endfor %}
    {%- endif %}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("start_version", &deployment.deployment.start_version);
        context.insert("end_version", &deployment.deployment.end_version);
        context.insert("vertical_line", &"│".magenta().to_string());
        context.insert(
            "duration",
            &duration(
                deployment.deployment.started as i64,
                deployment.deployment.ended as i64,
            ),
        );
        context.insert(
            "started",
            &self
                .format_time(deployment.deployment.started)
                .unwrap_or("Unknown".to_string()),
        );
        context.insert(
            "ended",
            &self
                .format_time(deployment.deployment.ended)
                .unwrap_or("Unknown".to_string()),
        );
        context.insert("state", &colorize_status_text(deployment.deployment.state));
        context.insert(
            "status",
            &colorize_status_text(deployment.deployment.status),
        );
        context.insert("status_reason", &deployment.deployment.status_reason);
        context.insert("logs", &deployment.deployment.logs);

        let content = tera.render("main", &context)?;
        print!("{}", content);
        Ok(())
    }
}
