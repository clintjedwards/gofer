use crate::cli::Cli;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use polyfmt::{println, success, Spinner};
use std::io::{Read, Write};

#[derive(Debug, Args, Clone)]
pub struct ObjectSubcommands {
    #[clap(subcommand)]
    pub command: ObjectCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ObjectCommands {
    /// List all objects from the pipeline specific store.
    List {
        /// Pipeline Identifier.
        id: String,
    },

    /// Read an object from the pipeline store
    Get {
        /// Pipeline Identifier.
        id: String,

        key: String,

        /// Attempt to print the object as a utf-8 string.
        #[arg(short, long, default_value = "false")]
        stringify: bool,
    },

    /// Write an object into the pipeline store
    ///
    /// The pipeline store allows storage of objects as key-values pairs that many runs might need to reference. These pipeline
    /// level objects are kept forever until the limit of number of pipeline objects is reached(this may be different depending
    /// on configuration). Once this limit is reached the _oldest_ object will be removed to make space for the new object.
    ///
    /// You can store both regular text values or read in entire files using the '@' symbol and piping.
    Put {
        /// Pipeline Identifier.
        id: String,

        key: String,

        /// Path the object file. Use an @ character to pass object via stdin instead.
        path: String,

        /// Replace value if it exists.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    /// Remove a pipeline object.
    Delete {
        /// Pipeline Identifier.
        id: String,

        key: String,
    },
}

impl Cli {
    pub async fn handle_pipeline_object_subcommands(
        &self,
        command: ObjectSubcommands,
    ) -> Result<()> {
        let cmds = command.command;
        match cmds {
            ObjectCommands::List { id } => self.pipeline_object_list(command.namespace, &id).await,
            ObjectCommands::Get { id, key, stringify } => {
                self.pipeline_object_get(command.namespace, &id, &key, stringify)
                    .await
            }
            ObjectCommands::Put {
                id,
                key,
                path,
                force,
            } => {
                self.pipeline_object_put(command.namespace, &id, &key, &path, force)
                    .await
            }
            ObjectCommands::Delete { id, key } => {
                self.pipeline_object_delete(command.namespace, &id, &key)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn pipeline_object_list(&self, namespace_id: Option<String>, id: &str) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let objects = self
            .client
            .list_pipeline_objects(&namespace, id)
            .await
            .context("Could not successfully retrieve pipeline objects from Gofer api")?
            .into_inner()
            .objects;

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

        for object in objects {
            table.add_row(vec![
                Cell::new(object.key).fg(Color::Green),
                Cell::new(
                    self.format_time(object.created)
                        .unwrap_or("Unknown".to_string()),
                ),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn pipeline_object_get(
        &self,
        namespace_id: Option<String>,
        id: &str,
        key: &str,
        stringify: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let object = &self
            .client
            .get_pipeline_object(&namespace, id, key)
            .await
            .context("Could not successfully retrieve object from Gofer api")?
            .object;

        if stringify {
            std::println!("{}", String::from_utf8_lossy(object))
        } else {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(object)?;
            handle.flush()?;
        }

        Ok(())
    }

    pub async fn pipeline_object_put(
        &self,
        namespace_id: Option<String>,
        id: &str,
        key: &str,
        path: &str,
        force: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let mut data: Vec<u8> = Vec::new();

        let spinner = Spinner::create("Reading object");

        if path == "@" {
            std::io::stdin()
                .read_to_end(&mut data)
                .context("Could not read object from stdin")?;
        } else {
            let path = std::path::PathBuf::from(path);
            let mut file = std::fs::File::open(path).context("Could not open object file")?;
            file.read_to_end(&mut data)
                .context("Could not read object file")?;
        };

        spinner.set_message("Uploading object".into());

        let object = &self
            .client
            .put_pipeline_object(
                &namespace,
                id,
                &gofer_sdk::api::types::PutPipelineObjectRequest {
                    content: data,
                    force,
                    key: key.into(),
                },
            )
            .await
            .context("Could not successfully push object to Gofer api")?
            .object;

        drop(spinner);

        success!("Successfully uploaded object '{}'", object.key);

        Ok(())
    }

    pub async fn pipeline_object_delete(
        &self,
        namespace_id: Option<String>,
        id: &str,
        key: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .delete_pipeline_object(&namespace, id, key)
            .await
            .context("Could not delete object")?;

        success!("Successfully removed object '{}'", key);
        Ok(())
    }
}
