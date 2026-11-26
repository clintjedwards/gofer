//! Runs builds in BuildKit, in a container that the CLI starts on the user's Docker and keeps between builds.
//!
//! The container is privileged, which BuildKit needs to run build steps in containers of their own. Its cache lives in a
//! volume, so replacing the container doesn't lose it.
//!
//! Docker has a BuildKit of its own, which would save us running this container, but:
//!
//! - It keeps what it builds in Docker. With Docker's older image store, getting the image back out gives uncompressed
//!   layers, and compressing them ourselves added 27 seconds to an 889MB image, on every build. Ours sends the image
//!   back already compressed, and keeps compressed layers around for the next build.
//! - Its cache is the user's own `docker build` cache, so Gofer couldn't limit or empty it without touching theirs.
//! - Its version is whatever Docker ships. Ours is pinned, so builds behave the same on every machine.

use super::{
    context::{self, ChannelWriter, Contents},
    image::{self, Image},
};
use anyhow::{Context, Result, bail};
use bollard::{
    Docker, body_try_stream,
    container::LogOutput,
    errors::Error as DockerError,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    models::{ContainerCreateBody, HostConfig, Mount, MountType},
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, RemoveContainerOptions, RemoveVolumeOptions,
    },
};
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io, path::PathBuf, time::Duration};
use tokio::{io::AsyncWriteExt, sync::mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::io::{StreamReader, SyncIoBridge};

/// The BuildKit release we build with. It's pinned by digest so every machine runs exactly the same thing.
///
/// <https://github.com/moby/buildkit/releases/tag/v0.35.1>
const IMAGE: &str =
    "moby/buildkit:v0.35.1@sha256:baf6138eb8c37644800c7c913fba051e840ebb9ba13eaf92d92ef9a1712d0b4c";

const CONTAINER: &str = "gofer-buildkit";
const VOLUME: &str = "gofer-buildkit-cache";

/// Says which image and config a container was made with, so we can tell when it's out of date.
const VERSION_LABEL: &str = "gofer.buildkit.version";

const BUILD_SCRIPT: &str = include_str!("build.sh");

/// Builds are copied into this folder in the container, each in a folder of its own.
const BUILDS_FOLDER: &str = "gofer-builds";

/// Docker isn't running, or this user can't use it. What Docker said is the cause.
#[derive(Debug, thiserror::Error)]
#[error("Could not reach Docker; check that it's running and that you can use it")]
pub struct NoDocker;

/// BuildKit stopped with an error. These are the lines it printed about it.
#[derive(Debug, thiserror::Error)]
#[error("{}", .0.join("\n"))]
pub struct Failed(pub Vec<String>);

pub struct Builder {
    docker: Docker,
    cache_max_gb: u64,
}

impl Builder {
    pub fn new(cache_max_gb: u64) -> Result<Self> {
        let docker = Docker::connect_with_defaults()
            .context(NoDocker)?
            // The limit is on each whole request, and a big upload or a long build is one request.
            .with_timeout(Duration::from_secs(24 * 60 * 60));

        Ok(Self {
            docker,
            cache_max_gb,
        })
    }

    /// BuildKit's own cleanup only drops cache mounts once they're unused for 48 hours and over 512MB in total, and one
    /// Rust cache is bigger than that. So we use one rule instead, which keeps the whole cache under a size by dropping
    /// what was used longest ago.
    ///
    /// - <https://github.com/moby/buildkit/blob/v0.35.1/cmd/buildkitd/config/gcpolicy.go#L71-L90>
    /// - <https://github.com/moby/buildkit/blob/v0.35.1/docs/buildkitd.toml.md>
    fn config(&self) -> String {
        format!(
            "[worker.oci]\n  gc = true\n\n  [[worker.oci.gcpolicy]]\n    all = true\n    maxUsedSpace = \"{}GiB\"\n",
            self.cache_max_gb
        )
    }

    fn version(&self) -> String {
        let hash = Sha256::new_with_prefix(IMAGE)
            .chain_update(self.config())
            .finalize();
        hex::encode(&hash[..8])
    }

    /// Makes sure the builder is running and is the one this CLI expects, replacing it if it isn't.
    pub async fn start(&self) -> Result<()> {
        self.docker.ping().await.context(NoDocker)?;

        if self.docker.inspect_image(IMAGE).await.is_err() {
            self.docker
                .create_image(
                    Some(CreateImageOptions {
                        from_image: Some(IMAGE.into()),
                        ..Default::default()
                    }),
                    None,
                    None,
                )
                .try_collect::<Vec<_>>()
                .await
                .with_context(|| {
                    format!("Could not download {IMAGE}, which is what builds images; check the network connection")
                })?;
        }

        // Other builds, from this CLI or another one, can be starting it at the same moment. While one of them is part
        // way through making or removing the container, Docker tells the others that it isn't there (404) or that it's
        // busy (409). For example, Docker reserves the name before the container exists, so a build that was told the
        // name is taken can still be told there's no such container when it goes to start it. So on those we look
        // again; it settles within a moment.
        let version = self.version();
        let mut tries = 0;
        loop {
            match self.try_start(&version).await {
                Ok(()) => break,
                Err(e) if tries < 100 && matches!(status(&e), Some(404 | 409)) => {
                    tries += 1;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => return Err(e).context("Could not start the builder"),
            }
        }

        self.wait_until_ready().await
    }

    async fn try_start(&self, version: &str) -> Result<(), DockerError> {
        match self.docker.inspect_container(CONTAINER, None).await {
            Ok(container) => {
                let labels = container.config.and_then(|config| config.labels);
                let current = labels.as_ref().and_then(|labels| labels.get(VERSION_LABEL));

                // The old one has to be gone before the new one starts. Two BuildKits must never use the same volume
                // at once.
                //
                // It's removed by id because another build may have replaced it since we looked, and that new one
                // is the one we want.
                if current.map(String::as_str) != Some(version) {
                    let id = container.id.unwrap_or_default();
                    self.docker.remove_container(&id, Some(force())).await?;
                    self.create(version).await?;
                }
            }
            Err(e) if status(&e) == Some(404) => self.create(version).await?,
            Err(e) => return Err(e),
        }

        // Docker says 304 when it's already running.
        match self.docker.start_container(CONTAINER, None).await {
            Err(e) if status(&e) != Some(304) => Err(e),
            _ => Ok(()),
        }
    }

    async fn create(&self, version: &str) -> Result<(), DockerError> {
        // BuildKit reads its config when it starts. Writing it from the container's own command means it can't start
        // without it, even if another `gofer up` is the one that starts it.
        let entrypoint = [
            "/bin/sh",
            "-c",
            r#"mkdir -p /etc/buildkit && printf '%s' "$1" > /etc/buildkit/buildkitd.toml && exec buildkitd"#,
            "start-buildkit",
        ]
        .into_iter()
        .map(String::from)
        .chain([self.config()])
        .collect();

        let container = ContainerCreateBody {
            image: Some(IMAGE.into()),
            entrypoint: Some(entrypoint),
            labels: Some(HashMap::from([(VERSION_LABEL.into(), version.into())])),
            host_config: Some(HostConfig {
                privileged: Some(true),
                // Build steps start processes of their own, so something has to clean up after the ones that exit.
                init: Some(true),
                mounts: Some(vec![Mount {
                    target: Some("/var/lib/buildkit".into()),
                    source: Some(VOLUME.into()),
                    typ: Some(MountType::VOLUME),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: Some(CONTAINER.into()),
            ..Default::default()
        };

        self.docker
            .create_container(Some(options), container)
            .await
            .map(|_| ())
    }

    async fn wait_until_ready(&self) -> Result<()> {
        let mut last_output = String::new();

        for _ in 0..300 {
            let (code, output) = (self.run(&["buildctl", "debug", "workers"]).await)
                .context("The builder didn't start")?;
            if code == 0 {
                return Ok(());
            }

            last_output = output;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        bail!("The builder didn't start; {}", last_output.trim())
    }

    /// Runs a command in the builder, and returns its exit code and everything it printed.
    async fn run(&self, command: &[&str]) -> Result<(i64, String)> {
        let exec = self
            .docker
            .create_exec(
                CONTAINER,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(command.to_vec()),
                    ..Default::default()
                },
            )
            .await?;

        let StartExecResults::Attached { output, .. } =
            self.docker.start_exec(&exec.id, None).await?
        else {
            bail!("Docker didn't send the output of '{}'", command.join(" "));
        };

        let output: Vec<LogOutput> = output.try_collect().await?;
        let output = output.iter().map(LogOutput::to_string).collect();
        let code = self
            .docker
            .inspect_exec(&exec.id)
            .await?
            .exit_code
            .unwrap_or(-1);

        Ok((code, output))
    }

    /// Whether Docker runs on something other than amd64, like on an ARM Mac. Images are always for amd64, so building
    /// them there is emulated.
    pub async fn emulated(&self) -> Result<bool> {
        let version = self.docker.version().await.context(NoDocker)?;
        Ok(version.arch.as_deref() != Some("amd64"))
    }

    /// Builds an image from what's in `contents`, laid out the way [`super::dockerfile::Rendered`] describes. Every line
    /// BuildKit prints goes to `progress`; the progress itself is JSON. The builder has to be started first.
    ///
    /// When BuildKit fails, the error is a [`Failed`].
    pub async fn build(
        &self,
        contents: Contents,
        no_cache: bool,
        entrypoint: Option<&[String]>,
        command: Option<&[String]>,
        mut progress: impl FnMut(&str),
    ) -> Result<Image> {
        let folder = format!("{BUILDS_FOLDER}/{}", uuid::Uuid::now_v7().simple());

        if let Err(e) = self.upload(contents, &folder).await {
            // What did make it in would otherwise stay there.
            _ = self.run(&["rm", "-rf", &format!("/{folder}")]).await;
            return Err(e);
        }

        let no_cache = if no_cache { "--no-cache" } else { "" };
        let command_line = [
            "/bin/sh",
            "-c",
            BUILD_SCRIPT,
            "gofer-build",
            &format!("/{folder}"),
            no_cache,
        ];

        let exec = self
            .docker
            .create_exec(
                CONTAINER,
                CreateExecOptions {
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(false),
                    cmd: Some(command_line.to_vec()),
                    ..Default::default()
                },
            )
            .await
            .context("Could not start the build")?;

        let options = StartExecOptions {
            detach: false,
            tty: false,
            output_capacity: Some(64 * 1024),
        };
        let StartExecResults::Attached {
            mut output,
            mut input,
        } = self
            .docker
            .start_exec(&exec.id, Some(options))
            .await
            .context("Could not start the build")?
        else {
            bail!("Docker didn't attach to the build");
        };

        // build.sh stops the build once stdin closes. Dropping this doesn't close it though, only shutting it down does.
        let mut unpack_failed = false;

        let image_folder = image_folder()?;
        let (sender, receiver) = mpsc::channel::<io::Result<Bytes>>(16);
        let mut sender = Some(sender);
        let unpack = {
            let path = image_folder.path().to_path_buf();
            let reader = SyncIoBridge::new(StreamReader::new(ReceiverStream::new(receiver)));
            tokio::task::spawn_blocking(move || image::unpack(reader, &path))
        };

        // BuildKit writes its progress as one JSON object per line. Anything else is it saying what went wrong.
        let mut errors: Vec<String> = vec![];
        let mut partial_line: Vec<u8> = vec![];
        let mut add_line = |line: &[u8]| {
            let line = String::from_utf8_lossy(line);
            if line.trim().is_empty() {
                return;
            }

            progress(&line);
            if !line.starts_with('{') {
                errors.push(line.trim().to_string());
            }
        };

        while let Some(frame) = output.next().await {
            match frame.context("Lost the connection to the build")? {
                LogOutput::StdOut { message } => {
                    // Unpacking only stops early when something was wrong with the image, which it reports below.
                    // There's no point in building the rest.
                    if let Some(sending) = &sender
                        && sending.send(Ok(message)).await.is_err()
                    {
                        sender = None;
                        unpack_failed = true;
                        _ = input.shutdown().await;
                    }
                }
                LogOutput::StdErr { message } => {
                    partial_line.extend_from_slice(&message);

                    while let Some(end) = partial_line.iter().position(|byte| *byte == b'\n') {
                        let line: Vec<u8> = partial_line.drain(..=end).collect();
                        add_line(&line);
                    }
                }
                LogOutput::Console { .. } => {
                    bail!("Docker sent the build's output as a terminal's, so it can't be read")
                }
                LogOutput::StdIn { .. } => {}
            }
        }
        add_line(&partial_line);

        drop(sender);
        _ = input.shutdown().await;

        let code = self.docker.inspect_exec(&exec.id).await?.exit_code;
        let unpacked = unpack.await?;

        // When unpacking failed first, the build only failed because we stopped it, so the unpack error is the one that
        // says what went wrong.
        if code != Some(0) && !unpack_failed {
            if errors.is_empty() {
                errors.push(format!(
                    "The build stopped with exit code {}",
                    code.unwrap_or(-1)
                ));
            }
            return Err(Failed(errors).into());
        }

        let manifest = unpacked.context("Could not read the image BuildKit made")?;
        image::finish(image_folder, manifest, entrypoint, command)
    }

    /// Copies a build's files into the builder, writing the tar on another thread as it uploads.
    async fn upload(&self, contents: Contents, folder: &str) -> Result<()> {
        let (sender, receiver) = mpsc::channel::<io::Result<Bytes>>(4);

        let write = {
            let folder = folder.to_string();
            let errors = sender.clone();

            tokio::task::spawn_blocking(move || {
                let result = context::write(ChannelWriter::new(sender), &folder, &contents);

                // Ending the upload with an error, instead of ending it early, makes sure Docker doesn't keep half of it.
                if let Err(e) = &result {
                    _ = errors.blocking_send(Err(io::Error::new(e.kind(), e.to_string())));
                }

                result
            })
        };

        let options = bollard::query_parameters::UploadToContainerOptions {
            path: "/".into(),
            ..Default::default()
        };
        let uploaded = self
            .docker
            .upload_to_container(
                CONTAINER,
                Some(options),
                body_try_stream(ReceiverStream::new(receiver)),
            )
            .await;

        // When reading a file failed, that's the more useful error since it says which file.
        write
            .await?
            .context("Could not send the files to the builder")?;
        uploaded.context("Could not send the files to the builder")?;
        Ok(())
    }

    /// Empties the build cache. Returns how much space that freed, as BuildKit puts it.
    pub async fn prune(&self) -> Result<String> {
        self.start().await?;

        let (code, output) = self.run(&["buildctl", "prune", "--all"]).await?;
        if code != 0 {
            bail!("Could not empty the builder's cache; {}", output.trim());
        }

        let freed = output.lines().find_map(|line| line.strip_prefix("Total:"));
        Ok(freed.unwrap_or("0B").trim().to_string())
    }

    /// Removes the builder and its cache. Returns whether there was anything to remove.
    pub async fn remove(&self) -> Result<bool> {
        let container = self.remove_container().await?;

        let volume = match self
            .docker
            .remove_volume(VOLUME, None::<RemoveVolumeOptions>)
            .await
        {
            Ok(()) => true,
            Err(e) if status(&e) == Some(404) => false,
            Err(e) => return Err(e).context("Could not remove the builder's cache"),
        };

        Ok(container || volume)
    }

    async fn remove_container(&self) -> Result<bool> {
        match self.docker.remove_container(CONTAINER, Some(force())).await {
            Ok(()) => Ok(true),
            Err(e) if status(&e) == Some(404) => Ok(false),
            Err(e) => Err(e).context("Could not remove the builder"),
        }
    }
}

/// Removes a container even if it's running.
fn force() -> RemoveContainerOptions {
    RemoveContainerOptions {
        force: true,
        ..Default::default()
    }
}

fn status(e: &DockerError) -> Option<u16> {
    match e {
        DockerError::DockerResponseServerError { status_code, .. } => Some(*status_code),
        _ => None,
    }
}

/// Makes a folder to unpack an image into, which removes itself when it's dropped.
fn image_folder() -> Result<tempfile::TempDir> {
    // Not /tmp, which is often in memory, and images can be large.
    let parent: PathBuf = dirs::cache_dir()
        .context("Could not find a cache folder to unpack images in")?
        .join("gofer")
        .join("images");
    std::fs::create_dir_all(&parent)
        .with_context(|| format!("Could not create '{}'", parent.display()))?;

    // A CLI that was killed never got to remove its folder.
    let day_ago = std::time::SystemTime::now() - Duration::from_secs(24 * 60 * 60);
    for old in std::fs::read_dir(&parent)?.flatten() {
        let modified = old.metadata().and_then(|metadata| metadata.modified());
        if modified.is_ok_and(|modified| modified < day_ago) {
            _ = std::fs::remove_dir_all(old.path());
        }
    }

    tempfile::Builder::new()
        .prefix("build-")
        .tempdir_in(&parent)
        .with_context(|| format!("Could not create a folder in '{}'", parent.display()))
}
