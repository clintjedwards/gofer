use crate::cli::{Cli, archive};
use anyhow::{Context, Result, bail};
use bytes::BufMut;
use clap::{Args, Subcommand};
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use futures::StreamExt;
use polyfmt::{Format, pause, print, println, resume, success};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Args, Clone)]
pub struct ObjectSubcommands {
    #[clap(subcommand)]
    pub command: ObjectCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,

    /// Pipeline Identifier. Gofer sets GOFER_PIPELINE_ID inside every task.
    #[clap(long, global = true, env = "GOFER_PIPELINE_ID")]
    pub pipeline: Option<String>,

    /// Run Identifier. Gofer sets GOFER_RUN_ID inside every task.
    #[clap(long, global = true, env = "GOFER_RUN_ID")]
    pub run: Option<u64>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ObjectCommands {
    /// List all objects from the run specific store.
    List,

    /// Read an object from the run store
    #[command(after_help = r#"Examples:
  gofer run object get test --pipeline simple --run 1 > /tmp/gofer_test
  gofer run object get test --stringify
  gofer run object get dist --extract ./dist
"#)]
    Get {
        key: String,

        /// Attempt to print the object as a utf-8 string.
        #[arg(short, long, default_value = "false", conflicts_with = "extract")]
        stringify: bool,

        /// Extract an object uploaded from a directory into this directory, creating it if needed.
        #[arg(short, long, value_name = "DIR")]
        extract: Option<PathBuf>,
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
    ///
    /// If the path is a directory it's uploaded as a gzipped tarball; use `get --extract` to unpack it.
    #[command(after_help = r#"Examples:
  gofer run object put test ~/.bin/gofer --pipeline simple --run 1
  gofer run object put dist ./dist
  echo "hello" | gofer run object put test @
"#)]
    Put {
        key: String,

        /// Path to the object file or directory. Use an @ character to pass object via stdin instead.
        path: String,

        /// Replace value if it exists.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    /// Remove a run object.
    Delete { key: String },
}

impl Cli {
    pub async fn handle_run_object_subcommands(&self, command: ObjectSubcommands) -> Result<()> {
        let pipeline_id = command
            .pipeline
            .context("Pipeline id not provided; pass --pipeline or set GOFER_PIPELINE_ID")?;
        let run_id = command
            .run
            .context("Run id not provided; pass --run or set GOFER_RUN_ID")?;

        match command.command {
            ObjectCommands::List => {
                self.run_object_list(command.namespace, &pipeline_id, run_id)
                    .await
            }
            ObjectCommands::Get {
                key,
                stringify,
                extract,
            } => {
                self.run_object_get(
                    command.namespace,
                    &pipeline_id,
                    run_id,
                    &key,
                    stringify,
                    extract,
                )
                .await
            }
            ObjectCommands::Put { key, path, force } => {
                self.run_object_put(command.namespace, &pipeline_id, run_id, &key, &path, force)
                    .await
            }
            ObjectCommands::Delete { key } => {
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
            .load_style(comfy_table::presets::ASCII_MARKDOWN)
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
        extract: Option<PathBuf>,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let object = self
            .client
            .get_run_object(&namespace, pipeline_id, run_id, key)
            .await
            .context("Could not successfully retrieve object from Gofer api")?;
        let mut object = object.into_inner_stream();

        if let Some(dest) = extract {
            archive::untar_into(object, dest.clone()).await?;
            success!(
                "Successfully extracted object '{}' into '{}'",
                key,
                dest.display()
            );
            return Ok(());
        }

        if stringify {
            let mut buffer = bytes::BytesMut::with_capacity(1024); // 1kb

            while let Some(chunk) = object.next().await {
                let chunk = chunk?;

                let remaining_space = buffer.capacity() - buffer.len();

                if remaining_space > chunk.len() {
                    buffer.put(chunk);
                } else {
                    bail!("Could not stringify object; object larger than 1KB");
                }
            }

            println!("{}", String::from_utf8_lossy(&buffer));
            return Ok(());
        }

        // Raw object bytes go straight to stdout so we pause the formatter while writing them.
        pause!();

        let mut stdout = std::io::stdout();

        let result = async {
            while let Some(chunk) = object.next().await {
                let chunk = chunk?;

                stdout.write_all(&chunk)?;
            }

            Ok(())
        }
        .await;

        resume!();

        result
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

        print!("Uploading object"; vec![Format::Spinner]);

        let body_stream = if path == "@" {
            let stdin = tokio::io::stdin();
            reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(stdin))
        } else {
            let path = PathBuf::from(path);

            if path.is_dir() {
                archive::tar_directory(path)
            } else {
                let file = tokio::fs::File::open(path)
                    .await
                    .context("Could not open file")?;
                reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(file))
            }
        };

        let object = &self
            .client
            .put_run_object(
                &namespace,
                pipeline_id,
                run_id,
                key,
                Some(force),
                body_stream,
            )
            .await
            .context("Could not successfully push object to Gofer api")?
            .object;

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
