use crate::cli::{validate_identifier, Cli};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};

#[derive(Debug, Args, Clone)]
pub struct PipelineSecretSubcommands {
    #[clap(subcommand)]
    pub command: PipelineSecretCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum PipelineSecretCommands {
    /// View keys from a pipeline's secret store.
    List {
        /// Pipeline identifier.
        pipeline_id: String,
    },

    /// Read a secret from a pipeline's secret store.
    Get {
        /// Pipeline identifier.
        pipeline_id: String,
        key: String,

        /// Include secret in plaintext.
        #[arg(short, long, default_value = "false")]
        include_secret: bool,
    },

    /// Write a secret to a pipeline's secret store.
    ///
    /// You can store both regular text values or read in entire files using the '@' prefix.
    Put {
        /// Pipeline identifier.
        pipeline_id: String,
        key: String,

        /// takes a plain text string or use character '@' to pass in text to stdin.
        /// ex. echo "some_secret" > gofer secret put mysecret @
        secret: String,

        /// Replace value if it exists.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },
}

impl Cli {
    pub async fn handle_pipeline_secret_subcommands(
        &self,
        command: PipelineSecretSubcommands,
    ) -> Result<()> {
        let cmds = command.command;
        match cmds {
            PipelineSecretCommands::List { pipeline_id } => {
                self.pipeline_secret_list(command.namespace, &pipeline_id)
                    .await
            }
            PipelineSecretCommands::Get {
                pipeline_id,
                key,
                include_secret,
            } => {
                self.pipeline_secret_get(command.namespace, &pipeline_id, &key, include_secret)
                    .await
            }
            PipelineSecretCommands::Put {
                pipeline_id,
                key,
                secret,
                force,
            } => {
                self.pipeline_secret_put(command.namespace, &pipeline_id, &key, &secret, force)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn pipeline_secret_list(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let secrets = self
            .client
            .list_pipeline_secrets(&namespace, pipeline_id)
            .await
            .context("Could not successfully retrieve pipeline secrets from Gofer api")?
            .into_inner()
            .secrets;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("key")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("created")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for secret in secrets {
            table.add_row(vec![
                Cell::new(secret.key).fg(Color::Green),
                Cell::new(
                    self.format_time(secret.created)
                        .unwrap_or("Unknown".to_string()),
                ),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn pipeline_secret_get(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        key: &str,
        include_secret: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let secret = self
            .client
            .get_pipeline_secret(&namespace, pipeline_id, key, include_secret)
            .await
            .context("Could not successfully retrieve secret from Gofer api")?;

        const TEMPLATE: &str = r#"  Key: {{ key }}
  Secret: {{ secret }}

  Created {{ created }}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("key", &secret.metadata.key);
        context.insert(
            "secret",
            &secret.secret.clone().unwrap_or("[Redacted]".into()),
        );
        context.insert(
            "created",
            &self
                .format_time(secret.metadata.created)
                .unwrap_or("Unknown".to_string()),
        );

        let content = tera.render("main", &context)?;
        print!("{}", content);
        Ok(())
    }

    pub async fn pipeline_secret_put(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        key: &str,
        secret: &str,
        force: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let mut secret_input = String::new();

        if secret == "@" {
            std::io::stdin()
                .read_line(&mut secret_input)
                .context("Could not read secret from stdin")?;
        } else {
            secret_input = secret.into();
        };

        validate_identifier(key).context("invalid key name")?;

        self.client
            .put_pipeline_secret(
                &namespace,
                pipeline_id,
                &gofer_sdk::api::types::PutPipelineSecretRequest {
                    content: secret_input,
                    force,
                    key: key.into(),
                },
            )
            .await
            .context("Could not insert pipeline secret")?;

        success!("Successfully inserted new secret '{}'", key);

        Ok(())
    }
}
