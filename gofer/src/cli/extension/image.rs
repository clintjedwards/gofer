//! `gofer extension image`: the extension images stored in Gofer.

use crate::{
    api::{extension_images::StoredImageRef, extensions::reconcile},
    cli::{
        Cli, api_error_message,
        builder::{self, Builder},
        up::images::UploadTarget,
    },
};
use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand};
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement, presets::ASCII_MARKDOWN};
use gofer_sdk::{
    api::types::PutExtensionImageRequest,
    config::{Build, Steps},
};
use polyfmt::{Format, print, println, success};
use std::path::{Path, PathBuf};

#[derive(Debug, Args, Clone)]
pub struct ImageSubcommands {
    #[clap(subcommand)]
    pub command: ImageCommands,
}

/// An extension's manifest can name an image stored in Gofer, as `gofer://<name>:<tag>`, instead of one in a registry.
/// Gofer's own extensions are stored that way, from Gofer itself; these commands store and manage others.
#[derive(Debug, Subcommand, Clone)]
pub enum ImageCommands {
    /// Store an extension image in Gofer.
    ///
    /// Copy an image from a registry with --from, which doesn't need Docker, or build one from a Dockerfile with
    /// --dockerfile, the way `gofer up` builds a task's image. Logins for registries come from Docker's config, also
    /// like `gofer up`, and only the parts Gofer doesn't have yet are uploaded.
    ///
    /// A name and tag is fixed once it's stored. To upgrade an extension, store it under a new tag, point its manifest
    /// at that, and run `gofer extension reload`.
    #[command(after_help = r#"Examples:
  gofer extension image push slack:1.2.0 --from ghcr.io/example/slack:1.2.0
  gofer extension image push slack:1.2.0 --dockerfile ./Dockerfile
"#)]
    Push {
        /// The name and tag to store it as, like slack:1.2.0. A manifest names it as gofer://slack:1.2.0.
        image: String,

        /// A registry image to copy, like ghcr.io/example/slack:1.2.0.
        #[arg(long, conflicts_with_all = ["dockerfile", "context"], required_unless_present = "dockerfile")]
        from: Option<String>,

        /// A Dockerfile to build the image from.
        #[arg(long)]
        dockerfile: Option<PathBuf>,

        /// The folder the Dockerfile can use files from. Defaults to the Dockerfile's folder.
        #[arg(long, requires = "dockerfile")]
        context: Option<PathBuf>,

        /// Replace the image this name and tag point to. Extensions using it get it the next time they restart.
        #[arg(short, long, default_value = "false")]
        force: bool,
    },

    /// List the extension images stored in Gofer.
    List,

    /// Delete an extension image. It's refused while an extension uses it.
    Delete {
        /// The name and tag it's stored as, like slack:1.2.0.
        image: String,
    },
}

impl Cli {
    pub async fn handle_extension_image_subcommands(
        &self,
        command: ImageSubcommands,
    ) -> Result<()> {
        match command.command {
            ImageCommands::Push {
                image,
                from,
                dockerfile,
                context,
                force,
            } => {
                self.extension_image_push(&image, from, dockerfile, context, force)
                    .await
            }
            ImageCommands::List => self.extension_image_list().await,
            ImageCommands::Delete { image } => self.extension_image_delete(&image).await,
        }
    }

    async fn extension_image_push(
        &self,
        image: &str,
        from: Option<String>,
        dockerfile: Option<PathBuf>,
        context: Option<PathBuf>,
        force: bool,
    ) -> Result<()> {
        let image = StoredImageRef::from_name_and_tag(image)?;

        // Gofer refuses these too, but only once the image is uploaded.
        if reconcile::is_provided_extension(&image.name) {
            bail!(
                "'{}' is the name of one of Gofer's own extensions, whose images come with Gofer; pick another name",
                image.name
            );
        }

        // Built the same way as a task whose config is in this folder: a base and nothing else, or a Dockerfile.
        let folder = std::env::current_dir().context("Could not find the current folder")?;
        let build = match (from, dockerfile) {
            (Some(from), _) => Build::Steps(Steps {
                base: from,
                steps: vec![],
                keep: None,
            }),
            (None, Some(dockerfile)) => {
                let context = context.unwrap_or_else(|| {
                    dockerfile
                        .parent()
                        .filter(|parent| !parent.as_os_str().is_empty())
                        .unwrap_or(Path::new("."))
                        .to_path_buf()
                });
                Build::Dockerfile {
                    path: dockerfile.to_string_lossy().into(),
                    context: context.to_string_lossy().into(),
                }
            }
            (None, None) => bail!("Say where the image comes from with --from or --dockerfile"),
        };

        let sources = builder::sources(&build, &folder)?;
        let source_hash = builder::source_hash(&sources)?;
        let docker = match builder::needs_docker(&build) {
            true => Some(
                Builder::new(self.conf.builder_cache_max_gb)
                    .context("Building from a Dockerfile needs Docker")?,
            ),
            false => None,
        };

        let name = format!("extension-images/{}", image.name);
        let task = builder::Task {
            name: &name,
            build: &build,
            entrypoint: None,
            command: None,
        };

        print!("Getting {image} ready"; vec![Format::Spinner]);
        let built = builder::build(
            docker.as_ref(),
            &task,
            &sources,
            &source_hash,
            false,
            |step| {
                print!("Building {image}: {step}"; vec![Format::Spinner]);
            },
        )
        .await?;

        let digest = built.image.manifest.to_string();
        self.upload_images(UploadTarget::ExtensionImages, &[(vec![name], built)])
            .await?;

        self.client
            .put_extension_image(
                &image.name,
                &image.tag,
                &PutExtensionImageRequest {
                    digest: digest.clone(),
                    force,
                },
            )
            .await
            .map_err(|e| anyhow!("Could not store {image}; {}", api_error_message(e)))?;

        success!("Stored {image} ({digest})");
        println!(
            "  Point an extension's manifest at it with image = \"{image}\", then run 'gofer extension reload'."
        );
        Ok(())
    }

    async fn extension_image_list(&self) -> Result<()> {
        let images = self
            .client
            .list_extension_images()
            .await
            .map_err(|e| anyhow!("Could not list extension images; {}", api_error_message(e)))?
            .into_inner()
            .images;

        let mut table = comfy_table::Table::new();
        table
            .load_style(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(["image", "digest", "used by", "stored"].map(|header| {
                Cell::new(header)
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue)
            }));

        for image in images {
            let reference = StoredImageRef {
                name: image.name,
                tag: image.tag,
            };
            let reference = match image.builtin {
                true => format!("{reference} (comes with Gofer)"),
                false => reference.to_string(),
            };

            // The start of a digest is enough to tell images apart, the way Docker shows them.
            let digest = image.digest.trim_start_matches("sha256:");
            table.add_row(vec![
                Cell::new(reference).fg(Color::Green),
                Cell::new(&digest[..digest.len().min(12)]),
                Cell::new(image.used_by.join(", ")),
                Cell::new(self.format_time(image.created).unwrap_or_default()),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    async fn extension_image_delete(&self, image: &str) -> Result<()> {
        let image = StoredImageRef::from_name_and_tag(image)?;

        self.client
            .delete_extension_image(&image.name, &image.tag)
            .await
            .map_err(|e| anyhow!("Could not delete {image}; {}", api_error_message(e)))?;

        success!("Deleted {image}");
        Ok(())
    }
}
