use crate::cli::{
    colorize_status_text, colorize_status_text_comfy, humanize_relative_duration, Cli,
};
use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use futures::StreamExt;
use polyfmt::{error, print, println, question, success};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[derive(Debug, Args, Clone)]
pub struct ExtensionSubcommands {
    #[clap(subcommand)]
    pub command: ExtensionCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ExtensionCommands {
    /// Get a specific extension by id.
    Get {
        /// Extension Identifier.
        id: String,
    },

    /// List all extensions.
    List,

    /// Install a specific Gofer extension.
    ///
    /// You can find a list of provided extensions here: https://gofer.clintjedwards.com/docs/ref/extensions/provided/index.html
    ///
    /// Ex. gofer extension install github ghcr.io/clintjedwards/gofer/extensions/github:0.7.0
    Install {
        id: String,
        image: String,

        /// Extension configuration values. These are usually documented by the extension and may be required to
        /// install the extension.
        ///
        /// Ex. --config="APP_ID=some_id" --config="APP_NAME=some_name"
        #[arg(short, long)]
        config: Vec<String>,

        /// Collect config values interactively rather than purely by command line. Useful if you have to do a lot of
        /// copy pasting of values.
        #[arg(short, long, default_value = "false")]
        interactive: bool,

        /// Add additional roles to the extension.
        ///
        /// Extensions by default are provisioned with tokens that only have access to a base set of things the extension
        /// might need access to. To provide extensions with additional functionality you can create a new role and
        /// add it to the extension here by role_id.
        #[arg(short, long)]
        additional_roles: Vec<String>,
    },
    Uninstall {
        /// Extension Identifier.
        id: String,
    },

    /// Update extension from a disabled state to a enabled.
    Enable {
        /// Extension Identifier.
        id: String,
    },

    /// Stop a extension from being able to run.
    Disable {
        /// Extension Identifier.
        id: String,
    },

    /// Return logs for extension by id.
    Logs {
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
            ExtensionCommands::Install {
                id,
                image,
                config,
                interactive,
                additional_roles,
            } => {
                self.extension_install(&id, &image, config, interactive, additional_roles)
                    .await
            }
            ExtensionCommands::Uninstall { id } => self.extension_uninstall(&id).await,
            ExtensionCommands::Enable { id } => self.extension_enable(&id).await,
            ExtensionCommands::Disable { id } => self.extension_disable(&id).await,
            ExtensionCommands::Logs { id } => self.extension_logs(&id).await,
        }
    }
}

impl Cli {
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
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("url")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for extension in extensions {
            table.add_row(vec![
                Cell::new(&extension.registration.extension_id).fg(Color::Green),
                Cell::new(&extension.url),
                Cell::new(&extension.state.to_string())
                    .fg(colorize_status_text_comfy(extension.state)),
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
  Started {{ started }}

  Endpoint: {{ url }}

  {%- if documentation %}

  Config Params:
    {%- if config_parms %}
    {%- for line in config_params %}
    • {{ line.key }} ::{% if line.required %} Required {% endif %} :: {{line.documentation}}
    {%- endfor %}
    {%- else %}
    None
    {%- endif %}

  Pipeline Params:
    {%- if pipeline_params %}
    {%- for line in pipeline_params %}
    • {{ line.key }} {% if line.required %}:: Required {% endif %}:: {{line.documentation}}
    {%- endfor %}
    {%- else -%}
    None
    {%- endif %}

  Info:
  {%- endif %}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert(
            "started",
            &humanize_relative_duration(extension.started).unwrap_or_else(|| "Not yet".to_string()),
        );
        context.insert("url", &extension.url);
        context.insert("config_params", &extension.documentation.config_params);
        context.insert(
            "pipeline_params",
            &extension.documentation.pipeline_subscription_params,
        );
        context.insert("documentation", &extension.documentation.body);

        let content = tera.render("main", &context)?;
        println!(
            "  Extension {} :: {}",
            &extension.registration.extension_id.cyan(),
            colorize_status_text(extension.state)
        );
        print!("{}", content);
        print!("{}", "\n");
        if extension.documentation.body.is_empty() {
            println!("{}", "No documentation found");
        } else {
            println!("{}", &extension.documentation.body);
        }
        Ok(())
    }

    pub async fn extension_install(
        &self,
        id: &str,
        image: &str,
        config: Vec<String>,
        interactive: bool,
        additional_roles: Vec<String>,
    ) -> Result<()> {
        let mut settings = std::collections::HashMap::new();

        for key_value_pair_str in config {
            let (key, value) = match key_value_pair_str.split_once('=') {
                Some((key, value)) => (key.to_string(), value.to_string()),
                None => {
                    bail!("Malformed config string '{key_value_pair_str}'; Must be in format: <KEY>=<VALUE>");
                }
            };

            settings.insert(key, value);
        }

        if interactive {
            println!("Enter the extension config values below;");
            println!("Enter an empty key when finished;");
            println!("Entering a duplicate key overrides the previous one:");

            println!("\n");
            loop {
                let key = question!("Key: ");
                if key.is_empty() {
                    break;
                }

                let value = question!("Value: ");

                success!("Recorded: {} > {}", &key, &value);
                settings.insert(key, value);
            }
        }

        self.client
            .install_extension(&gofer_sdk::api::types::InstallExtensionRequest {
                additional_roles: Some(additional_roles),
                id: id.to_string(),
                image: image.to_string(),
                registry_auth: None,
                settings,
            })
            .await
            .context("Could not successfully install extension")?;

        success!("Successfully installed extension '{id}'!");
        Ok(())
    }

    pub async fn extension_uninstall(&self, id: &str) -> Result<()> {
        self.client
            .uninstall_extension(id)
            .await
            .context("Could not successfully uninstall extension")?;

        success!("Successfully uninstalled extension '{id}'!");
        Ok(())
    }

    pub async fn extension_enable(&self, id: &str) -> Result<()> {
        self.client
            .update_extension(
                id,
                &gofer_sdk::api::types::UpdateExtensionRequest { enable: true },
            )
            .await
            .context("Could not successfully enable extension")?;

        success!("Successfully enabled extension '{id}'!");
        Ok(())
    }

    pub async fn extension_disable(&self, id: &str) -> Result<()> {
        self.client
            .update_extension(
                id,
                &gofer_sdk::api::types::UpdateExtensionRequest { enable: false },
            )
            .await
            .context("Could not successfully enable extension")?;

        success!("Successfully disabled extension '{id}'!");
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
}
