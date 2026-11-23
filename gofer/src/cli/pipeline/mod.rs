mod config;
mod deployment;
mod object;

use crate::cli::{
    Cli, colorize_status_text, colorize_status_text_comfy, duration, rail, rail_table, tasks_table,
};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{println, success};

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
    ///
    /// Runs that are already going finish normally; runs still waiting in the queue are cancelled. By default the pipeline
    /// stays disabled until 'gofer pipeline enable'. Use '--for' or '--until' to have it turn itself back on.
    ///
    /// Disabling a pipeline that's already disabled replaces when it turns back on.
    ///
    /// ex. gofer pipeline disable simple --for 7d
    /// ex. gofer pipeline disable simple --until 2026-12-01
    Disable {
        /// Pipeline Identifier.
        id: String,

        /// How long to disable the pipeline for. ex. 12h, 7d, "1d 6h"
        #[arg(long = "for", value_name = "DURATION", value_parser = humantime::parse_duration, conflicts_with = "until")]
        duration: Option<std::time::Duration>,

        /// The day the pipeline turns back on, as YYYY-MM-DD. It turns on at the start of that day in your local time.
        #[arg(long, value_name = "DATE")]
        until: Option<NaiveDate>,
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
    /// ex. gofer pipeline subscribe simple interval every-5-minutes -s every="5m"
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

    /// Stop a subscription from triggering runs, without removing it.
    ///
    /// The subscription and its settings are kept, so 'gofer pipeline enable-subscription' turns it back on as it was.
    ///
    /// ex. gofer pipeline disable-subscription simple interval every-5-minutes
    DisableSubscription {
        /// Pipeline Identifier.
        id: String,

        /// The extension id the subscription is for.
        extension_id: String,

        /// The subscription's name.
        label: String,
    },

    /// Turn a disabled subscription back on.
    EnableSubscription {
        /// Pipeline Identifier.
        id: String,

        /// The extension id the subscription is for.
        extension_id: String,

        /// The subscription's name.
        label: String,
    },

    /// Manage pipeline object store.
    Object(object::ObjectSubcommands),

    /// Manage pipeline configs/manifests.
    Config(config::ConfigSubcommands),

    /// Manage pipeline deployments
    Deployment(deployment::DeploymentSubcommands),
}

