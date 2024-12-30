use crate::cli::{validate_identifier, Cli};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};

#[derive(Debug, Args, Clone)]
pub struct GlobalSecretSubcommands {
    #[clap(subcommand)]
    pub command: GlobalSecretCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum GlobalSecretCommands {
    /// View keys from the global secret store.
    List,

    /// Read a secret from the global secret store.
    Get {
        key: String,

        /// Include secret in plaintext.
        #[arg(short, long, default_value = "false")]
        include_secret: bool,
    },

    /// Write a secret to the global secret store.
    ///
    /// You can store both regular text values or read in from stdin using the '@' prefix.
    ///
    /// Global secrets are namespaced to allow the segregation of global secrets among different groups.
    /// These namespaces strings allow simple regex expressions to match the actual namespaces within your
    /// environment.
    ///
    /// By default, omitting the namespace allows it to match ALL namespaces.
    ///
    /// For example an environment that uses prefixes to separate minor teams within an organization might look something
    /// like this: "ops-teama", "ops-teamb".
    ///
    /// In this case a global secret can be assigned to a specific team by just using the flag '-n "ops-teama"'. In the case
    /// that you had a global secret that need to be shared amongst all ops teams you could simply write a namespace filter
    /// that has a prefix like so '-n "ops-*"'.
    Put {
        key: String,

        /// takes a plain text string or use character '@' to pass in text to stdin.
        /// ex. echo "some_secret" > gofer secret put mysecret @
        secret: String,

        /// List of namespaces allowed to access this secret. Accepts regexes.
        #[arg(short, long, default_value = ".*")]
        namespaces: Vec<String>,

        /// Replace value if it exists.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },
}

impl Cli {
    pub async fn handle_global_secret_subcommands(
        &self,
        command: GlobalSecretSubcommands,
    ) -> Result<()> {
        let cmds = command.command;
        match cmds {
            GlobalSecretCommands::List => self.global_secret_list().await,
            GlobalSecretCommands::Get {
                key,
                include_secret,
            } => self.global_secret_get(&key, include_secret).await,
            GlobalSecretCommands::Put {
                key,
                secret,
                namespaces,
                force,
            } => {
                self.global_secret_put(&key, &secret, namespaces, force)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn global_secret_list(&self) -> Result<()> {
        let secrets = self
            .client
            .list_global_secrets()
            .await
            .context("Could not successfully retrieve global secrets from Gofer api")?
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
                Cell::new("namespaces")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("created")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for secret in secrets {
            table.add_row(vec![
                Cell::new(secret.key).fg(Color::Green),
                Cell::new(format!("{:?}", secret.namespaces)),
                Cell::new(
                    self.format_time(secret.created)
                        .unwrap_or("Unknown".to_string()),
                ),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn global_secret_get(&self, key: &str, include_secret: bool) -> Result<()> {
        let secret = self
            .client
            .get_global_secret(key, include_secret)
            .await
            .context("Could not successfully retrieve secret from Gofer api")?;

        const TEMPLATE: &str = r#"  Key: {{ key }}
  Secret: {{ secret }}
  Allowed Namespaces:
  {%- for line in namespaces %}
    - {{ line }}
  {%- endfor %}

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
        context.insert("namespaces", &secret.metadata.namespaces);
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

    pub async fn global_secret_put(
        &self,
        key: &str,
        secret: &str,
        namespaces: Vec<String>,
        force: bool,
    ) -> Result<()> {
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
            .put_global_secret(&gofer_sdk::api::types::PutGlobalSecretRequest {
                content: secret_input,
                force,
                key: key.into(),
                namespaces,
            })
            .await
            .context("Could not insert global secret")?;

        success!("Successfully inserted new secret '{}'", key);

        Ok(())
    }
}
