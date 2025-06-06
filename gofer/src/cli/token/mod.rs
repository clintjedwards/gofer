use crate::cli::{colorize_status_text, colorize_status_text_comfy, Cli};
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
        /// The username for the token.
        ///
        /// This will appear as a helper to see which user performed which action.
        user: String,

        /// The roles that should be assigned to this token.
        #[arg(short, long)]
        roles: Vec<String>,

        /// Total time in seconds until token expires.
        /// Expiry of 0 means the token does not expire.
        #[arg(short, long, default_value = "0")]
        expiry: u64,

        #[arg(short, long)]
        metadata: Vec<String>,
    },

    /// Creates the initial root access token.
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
                roles,
                expiry,
                metadata,
                user,
            } => self.token_create(roles, expiry, metadata, user).await,
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
                Cell::new("user")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("roles")
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
                Cell::new(token.user),
                Cell::new(format!("{:?}", token.roles)),
                Cell::new(
                    self.format_time(token.created)
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                Cell::new(
                    self.format_time(token.expires)
                        .unwrap_or_else(|| "Never".to_string()),
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
  {%- if roles %}
  Roles:
    {%- for role in roles %}
    • {{ role }}
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
        context.insert("roles", &token.roles);
        context.insert("metadata", &token.metadata);
        context.insert(
            "created",
            &self
                .format_time(token.created)
                .unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert(
            "expires",
            &self
                .format_time(token.expires)
                .unwrap_or_else(|| "Unknown".to_string()),
        );

        let active = !token.disabled;

        context.insert("disabled", &colorize_status_text(active.to_string()));

        let content = tera.render("main", &context)?;
        println!("[{}] :: User: {}", &token.id, &token.user);
        print!("{}", content);
        Ok(())
    }

    pub async fn token_create(
        &self,
        roles: Vec<String>,
        expiry: u64,
        metadata: Vec<String>,
        user: String,
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

        let metadata = if metadata_map.is_empty() {
            None
        } else {
            Some(metadata_map)
        };

        let token = self
            .client
            .create_token(&gofer_sdk::api::types::CreateTokenRequest {
                expires: expiry,
                metadata,
                roles,
                user,
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
    {%- if roles %}
    Roles:
        {%- for role in roles %}
        • {{ role }}
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
        context.insert("roles", &token.roles);
        context.insert("metadata", &token.metadata);
        context.insert(
            "created",
            &self
                .format_time(token.created)
                .unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert(
            "expires",
            &self
                .format_time(token.expires)
                .unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert("disabled", &token.disabled);

        let content = tera.render("main", &context)?;
        println!("[{}] :: User {}", &token.id, &token.user);
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
