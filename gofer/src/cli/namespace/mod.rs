use crate::cli::Cli;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{print, println, success};

#[derive(Debug, Args, Clone)]
pub struct NamespaceSubcommands {
    #[clap(subcommand)]
    pub command: NamespaceCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum NamespaceCommands {
    /// List all namespaces.
    List,

    /// Fetch information about an individual namespace.
    Get {
        /// Namespace Identifier.
        id: String,
    },

    /// Create a new namespace.
    Create {
        /// Namespace Identifier.
        ///
        /// Must be:
        /// * 32 > characters < 3
        /// * Only alphanumeric characters or hyphens
        id: String,

        /// Humanized name for the namespace.
        ///
        /// Can contain spaces and a wider set of characters.
        name: String,

        /// A short description about the namespace.
        description: String,
    },
    Update {
        /// Namespace Identifier.
        id: String,

        /// Human readable name for namespace.
        #[arg(short, long)]
        name: Option<String>,

        /// Short description about the namespace.
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a namespace.
    Delete {
        /// Namespace Identifier.
        id: String,
    },
}

impl Cli {
    pub async fn handle_namespace_subcommands(&self, command: NamespaceSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            NamespaceCommands::List => self.namespace_list().await,
            NamespaceCommands::Get { id } => self.namespace_get(&id).await,
            NamespaceCommands::Create {
                id,
                name,
                description,
            } => self.namespace_create(&id, &name, &description).await,
            NamespaceCommands::Update {
                id,
                name,
                description,
            } => self.namespace_update(&id, name, description).await,
            NamespaceCommands::Delete { id } => self.namespace_delete(&id).await,
        }
    }
}

impl Cli {
    pub async fn namespace_list(&self) -> Result<()> {
        let namespaces = self
            .client
            .list_namespaces()
            .await
            .context("Could not successfully retrieve namespaces from Gofer api")?
            .into_inner()
            .namespaces;

        let mut table = comfy_table::Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("name")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("description")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("created")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for namespace in namespaces {
            table.add_row(vec![
                Cell::new(namespace.id).fg(Color::Green),
                Cell::new(namespace.name),
                Cell::new(namespace.description),
                Cell::new(
                    self.format_time(namespace.created)
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn namespace_get(&self, id: &str) -> Result<()> {
        let namespace = self
            .client
            .get_namespace(id)
            .await
            .context("Could not successfully retrieve namespace from Gofer api")?
            .into_inner()
            .namespace;

        const TEMPLATE: &str = r#"[{{id}}] {{name}}
{{description}}

Created {{created}}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("id", &namespace.id);
        context.insert("name", &namespace.name);
        context.insert("description", &namespace.description);
        context.insert(
            "created",
            &self
                .format_time(namespace.created)
                .unwrap_or_else(|| "Unknown".to_string()),
        );

        let content = tera.render("main", &context)?;
        print!("{}", content);
        Ok(())
    }

    pub async fn namespace_create(&self, id: &str, name: &str, description: &str) -> Result<()> {
        let namespace = self
            .client
            .create_namespace(&gofer_sdk::api::types::CreateNamespaceRequest {
                description: description.into(),
                id: id.into(),
                name: name.into(),
            })
            .await
            .context("Could not successfully create namespace from Gofer api")?
            .into_inner()
            .namespace;

        success!("Successfully created namespace '{}'!", namespace.id);
        Ok(())
    }

    pub async fn namespace_update(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()> {
        self.client
            .update_namespace(
                id,
                &gofer_sdk::api::types::UpdateNamespaceRequest { description, name },
            )
            .await
            .context("Could not successfully update namespace from Gofer api")?;

        success!("namespace '{}' updated!", id);
        Ok(())
    }

    pub async fn namespace_delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_namespace(id)
            .await
            .context("Could not successfully retrieve namespace from Gofer api")?;

        success!("namespace '{}' deleted!", id);
        Ok(())
    }
}
