use super::CliHarness;
use crate::cli::humanize_relative_duration;
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{presets::ASCII_MARKDOWN, Cell, CellAlignment, Color, ContentArrangement};
use gofer_proto::{
    CreateNamespaceRequest, DeleteNamespaceRequest, GetNamespaceRequest, ListNamespacesRequest,
    UpdateNamespaceRequest,
};
use std::process;

#[derive(Debug, Args)]
pub struct NamespaceSubcommands {
    #[clap(subcommand)]
    pub command: NamespaceCommands,
}

#[derive(Debug, Subcommand)]
pub enum NamespaceCommands {
    /// List namespaces.
    List,

    /// Create a new namespace.
    Create {
        /// Identifier for namespace; Must be alphanumeric, lowercase,
        /// with only underscores as alternate characters.
        id: String,
        /// Humanized name for namespace.
        #[clap(short, long)]
        name: Option<String>,
        /// Helpful description of namespace.
        #[clap(short, long)]
        description: Option<String>,
    },

    /// Detail namespace by id.
    Get { id: String },

    /// Update a namespace.
    Update {
        /// Identifier for namespace
        id: String,
        /// Humanized name for namespace.
        #[clap(short, long)]
        name: Option<String>,
        /// Helpful description of namespace.
        #[clap(short, long)]
        description: Option<String>,
    },

    /// Delete a namespace.
    Delete { id: String },
}

impl CliHarness {
    pub async fn namespace_list(&self) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(ListNamespacesRequest {
            offset: 0,
            limit: 0,
        });
        let response = client
            .list_namespaces(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Command failed; {}", e.message());
                process::exit(1);
            })
            .into_inner();

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

        for namespace in response.namespaces {
            table.add_row(vec![
                Cell::new(namespace.id).fg(Color::Green),
                Cell::new(namespace.name),
                Cell::new(namespace.description),
                Cell::new(
                    humanize_relative_duration(namespace.created)
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
            ]);
        }

        println!("{table}",);
    }

    pub async fn namespace_create(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
    ) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(CreateNamespaceRequest {
            id: id.to_string(),
            name: name.unwrap_or_default(),
            description: description.unwrap_or_default(),
        });
        let response = client
            .create_namespace(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            })
            .into_inner();

        let namespace = response.namespace.unwrap();

        println!("Created namespace: [{}] {}", namespace.id, namespace.name);
    }

    pub async fn namespace_get(&self, id: &str) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(GetNamespaceRequest { id: id.to_string() });
        let response = client
            .get_namespace(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            })
            .into_inner();

        let namespace = response.namespace.unwrap();

        println!(
            "[{}] {} :: Created {}

  {}",
            namespace.id.green(),
            namespace.name,
            humanize_relative_duration(namespace.created).unwrap_or_else(|| "Unknown".to_string()),
            namespace.description
        );
    }
    pub async fn namespace_update(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
    ) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(GetNamespaceRequest { id: id.to_string() });
        let response = client
            .get_namespace(request)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Command failed; {}", e);
                process::exit(1);
            })
            .into_inner();

        let current_namespace = response.namespace.unwrap();

        let request = tonic::Request::new(UpdateNamespaceRequest {
            id: id.to_string(),
            name: name.unwrap_or(current_namespace.name),
            description: description.unwrap_or(current_namespace.description),
        });
        client.update_namespace(request).await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        println!("Updated namespace '{}'", id);
    }
    pub async fn namespace_delete(&self, id: &str) {
        let mut client = self.connect().await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        let request = tonic::Request::new(DeleteNamespaceRequest { id: id.to_string() });
        client.delete_namespace(request).await.unwrap_or_else(|e| {
            eprintln!("Command failed; {}", e);
            process::exit(1);
        });

        println!("Deleted namespace '{}'", id);
    }
}
