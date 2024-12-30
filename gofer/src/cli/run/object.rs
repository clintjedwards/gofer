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
    /// List all objects from the run specific store.
    List {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,
    },

    /// Read an object from the run store
    Get {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        key: String,

        /// Attempt to print the object as a utf-8 string.
        #[arg(short, long, default_value = "false")]
        stringify: bool,
    },

    /// Write an object into the run store
    ///
    /// Gofer has two ways to temporarily store objects that might be useful.
    ///
    /// This command allows users to store objects at the "run" level in a key-object fashion. Run level objects are
    /// great for storing things that need to be cached only for the communication between tasks.
    ///
    /// Run objects are kept individual to each run and removed after a certain run limit. This means that after a certain
    /// amount of runs for a particular pipeline a run's objects will be discarded. The limit of amount of objects you can
    /// store per run is of a much higher limit.
    Put {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        key: String,

        /// Path the object file. Use an @ character to pass object via stdin instead.
        path: String,

        /// Replace value if it exists.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    /// Remove a run object.
    Delete {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        key: String,
    },
}

impl Cli {
    pub async fn handle_run_object_subcommands(&self, command: ObjectSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            ObjectCommands::List {
                pipeline_id,
                run_id,
            } => {
                self.run_object_list(command.namespace, &pipeline_id, run_id)
                    .await
            }
            ObjectCommands::Get {
                pipeline_id,
                run_id,
                key,
                stringify,
            } => {
                self.run_object_get(command.namespace, &pipeline_id, run_id, &key, stringify)
                    .await
            }
            ObjectCommands::Put {
                pipeline_id,
                run_id,
                key,
                path,
                force,
            } => {
                self.run_object_put(command.namespace, &pipeline_id, run_id, &key, &path, force)
                    .await
            }
            ObjectCommands::Delete {
                pipeline_id,
                run_id,
                key,
            } => {
                self.run_object_delete(command.namespace, &pipeline_id, run_id, &key)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn run_object_list(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let objects = self
            .client
            .list_run_objects(&namespace, pipeline_id, run_id)
            .await
            .context("Could not successfully retrieve run objects from Gofer api")?
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

    pub async fn run_object_get(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        key: &str,
        stringify: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let object = &self
            .client
            .get_run_object(&namespace, pipeline_id, run_id, key)
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

    pub async fn run_object_put(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
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
            .put_run_object(
                &namespace,
                pipeline_id,
                run_id,
                &gofer_sdk::api::types::PutRunObjectRequest {
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

    pub async fn run_object_delete(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        key: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .delete_run_object(&namespace, pipeline_id, run_id, key)
            .await
            .context("Could not delete object")?;

        success!("Successfully removed object '{}'", key);
        Ok(())
    }
}