/// When a disabled pipeline should turn back on, or None if it should stay disabled.
fn disable_end(
    duration: Option<std::time::Duration>,
    until: Option<NaiveDate>,
    now: DateTime<Local>,
) -> Result<Option<DateTime<Local>>> {
    let end = match (duration, until) {
        (Some(duration), _) => chrono::Duration::from_std(duration)
            .ok()
            .and_then(|duration| now.checked_add_signed(duration))
            .context("Duration is too long")?,
        // A clock change can skip midnight in some timezones.
        (None, Some(until)) => until
            .and_time(NaiveTime::MIN)
            .and_local_timezone(Local)
            .earliest()
            .with_context(|| format!("{until} doesn't have a midnight in your timezone"))?,
        (None, None) => return Ok(None),
    };

    if end <= now {
        bail!("The pipeline has to be disabled until some time in the future");
    }

    Ok(Some(end))
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
            PipelineCommands::Disable {
                id,
                duration,
                until,
            } => {
                self.pipeline_disable(command.namespace, &id, duration, until)
                    .await
            }
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
            PipelineCommands::DisableSubscription {
                id,
                extension_id,
                label,
            } => {
                self.pipeline_set_subscription_status(
                    command.namespace,
                    &id,
                    &extension_id,
                    &label,
                    false,
                )
                .await
            }
            PipelineCommands::EnableSubscription {
                id,
                extension_id,
                label,
            } => {
                self.pipeline_set_subscription_status(
                    command.namespace,
                    &id,
                    &extension_id,
                    &label,
                    true,
                )
                .await
            }
            PipelineCommands::Object(object) => {
                self.handle_pipeline_object_subcommands(object).await
            }
            PipelineCommands::Config(config) => {
                self.handle_pipeline_config_subcommands(config).await
            }
            PipelineCommands::Deployment(deployment) => {
                self.handle_pipeline_deployment_subcommands(deployment)
                    .await
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
            .load_style(comfy_table::presets::ASCII_MARKDOWN)
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
                .list_runs(
                    &namespace,
                    &pipeline.pipeline_id,
                    Some(1),
                    None,
                    Some(true),
                    None,
                )
                .await
                .context("Could not successfully retrieve last run from Gofer api")?
                .into_inner()
                .runs;

            let last_run_time = match last_run.first() {
                Some(last_run) => last_run.created,
                None => 0,
            };

            table.add_row(vec![
                Cell::new(pipeline.pipeline_id).fg(Color::Green),
                Cell::new(pipeline.state).fg(colorize_status_text_comfy(pipeline.state)),
                Cell::new(
                    self.format_time(pipeline.created)
                        .unwrap_or("Unknown".to_string()),
                ),
                Cell::new(
                    self.format_time(last_run_time)
                        .unwrap_or("Never".to_string()),
                ),
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
            .get_config(&namespace, id, 0, None)
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
            .list_runs(&namespace, id, Some(5), None, Some(true), None)
            .await
            .context("Could not successfully retrieve recent runs for pipeline from Gofer api")?
            .into_inner()
            .runs;

        let last_run_time = match recent_runs.first() {
            Some(last_run) => last_run.created,
            None => 0,
        };

        let mut run_rows = vec![];

        recent_runs.truncate(10);
        for run in recent_runs {
            let state_prefix = if run.state == gofer_sdk::api::types::RunState::Running {
                "Running for"
            } else {
                "Lasted"
            };

            run_rows.push(vec![
                Cell::new(format!("#{}", run.run_id)).fg(Color::Blue),
                Cell::new(format!(
                    "{} by {}",
                    self.format_time(run.created).unwrap_or("Never".into()),
                    super::run::initiator_text(&run.initiator)
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

        let mut subscriptions = subscriptions;
        subscriptions.sort_by(|a, b| {
            (&a.subscription_id, &a.extension_id).cmp(&(&b.subscription_id, &b.extension_id))
        });

        // A subscription in error does nothing at all, so this is often the only sign of why something a pipeline
        // relied on, like scheduled runs, stopped happening.
        let subscription_rows = subscriptions
            .into_iter()
            .map(|subscription| {
                let reason = subscription
                    .status_reason
                    .map(|reason| reason.description)
                    .unwrap_or_default();

                vec![
                    Cell::new(subscription.subscription_id).fg(Color::Blue),
                    Cell::new(subscription.extension_id),
                    Cell::new(subscription.status)
                        .fg(colorize_status_text_comfy(subscription.status)),
                    Cell::new(reason),
                ]
            })
            .collect();

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Created {{ created }}
  {{ vertical_line }} Last run {{ last_run }}
  {%- if enables %}
  {{ vertical_line }} Turns back on {{ enables }}
  {%- endif %}
  {%- if allow_runs_from %}
  {{ vertical_line }} Allows runs from: {{ allow_runs_from }}
  {%- endif %}

  $ Recent Runs:
  {%- for line in recent_runs %}
  {{ line }}
  {%- endfor %}

  $ Tasks:
  {%- for line in tasks %}
  {{ line }}
  {%- endfor %}

  $ Subscriptions:
  {%- for line in subscriptions %}
  {{ line }}
  {%- endfor %}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &rail());
        context.insert(
            "allow_runs_from",
            &pipeline_config.allow_runs_from.join(", "),
        );
        context.insert(
            "recent_runs",
            &rail_table(&["RUN", "STARTED", "DURATION", "STATE", "STATUS"], run_rows),
        );
        context.insert("tasks", &tasks_table(&pipeline_config.tasks));
        context.insert(
            "subscriptions",
            &rail_table(
                &["SUBSCRIPTION", "EXTENSION", "STATUS", "REASON"],
                subscription_rows,
            ),
        );
        context.insert(
            "created",
            &self
                .format_time(pipeline_metadata.created)
                .unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert(
            "last_run",
            &self.format_time(last_run_time).unwrap_or("Never".into()),
        );

        context.insert(
            "enables",
            &self.format_time(pipeline_metadata.disabled_until),
        );

        let content = tera.render("main", &context)?;
        println!(
            "  Pipeline {} ({}) :: {}",
            pipeline_metadata.pipeline_id.cyan(),
            &pipeline_config.name,
            colorize_status_text(pipeline_metadata.state)
        );
        if !pipeline_config.description.is_empty() {
            println!("");
            println!("  {}", pipeline_config.description);
        }
        println!("{}", content.trim_end());
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
                    disabled_until: None,
                },
            )
            .await
            .context("Could not successfully enable pipeline from Gofer api")?;

        success!("pipeline '{}' enabled!", id);
        Ok(())
    }

    pub async fn pipeline_disable(
        &self,
        namespace_id: Option<String>,
        id: &str,
        duration: Option<std::time::Duration>,
        until: Option<NaiveDate>,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let disabled_until = disable_end(duration, until, Local::now())?;

        self.client
            .update_pipeline(
                &namespace,
                id,
                &gofer_sdk::api::types::UpdatePipelineRequest {
                    state: Some(gofer_sdk::api::types::PipelineState::Disabled),
                    disabled_until: disabled_until.map(|end| end.timestamp_millis() as u64),
                },
            )
            .await
            .context("Could not successfully disable pipeline from Gofer api")?;

        match disabled_until {
            Some(end) => success!(
                "pipeline '{}' disabled until {}!",
                id,
                end.format("%a %Y-%m-%d %H:%M %:z")
            ),
            None => success!("pipeline '{}' disabled!", id),
        }
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
                    bail!(
                        "Malformed setting string '{key_value_pair_str}'; Must be in format: <KEY>=<VALUE>"
                    );
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

    pub async fn pipeline_set_subscription_status(
        &self,
        namespace_id: Option<String>,
        id: &str,
        extension_id: &str,
        label: &str,
        enabled: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let status = if enabled {
            gofer_sdk::api::types::UpdateSubscriptionStatus::Active
        } else {
            gofer_sdk::api::types::UpdateSubscriptionStatus::Disabled
        };

        // Turning a subscription back on resends it to the extension, which can reject it, so its message is shown.
        self.client
            .update_subscription(
                &namespace,
                id,
                extension_id,
                label,
                &gofer_sdk::api::types::UpdateSubscriptionRequest {
                    status: Some(status),
                },
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Could not update subscription '{label}'; {}",
                    super::api_error_message(e)
                )
            })?;

        success!(
            "subscription '{}' on pipeline '{}' {}!",
            label,
            id,
            if enabled { "enabled" } else { "disabled" }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn local_midnight(year: i32, month: u32, day: u32) -> DateTime<Local> {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_time(NaiveTime::MIN)
            .and_local_timezone(Local)
            .earliest()
            .unwrap()
    }

    #[test]
    fn disable_end_handles_each_flag() {
        let now = local_midnight(2026, 11, 24);
        let week = std::time::Duration::from_secs(7 * 24 * 60 * 60);
        let date = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();

        assert_eq!(disable_end(None, None, now).unwrap(), None);
        assert_eq!(
            disable_end(Some(week), None, now).unwrap(),
            Some(now + chrono::Duration::days(7))
        );
        assert_eq!(
            disable_end(None, Some(date), now)
                .unwrap()
                .unwrap()
                .naive_local(),
            date.and_time(NaiveTime::MIN)
        );
    }

    #[test]
    fn disable_end_rejects_bad_ends() {
        let now = local_midnight(2026, 11, 24);
        let long_ago = NaiveDate::from_ymd_opt(1960, 1, 1).unwrap();
        let today = now.date_naive();

        assert!(disable_end(Some(std::time::Duration::MAX), None, now).is_err());
        assert!(disable_end(Some(std::time::Duration::ZERO), None, now).is_err());
        assert!(disable_end(None, Some(long_ago), now).is_err());
        assert!(disable_end(None, Some(today), now).is_err());
    }
}
