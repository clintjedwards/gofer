use crate::cli::{
    colorize_status_text, colorize_status_text_comfy, humanize_future_time,
    humanize_relative_duration, Cli,
};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};
use std::collections::HashMap;

#[derive(Debug, Args, Clone)]
pub struct TokenSubcommands {
    #[clap(subcommand)]
    pub command: TokenCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TokenCommands {
    /// List all tokens.
    List,

    /// Fetch information about an individual token.
    Get {
        /// Token Identifier.
        id: String,
    },

    /// Create a new token.
    Create {
        /// The type of token to create
        ///
        /// Valid values are: "Management" or "User".
        token_type: gofer_sdk::api::types::TokenType,

        /// Total time in seconds until token expires.
        #[arg(short, long, default_value = "31536000")] // default is a year
        expiry: u64,

        /// Which namespaces the token will have access to.
        #[arg(short, long, default_value = "[]")]
        namespace: Vec<String>,

        #[arg(short, long, default_value = "[]")]
        metadata: Vec<String>,
    },

    /// Creates the initial management token.
    Bootstrap,

    /// Enable specific token.
    Enable {
        /// Token Identifier.
        id: String,
    },

    /// Disable specific token.
    Disable {
        /// Token Identifier.
        id: String,
    },

    /// Get details about the token currently being used
    Whoami,

    /// Delete specific token.
    Delete {
        /// Token Identifier.
        id: String,
    },
}

impl Cli {
    pub async fn handle_token_subcommands(&self, command: TokenSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            TokenCommands::List => self.token_list().await,
            TokenCommands::Get { id } => self.token_get(&id).await,
            TokenCommands::Create {
                token_type,
                expiry,
                namespace,
                metadata,
            } => {
                self.token_create(token_type, expiry, namespace, metadata)
                    .await
            }
            TokenCommands::Bootstrap {} => self.token_bootstrap().await,
            TokenCommands::Enable { id } => self.token_enable(&id).await,
            TokenCommands::Disable { id } => self.token_disable(&id).await,
            TokenCommands::Whoami => self.token_whoami().await,
            TokenCommands::Delete { id } => self.token_delete(&id).await,
        }
    }
}

impl Cli {
    pub async fn token_list(&self) -> Result<()> {
        let tokens = self
            .client
            .list_tokens()
            .await
            .context("Could not successfully retrieve tokens from Gofer api")?
            .into_inner()
            .tokens;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("type")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("created")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("expires")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("active")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for token in tokens {
            let active = !token.disabled;

            table.add_row(vec![
                Cell::new(token.id).fg(Color::Green),
                Cell::new(token.token_type),
                Cell::new(
                    humanize_relative_duration(token.created)
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                Cell::new(
                    humanize_future_time(token.expires).unwrap_or_else(|| "Unknown".to_string()),
                ),
                Cell::new(active).fg(colorize_status_text_comfy(active)),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn token_get(&self, id: &str) -> Result<()> {
        let token = self
            .client
            .get_token_by_id(id)
            .await
            .context("Could not successfully retrieve token from Gofer api")?
            .into_inner()
            .token;

        const TEMPLATE: &str = r#"
  {%- if namespaces %}
  Valid for Namespaces:
    {%- for namespace in namespaces %}
    • {{ namespace }}
    {%- endfor -%}
  {%- endif -%}
  {% if metadata %}
  Metadata:
    {%- for key, value in metadata %}
    • {{ key }}: {{ value }}
    {%- endfor -%}
  {%- endif %}

  Created {{created}} | Expires {{expires}} | Active: {{disabled}}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("namespaces", &token.namespaces);
        context.insert("metadata", &token.metadata);
        context.insert(
            "created",
            &humanize_relative_duration(token.created).unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert(
            "expires",
            &humanize_future_time(token.expires).unwrap_or_else(|| "Unknown".to_string()),
        );

        let active = !token.disabled;

        context.insert("disabled", &colorize_status_text(active.to_string()));

        let content = tera.render("main", &context)?;
        println!("[{}] :: {} Token", &token.id, &token.token_type.to_string());
        print!("{}", content);
        Ok(())
    }

    pub async fn token_create(
        &self,
        token_type: gofer_sdk::api::types::TokenType,
        expiry: u64,
        namespace: Vec<String>,
        metadata: Vec<String>,
    ) -> Result<()> {
        let mut metadata_map = HashMap::new();

        for var in metadata {
            let split_var = var.split_once('=');
            match split_var {
                Some((key, value)) => {
                    metadata_map.insert(key.into(), value.into());
                }
                None => {
                    bail!(
                        "malformed metadata '{}'; must be in format <KEY>=<VALUE>",
                        var
                    );
                }
            }
        }

        let token = self
            .client
            .create_token(&gofer_sdk::api::types::CreateTokenRequest {
                expires: expiry,
                metadata: metadata_map,
                namespaces: namespace,
                token_type,
            })
            .await
            .context("Could not successfully create token from Gofer api")?
            .into_inner();

        success!("Successfully created token '{}'!", token.token_details.id);
        success!("Secret: {}", token.secret);
        Ok(())
    }

    pub async fn token_enable(&self, id: &str) -> Result<()> {
        self.client
            .update_token(
                id,
                &gofer_sdk::api::types::UpdateTokenRequest {
                    disabled: Some(false),
                },
            )
            .await
            .context("Could not successfully update token from Gofer api")?;

        success!("token '{}' enabled!", id);
        Ok(())
    }

    pub async fn token_disable(&self, id: &str) -> Result<()> {
        self.client
            .update_token(
                id,
                &gofer_sdk::api::types::UpdateTokenRequest {
                    disabled: Some(true),
                },
            )
            .await
            .context("Could not successfully update token from Gofer api")?;

        success!("token '{}' disabled!", id);
        Ok(())
    }

    pub async fn token_bootstrap(&self) -> Result<()> {
        let token = self
            .client
            .create_bootstrap_token()
            .await
            .context("Could not successfully update token from Gofer api")?
            .into_inner();

        success!("Successfully created token '{}'!", token.token_details.id);
        success!("Secret: {}", token.secret);
        Ok(())
    }

    pub async fn token_whoami(&self) -> Result<()> {
        let token = self
            .client
            .whoami()
            .await
            .context("Could not successfully retrieve token from Gofer api")?
            .into_inner()
            .token;

        const TEMPLATE: &str = r#"
  {%- if namespaces %}
  Valid for Namespaces:
    {%- for namespace in namespaces %}
    • {{ namespace }}
    {%- endfor -%}
  {%- endif -%}

  {%- if metadata %}
  Metadata:
    {%- for key, value in metadata %}
    • {{ key }}: {{ value }}
    {%- endfor -%}
  {%- endif -%}

  Created {{created}} | Expires {{expires}} | Active: {{disabled}}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("namespaces", &token.namespaces);
        context.insert("metadata", &token.metadata);
        context.insert(
            "created",
            &humanize_relative_duration(token.created).unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert(
            "created",
            &humanize_relative_duration(token.expires).unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert("disabled", &token.disabled);

        let content = tera.render("main", &context)?;
        println!("[{}] :: {} Token", &token.id, &token.token_type.to_string());
        print!("{}", content);
        Ok(())
    }

    pub async fn token_delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_token(id)
            .await
            .context("Could not successfully retrieve token from Gofer api")?;

        success!("token '{}' deleted!", id);
        Ok(())
    }
}
