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
}

#[derive(Debug, Subcommand, Clone)]
pub enum ObjectCommands {
    /// List all objects from the pipeline specific store.
    List,

    /// Read an object from the pipeline store
    #[command(after_help = r#"Examples:
  gofer pipeline object get test --pipeline simple > /tmp/gofer_test
  gofer pipeline object get test --stringify
  gofer pipeline object get cache --extract ./node_modules
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

    /// Write an object into the pipeline store
    ///
    /// The pipeline store allows storage of objects as key-values pairs that many runs might need to reference. These pipeline
    /// level objects are kept forever until the limit of number of pipeline objects is reached(this may be different depending
    /// on configuration). Once this limit is reached the _oldest_ object will be removed to make space for the new object.
    ///
    /// You can store both regular text values or read in entire files using the '@' symbol and piping.
    ///
    /// If the path is a directory it's uploaded as a gzipped tarball; use `get --extract` to unpack it.
    #[command(after_help = r#"Examples:
  gofer pipeline object put test ~/.bin/gofer --pipeline simple
  gofer pipeline object put cache ./node_modules --force
  echo "hello" | gofer pipeline object put test @
"#)]
    Put {
        key: String,

        /// Path to the object file or directory. Use an @ character to pass object via stdin instead.
        path: String,

        /// Replace value if it exists.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    /// Remove a pipeline object.
    Delete { key: String },
}

impl Cli {
    pub async fn handle_pipeline_object_subcommands(
        &self,
        command: ObjectSubcommands,
    ) -> Result<()> {
        let id = command
            .pipeline
            .context("Pipeline id not provided; pass --pipeline or set GOFER_PIPELINE_ID")?;

        match command.command {
            ObjectCommands::List => self.pipeline_object_list(command.namespace, &id).await,
            ObjectCommands::Get {
                key,
                stringify,
                extract,
            } => {
                self.pipeline_object_get(command.namespace, &id, &key, stringify, extract)
                    .await
            }
            ObjectCommands::Put { key, path, force } => {
                self.pipeline_object_put(command.namespace, &id, &key, &path, force)
                    .await
            }
            ObjectCommands::Delete { key } => {
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

    pub async fn pipeline_object_get(
        &self,
        namespace_id: Option<String>,
        id: &str,
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
            .get_pipeline_object(&namespace, id, key)
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
            .put_pipeline_object(&namespace, id, key, Some(force), body_stream)
            .await
            .context("Could not successfully push object to Gofer api")?
            .object;

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
