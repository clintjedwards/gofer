use crate::api::extensions::reconcile::{Manifest, default_manifest, fetch_manifest};
use crate::cli::{Cli, colorize_status_text, colorize_status_text_comfy, rail, rail_table};
use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement, presets::ASCII_MARKDOWN};
use futures::StreamExt;
use gofer_sdk::api::types::{ApplyExtensionRequest, PlanAction, PlannedExtension};
use polyfmt::{error, println, question, success, warning};
use std::collections::BTreeSet;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[derive(Debug, Args, Clone)]
pub struct ExtensionSubcommands {
    #[clap(subcommand)]
    pub command: ExtensionCommands,
}

/// Extensions are installed and configured in Gofer's server config under `[[extensions.install]]`. These commands
/// let you inspect them and roll config changes out to the running server.
#[derive(Debug, Subcommand, Clone)]
pub enum ExtensionCommands {
    /// Get a specific extension by id.
    Get {
        /// Extension Identifier.
        id: String,
    },

    /// List all extensions.
    List,

    /// Bring the running extensions in line with Gofer's config.
    ///
    /// Gofer rereads its config file and shows what would change for each extension: installs, upgrades, setting
    /// changes, rotated secrets, and extensions that were disabled or removed. You then confirm each change one at a
    /// time. Applying a change restarts that extension, and if the new version won't start you can put back the
    /// version that was running before.
    ///
    /// Restarting Gofer applies everything in one go instead.
    Reload {
        /// Apply every change without asking. Extensions that fail to start are reverted automatically.
        #[arg(short, long, default_value = "false")]
        yes: bool,

        /// Only show what would change.
        #[arg(long, default_value = "false")]
        dry_run: bool,
    },

    /// Print a ready-to-paste [[extensions.install]] block for an extension's manifest.
    ///
    /// The block lists every setting the extension takes along with its documentation. Settings that hold secrets
    /// point at a global secret, and the command to create that secret is printed alongside.
    ///
    /// Ex. gofer extension manifest interval
    /// Ex. gofer extension manifest https://example.com/my_extension/manifest.toml
    Manifest {
        /// The name of one of Gofer's default extensions (cron, interval), whose manifest is read from your Gofer
        /// server, or an https URL or local path to any other extension's manifest.
        location: String,

        /// The id to give the extension. Defaults to the default extension's name, or "my-extension".
        #[arg(long)]
        id: Option<String>,
    },

    /// Permanently delete an extension that's no longer in Gofer's config.
    ///
    /// Removing an extension from the config only stops it; its pipeline subscriptions and stored objects are kept
    /// in case you add it back. Purging deletes those for good.
    Purge {
        /// Extension Identifier.
        id: String,

        /// Don't ask for confirmation.
        #[arg(short, long, default_value = "false")]
        yes: bool,
    },

    /// Return logs for extension by id.
    Logs {
        /// Extension Identifier.
        id: String,
    },

    /// Print debug information for the target extension.
    ///
    /// Every extension has a debug endpoint that returns some information that the extension is keeping. It can be
    /// helpful when debugging an extension to dump this information.
    Debug {
        /// Extension Identifier.
        id: String,
    },
}

impl Cli {
    pub async fn handle_extension_subcommands(&self, command: ExtensionSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            ExtensionCommands::Get { id } => self.extension_get(&id).await,
            ExtensionCommands::List => self.extension_list().await,
            ExtensionCommands::Reload { yes, dry_run } => self.extension_reload(yes, dry_run).await,
            ExtensionCommands::Manifest { location, id } => {
                self.extension_manifest(&location, id.as_deref()).await
            }
            ExtensionCommands::Purge { id, yes } => self.extension_purge(&id, yes).await,
            ExtensionCommands::Logs { id } => self.extension_logs(&id).await,
            ExtensionCommands::Debug { id } => self.extension_debug(&id).await,
        }
    }
}

fn params_table(params: &[gofer_sdk::api::types::Parameter]) -> Vec<String> {
    rail_table(
        &["KEY", "REQUIRED", "DESCRIPTION"],
        params
            .iter()
            .map(|param| {
                vec![
                    Cell::new(&param.key).fg(Color::Blue),
                    Cell::new(if param.required { "yes" } else { "no" }),
                    Cell::new(&param.documentation),
                ]
            })
            .collect(),
    )
}

/// Pulls the message out of an API error so per-extension failures read as a sentence instead of a debug dump.
fn api_error_message(err: gofer_sdk::api::Error<gofer_sdk::api::types::Error>) -> String {
    match err {
        gofer_sdk::api::Error::ErrorResponse(response) => response.into_inner().message,
        other => other.to_string(),
    }
}

fn action_color(action: &PlanAction) -> Color {
    match action {
        PlanAction::Install | PlanAction::Start => Color::Green,
        PlanAction::Update => Color::Yellow,
        PlanAction::Disable | PlanAction::Unconfigure => Color::AnsiValue(245),
        PlanAction::Invalid => Color::Red,
        PlanAction::Unchanged => Color::Reset,
    }
}

/// Prints one extension's planned change and why.
fn print_planned(planned: &PlannedExtension) {
    println!(
        "  {} {}",
        planned.extension_id.cyan(),
        format!("({})", planned.action).bold()
    );

    if !planned.manifest.is_empty() {
        println!("  {} manifest: {}", rail(), planned.manifest);
    }

    if planned.action == PlanAction::Invalid {
        println!("  {} {}", rail(), planned.error.red());
    }

    for change in &planned.changes {
        let line = match (change.old.is_empty(), change.new.is_empty()) {
            (true, _) => format!("{} {} = {}", "+".green(), change.field, change.new),
            (_, true) => format!("{} {} (was {})", "-".red(), change.field, change.old),
            _ => format!(
                "{} {}: {} -> {}",
                "~".yellow(),
                change.field,
                change.old,
                change.new
            ),
        };
        println!("  {} {line}", rail());
    }

    match planned.action {
        PlanAction::Disable => println!("  {} will be stopped", rail()),
        PlanAction::Unconfigure => println!(
            "  {} will be stopped; its subscriptions are kept until you run 'gofer extension purge {}'",
            rail(),
            planned.extension_id
        ),
        PlanAction::Start => println!("  {} isn't running; it'll be started again", rail()),
        _ => {}
    }
    println!("");
}

enum Choice {
    Yes,
    Skip,
    Quit,
}

fn ask(prompt: &str) -> Choice {
    loop {
        let answer = question!("{prompt} [y]es / [s]kip / [q]uit: ");
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Choice::Yes,
            "s" | "skip" | "n" | "no" => return Choice::Skip,
            "q" | "quit" => return Choice::Quit,
            _ => continue,
        }
    }
}

fn ask_yes_no(prompt: &str) -> bool {
    let answer = question!("{prompt} [y/N]: ");
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// A config value for the [[extensions.install]] block, quoted as a TOML string.
fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

/// Prints the [[extensions.install]] block for a manifest. `location` is left out for Gofer's default extensions,
/// which use the manifest built into Gofer.
fn print_install_block(manifest: &Manifest, location: Option<&str>, id: &str) {
    let mut secret_commands = vec![];
    let mut lines = vec![
        "[[extensions.install]]".to_string(),
        format!("id = {}", toml_string(id)),
    ];
    if let Some(location) = location {
        lines.push(format!("manifest = {}", toml_string(location)));
    }

    if manifest.config_params.is_empty() {
        lines.push("# This extension takes no settings.".into());
    } else {
        lines.push("[extensions.install.settings]".into());
    }

    for param in &manifest.config_params {
        lines.push(String::new());
        for doc_line in param.documentation.lines() {
            lines.push(format!("# {doc_line}"));
        }

        let mut notes = vec![
            if param.required {
                "required"
            } else {
                "optional"
            }
            .to_string(),
        ];
        if !param.default.is_empty() {
            notes.push(format!("defaults to {}", toml_string(&param.default)));
        }
        if param.secret {
            notes.push("secret; must be a global secret reference".into());
        }
        lines.push(format!("# ({})", notes.join(", ")));

        let value = if param.secret {
            // Secret keys only allow letters, numbers, and hyphens.
            let secret_key = format!("{id}-{}", param.key).replace('_', "-");
            secret_commands.push(format!("gofer secret global put {secret_key} < <file>"));
            toml_string(&format!("global_secret{{{{{secret_key}}}}}"))
        } else if !param.default.is_empty() {
            toml_string(&param.default)
        } else {
            toml_string("")
        };

        // Optional settings that have a default are left commented out so the extension's default stays in charge.
        if !param.required && !param.default.is_empty() {
            lines.push(format!("# {} = {value}", param.key));
        } else {
            lines.push(format!("{} = {value}", param.key));
        }
    }

    if !secret_commands.is_empty() {
        println!("# Store the secret settings first; leave out '< <file>' to be prompted instead:");
        for command in &secret_commands {
            println!("#   {command}");
        }
        println!("");
    }

    for line in lines {
        println!("{line}");
    }

    println!("");
    println!("# Add the block to Gofer's config, then run 'gofer extension reload'.");
}

impl Cli {
    /// Prints the [[extensions.install]] block for a default extension (read from the Gofer server, so it matches
    /// what that server runs) or for any other extension's manifest URL or path.
    pub async fn extension_manifest(&self, location: &str, id: Option<&str>) -> Result<()> {
        if default_manifest(location).is_none() {
            let manifest = fetch_manifest(location).await?;
            print_install_block(&manifest, Some(location), id.unwrap_or("my-extension"));
            return Ok(());
        }

        let url = format!(
            "{}/extensions/manifests/{location}.toml",
            self.conf.api_base_url.trim_end_matches('/')
        );
        let content = reqwest::get(&url)
            .await
            .and_then(|response| response.error_for_status())
            .with_context(|| {
                format!("Could not get the '{location}' manifest from Gofer at {url}")
            })?
            .text()
            .await
            .with_context(|| format!("Could not read the '{location}' manifest from Gofer"))?;

        let manifest = Manifest::parse(&content)?;
        print_install_block(&manifest, None, id.unwrap_or(location));
        Ok(())
    }

    pub async fn extension_list(&self) -> Result<()> {
        let extensions = self
            .client
            .list_extensions()
            .await
            .context("Could not successfully retrieve extensions from Gofer api")?
            .into_inner()
            .extensions;

        let mut table = comfy_table::Table::new();
        table
            .load_style(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("status")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("image")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("reason")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        let mut extensions = extensions;
        extensions.sort_by(|a, b| {
            a.registration
                .extension_id
                .cmp(&b.registration.extension_id)
        });

        for extension in extensions {
            table.add_row(vec![
                Cell::new(&extension.registration.extension_id).fg(Color::Green),
                Cell::new(extension.state.to_string())
                    .fg(colorize_status_text_comfy(extension.state)),
                Cell::new(extension.registration.status.to_string())
                    .fg(colorize_status_text_comfy(extension.registration.status)),
                Cell::new(&extension.registration.image),
                Cell::new(&extension.state_reason),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn extension_get(&self, id: &str) -> Result<()> {
        let extension = self
            .client
            .get_extension(id)
            .await
            .context("Could not successfully retrieve extension from Gofer api")?
            .into_inner()
            .extension;

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Status: {{ status }}
  {%- if reason %}
  {{ vertical_line }} Reason: {{ reason }}
  {%- endif %}
  {{ vertical_line }} Image: {{ image }}
  {{ vertical_line }} Manifest: {{ manifest }}
  {{ vertical_line }} Endpoint: {{ url }}
  {{ vertical_line }} Started {{ started }}

  $ Pipeline Params:
  {%- for line in pipeline_params %}
  {{ line }}
  {%- endfor %}

  $ Config Params:
  {%- for line in config_params %}
  {{ line }}
  {%- endfor %}

  $ Settings:
  {%- for line in settings %}
  {{ line }}
  {%- endfor %}

  Info:
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let or_none = |value: &str| {
            if value.is_empty() {
                "None".to_string()
            } else {
                value.blue().to_string()
            }
        };

        let mut context = tera::Context::new();
        context.insert("vertical_line", &rail());
        context.insert(
            "status",
            &colorize_status_text(extension.registration.status),
        );
        context.insert("reason", &extension.state_reason);
        context.insert("image", &or_none(&extension.registration.image));
        context.insert("manifest", &or_none(&extension.manifest));
        context.insert("url", &or_none(&extension.url));
        context.insert(
            "started",
            &self
                .format_time(extension.started)
                .unwrap_or_else(|| "Not yet".to_string()),
        );
        context.insert(
            "pipeline_params",
            &params_table(&extension.documentation.pipeline_subscription_params),
        );
        context.insert(
            "config_params",
            &params_table(&extension.documentation.config_params),
        );

        // Secrets are stored as global secret references, so settings are safe to show as they are.
        let mut settings: Vec<String> = extension
            .registration
            .settings
            .iter()
            .map(|setting| format!("{} {} = {}", rail(), setting.key.blue(), setting.value))
            .collect();
        if settings.is_empty() {
            settings.push(format!("{} None", rail()));
        }
        context.insert("settings", &settings);

        let content = tera.render("main", &context)?;
        println!(
            "  Extension {} :: {}",
            &extension.registration.extension_id.cyan(),
            colorize_status_text(extension.state)
        );
        println!("{}", content.trim_end());
        if extension.documentation.body.is_empty() {
            println!("  No documentation found");
        } else {
            for line in extension.documentation.body.lines() {
                println!("  {line}");
            }
        }
        Ok(())
    }

    pub async fn extension_reload(&self, yes: bool, dry_run: bool) -> Result<()> {
        let plan = self
            .client
            .plan_extensions()
            .await
            .map_err(|e| anyhow!("Could not plan extension changes; {}", api_error_message(e)))?
            .into_inner()
            .extensions;

        let unchanged = plan
            .iter()
            .filter(|planned| planned.action == PlanAction::Unchanged)
            .count();
        let invalid: Vec<&PlannedExtension> = plan
            .iter()
            .filter(|planned| planned.action == PlanAction::Invalid)
            .collect();
        let pending: Vec<&PlannedExtension> = plan
            .iter()
            .filter(|planned| {
                !matches!(planned.action, PlanAction::Unchanged | PlanAction::Invalid)
            })
            .collect();

        for planned in invalid.iter().chain(pending.iter()) {
            print_planned(planned);
        }

        if !invalid.is_empty() {
            warning!(
                "{} extension(s) have config problems and will be left as they are until fixed",
                invalid.len()
            );
        }

        if pending.is_empty() {
            success!("Running extensions already match Gofer's config ({unchanged} unchanged)");
            return Ok(());
        }

        println!(
            "{} change(s) to apply, {unchanged} extension(s) unchanged.",
            pending.len()
        );

        if dry_run {
            return Ok(());
        }

        let mut summary: Vec<(String, PlanAction, String, String)> = vec![];

        for planned in pending {
            let id = planned.extension_id.clone();

            if !yes {
                match ask(&format!("{} '{id}'?", planned.action)) {
                    Choice::Yes => {}
                    Choice::Skip => {
                        summary.push((id, planned.action, "skipped".into(), String::new()));
                        continue;
                    }
                    Choice::Quit => break,
                }
            }

            let response = match self
                .client
                .apply_extension(
                    &id,
                    &ApplyExtensionRequest {
                        plan_hash: planned.plan_hash.clone(),
                    },
                )
                .await
            {
                Ok(response) => response.into_inner(),
                Err(e) => {
                    let message = api_error_message(e);
                    error!("Could not apply '{id}'; {message}");
                    summary.push((id, planned.action, "failed".into(), message));
                    continue;
                }
            };

            let extension = response.extension;
            let should_run = matches!(
                planned.action,
                PlanAction::Install | PlanAction::Update | PlanAction::Start
            );

            if !should_run || extension.state.to_string() == "running" {
                success!("{} '{id}'", planned.action);
                summary.push((id, planned.action, "done".into(), String::new()));
                continue;
            }

            error!("'{id}' did not start; {}", extension.state_reason);

            let mut outcome = ("failed".to_string(), extension.state_reason.clone());

            if response.can_revert
                && (yes
                    || ask_yes_no(&format!(
                        "Put back the version of '{id}' that was running before?"
                    )))
            {
                match self.client.revert_extension(&id).await {
                    Ok(reverted) => {
                        let reverted = reverted.into_inner().extension;
                        if reverted.state.to_string() == "running" {
                            success!("Reverted '{id}'; fix its config and run reload again");
                            outcome = (
                                "reverted".into(),
                                format!("new version failed: {}", extension.state_reason),
                            );
                        } else {
                            error!("Reverting '{id}' also failed; {}", reverted.state_reason);
                            outcome.1 = reverted.state_reason;
                        }
                    }
                    Err(e) => {
                        let message = api_error_message(e);
                        error!("Could not revert '{id}'; {message}");
                        outcome.1 = message;
                    }
                }
            }

            summary.push((id, planned.action, outcome.0, outcome.1));
        }

        let mut table = comfy_table::Table::new();
        table
            .load_style(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id").fg(Color::Blue),
                Cell::new("action").fg(Color::Blue),
                Cell::new("result").fg(Color::Blue),
                Cell::new("reason").fg(Color::Blue),
            ]);

        for (id, action, result, reason) in summary {
            table.add_row(vec![
                Cell::new(id).fg(Color::Green),
                Cell::new(action.to_string()).fg(action_color(&action)),
                Cell::new(&result).fg(match result.as_str() {
                    "done" => Color::Green,
                    "failed" => Color::Red,
                    "reverted" => Color::Yellow,
                    _ => Color::Reset,
                }),
                Cell::new(reason),
            ]);
        }

        println!("");
        println!("{}", table);
        Ok(())
    }

    pub async fn extension_purge(&self, id: &str, yes: bool) -> Result<()> {
        let subscriptions = self
            .client
            .list_extension_subscriptions(id)
            .await
            .map_err(|e| {
                anyhow!(
                    "Could not list subscriptions for '{id}'; {}",
                    api_error_message(e)
                )
            })?
            .into_inner()
            .subscriptions;

        let pipelines: BTreeSet<String> = subscriptions
            .iter()
            .map(|subscription| {
                format!("{}/{}", subscription.namespace_id, subscription.pipeline_id)
            })
            .collect();

        if pipelines.is_empty() {
            println!("No pipelines are subscribed to '{id}'.");
        } else {
            println!(
                "Purging '{id}' unsubscribes {} pipeline(s) ({} subscription(s)):",
                pipelines.len(),
                subscriptions.len()
            );
            for pipeline in &pipelines {
                println!("  {} {pipeline}", rail());
            }
        }

        if !yes
            && !ask_yes_no(&format!(
                "Permanently delete '{id}', its subscriptions, and its stored objects?"
            ))
        {
            bail!("Purge cancelled");
        }

        self.client
            .purge_extension(id)
            .await
            .map_err(|e| anyhow!("Could not purge extension; {}", api_error_message(e)))?;

        success!("Purged extension '{id}'");
        Ok(())
    }

    pub async fn extension_logs(&self, id: &str) -> Result<()> {
        let extension_logs_conn = self
            .client
            .get_extension_logs(id)
            .await
            .map_err(|e| anyhow!("could not get logs; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            extension_logs_conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (_, mut read) = stream.split();

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => println!("{}", text),
                Ok(Message::Binary(_)) => println!("Received binary data"),
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        match frame.code {
                            tungstenite::protocol::frame::coding::CloseCode::Normal => break,
                            _ => {
                                error!("Connection closed by server; {}", frame.reason)
                            }
                        }
                        break;
                    }
                    error!("Connection closed by server without reason");
                    break;
                }
                Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => {
                    error!("Connection closed");
                    break;
                }
                Err(tokio_tungstenite::tungstenite::Error::Protocol(e))
                    if e.to_string()
                        .contains("Connection reset without closing handshake") =>
                {
                    error!("Connection reset without closing handshake");
                    break;
                }
                Err(e) => {
                    bail!("Error receiving message: {}", e);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn extension_debug(&self, id: &str) -> Result<()> {
        let info = self
            .client
            .get_extension_debug_info(id)
            .await
            .context("Could not successfully retrieve extension debug info from Gofer api")?
            .into_inner()
            .info;

        let json_value: serde_json::Value = serde_json::from_str(&info)?;
        let pretty_json = serde_json::to_string_pretty(&json_value)?;

        println!("{}", pretty_json);
        Ok(())
    }
}
