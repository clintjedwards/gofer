//! Starts, stops, and watches the Docker containers that tasks and extensions run in.

use crate::api::task_executions::{
    Phase, TaskExecution, TimelineEntry, task_execution_container_id,
};
use anyhow::Context;
use bollard::exec::{CreateExecOptions, StartExecOptions};
use bytes::Bytes;
use dashmap::DashMap;
use futures::Stream;
use futures::stream::{StreamExt, TryStreamExt};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, pin::Pin, sync::Arc};
use strum::{Display, EnumString};
use tokio::io::AsyncWrite;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info, instrument, trace, warn};

/// The platform every task image is built for. Docker is asked for it when a task's container is made, so a machine
/// that isn't amd64 runs it under emulation instead of failing.
const TASK_PLATFORM: &str = "linux/amd64";

/// Where the run folder's volume is mounted, and the name of the folder in it that tasks use, which makes the folder
/// `/gofer/run`. See [`Containers::open_run_folder`] for why they're not the same.
const RUN_VOLUME_PATH: &str = "/gofer";
const RUN_FOLDER_NAME: &str = "run";

/// Where every task finds the gofer CLI, which is Gofer's own binary. It's under `/gofer` with the run folder, which
/// images aren't meant to use, and its folder goes on the end of the task's PATH.
const TASK_CLI_FOLDER: &str = "/gofer/bin";
const TASK_CLI_PATH: &str = "/gofer/bin/gofer";

/// The PATH Docker gives a container whose image doesn't set one.
const DEFAULT_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

/// The labels on a run folder's volume, which say whose run it belongs to.
const NAMESPACE_LABEL: &str = "gofer.namespace";
const PIPELINE_LABEL: &str = "gofer.pipeline";
const RUN_LABEL: &str = "gofer.run";

/// A folder that every task in a run shares, at `/gofer/run`, so one task can leave files for the tasks after it. It's
/// a Docker volume of its own, created by the run's first task and removed when the run ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFolder {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub run_id: u64,
}

impl RunFolder {
    /// The volume's name. Ids can't have underscores in them, so two runs never get the same one.
    pub fn volume(&self) -> String {
        format!(
            "gofer_{}_{}_{}",
            self.namespace_id, self.pipeline_id, self.run_id
        )
    }

    fn labels(&self) -> HashMap<String, String> {
        HashMap::from([
            (NAMESPACE_LABEL.into(), self.namespace_id.clone()),
            (PIPELINE_LABEL.into(), self.pipeline_id.clone()),
            (RUN_LABEL.into(), self.run_id.to_string()),
        ])
    }

    fn from_labels(labels: &HashMap<String, String>) -> Option<Self> {
        Some(Self {
            namespace_id: labels.get(NAMESPACE_LABEL)?.clone(),
            pipeline_id: labels.get(PIPELINE_LABEL)?.clone(),
            run_id: labels.get(RUN_LABEL)?.parse().ok()?,
        })
    }
}

/// Represents different container failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ContainerError {
    /// Failed to communicate with Docker due to network error or other.
    #[error("could not connect to Docker; {0}")]
    Connection(String),

    /// Container requested by name could not be found.
    #[error("container not found; {0}")]
    NoSuchContainer(String),

    /// Image requested by name could not be found.
    #[error("image not found; {0}")]
    NoSuchImage(String),

    /// Docker's copy of an image from Gofer's store doesn't have the layers Gofer checked.
    #[error(
        "Docker's copy of the image doesn't have the layers it was stored with, so it wasn't run. Docker read the image \
         differently from how Gofer checked it."
    )]
    ImageMismatch,

    /// An expected and unknown error has occurred.
    #[error("unexpected Docker error occurred; {0}")]
    Unknown(String),
}

#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerState {
    Unknown,
    Running,
    Paused,
    Restarting,
    Exited,
}

/// Private repositories sometimes require authentication.
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

// By the time it gets here the password is the real one, so keep it out of anything that debug prints a request.
impl std::fmt::Debug for RegistryAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryAuth")
            .field("user", &self.user)
            .field("pass", &"[redacted]")
            .finish()
    }
}

impl From<crate::api::RegistryAuth> for RegistryAuth {
    fn from(ra: crate::api::RegistryAuth) -> Self {
        Self {
            user: ra.user,
            pass: ra.pass,
        }
    }
}

/// Where a container's image comes from.
#[derive(Debug)]
pub enum Image {
    /// One that Docker pulls from a registry, like an extension's.
    Registry {
        /// The image's name; a tag can be included.
        name: String,

        auth: Option<RegistryAuth>,

        /// Pull it even when Docker already has an image by that name, for tags that move, like `latest`.
        always_pull: bool,
    },

    /// One that's already in Docker, because Gofer loaded it from its own store with [`Containers::load_image`]. A
    /// task's image is always one of these.
    Loaded(String),
}

#[derive(Debug)]
pub struct StartContainerRequest {
    /// A unique identifier to identify the container with.
    pub id: String,

    pub image: Image,

    /// Environment variables to be passed to the container.
    pub variables: HashMap<String, String>,

    /// Only needed by extensions; spin the container up with appropriate networking settings such that Gofer can
    /// connect to it.
    pub networking: Option<u16>,

    /// Replaces container's entrypoint with a custom one.
    pub entrypoint: Option<Vec<String>>,

    /// Replaces container's cmd instruction with a custom one.
    pub command: Option<Vec<String>>,

    /// Only for tasks; the run's folder to mount, which is created if this is the run's first task.
    pub run_folder: Option<RunFolder>,

    /// Only for tasks; mounts Gofer's own binary at `/gofer/bin/gofer` and adds `/gofer/bin` to the end of PATH, when
    /// this Gofer is a build that can run in any image. See [`task_cli`].
    pub gofer_cli: bool,

    /// Where to report the phases of starting the container as they begin. Lets task executions show where their time
    /// went before the container was running.
    pub timeline: Option<UnboundedSender<TimelineEntry>>,
}

#[derive(Debug)]
pub struct StartContainerResponse {
    /// An endpoint that only is returned for containers with networking set to on.
    pub url: Option<String>,
}

#[derive(Debug)]
pub struct StopContainerRequest {
    /// A unique identifier to identify the container with.
    pub id: String,

    /// The total time Docker should wait for a graceful stop before issuing a SIGKILL in seconds.
    /// 0 means send SIGKILL immediately.
    pub timeout: i64,
}

#[derive(Debug)]
pub struct GetStateRequest {
    /// Unique identifier for container to stop.
    pub id: String,
}

#[derive(Debug)]
pub struct GetStateResponse {
    /// In the event that the container is in a "complete" state; the exit code of that container.
    pub exit_code: Option<u8>,

    /// The current state of the container, state referencing how complete the container process of running is.
    pub state: ContainerState,
}

#[derive(Debug)]
pub struct GetLogsRequest {
    /// Unique identifier for container to get logs of.
    pub id: String,
}

pub struct AttachContainerRequest {
    pub id: String,
    pub command: Vec<String>,
}

pub struct AttachContainerResponse {
    pub output: Pin<Box<dyn Stream<Item = Result<Log, ContainerError>> + Send>>,
    pub input: Pin<Box<dyn AsyncWrite + Send>>,
}

/// Represents a single log line/entry from a particular container.
#[derive(Debug)]
pub enum Log {
    Unknown,
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    #[allow(dead_code)]
    Stdin(Vec<u8>),
    Console(Vec<u8>),
}

fn format_env_var(key: &str, value: &str) -> String {
    format!("{key}={value}")
}

/// Picks PATH out of an image's environment, which Docker lists as `KEY=value` strings, or the one Docker uses when
/// the image doesn't set it.
fn image_path_from_env(env: &[String]) -> String {
    env.iter()
        .find_map(|var| var.strip_prefix("PATH="))
        .unwrap_or(DEFAULT_PATH)
        .to_string()
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub prune: bool,
    pub prune_interval_seconds: u64,
    /// The total amount of time any request is allowed to be pending for in seconds.
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct Containers {
    client: bollard::Docker,

    /// A lock for each image being loaded, by name, so tasks that start at the same time don't each load the same one.
    loads: Arc<DashMap<String, Arc<tokio::sync::Mutex<()>>>>,

    /// Gofer's own binary, which tasks get as the gofer CLI. None when this build can't run in every image.
    cli: Option<PathBuf>,
}

/// Finds Gofer's own binary, so tasks can use it as the gofer CLI. Only a static build runs in every image: one
/// linked against glibc won't start on Alpine or distroless, and the error it gives there, "no such file or
/// directory", doesn't say why. So a build like that keeps itself out of tasks.
fn task_cli() -> Option<PathBuf> {
    if !cfg!(target_feature = "crt-static") {
        info!(
            "this Gofer isn't a static build, so tasks won't get the gofer CLI at {TASK_CLI_PATH}; build it for \
             x86_64-unknown-linux-musl, which the repository's .cargo/config.toml does"
        );
        return None;
    }

    match std::env::current_exe() {
        Ok(path) => Some(path),
        Err(e) => {
            warn!(error = %e, "could not find Gofer's own binary, so tasks won't get the gofer CLI");
            None
        }
    }
}

impl Containers {
    #[instrument(fields(origin = "containers"))]
    pub async fn new(config: &Config) -> Result<Self, ContainerError> {
        let config = config.clone();
        let client = bollard::Docker::connect_with_socket_defaults().map_err(|e| {
            ContainerError::Connection(format!(
                "{}; Make sure the Docker daemon is installed and running.",
                e
            ))
        })?;
        let client = client.with_timeout(tokio::time::Duration::from_secs(config.timeout_seconds));
        let prune_client = client.clone();

        // Check that we can actually get a connection.
        let version = client.version().await.map_err(|e| {
            ContainerError::Connection(format!(
                "{}; Make sure the Docker daemon is installed and running.",
                e
            ))
        })?;

        // We periodically need to clean up docker assets so we don't run out of disk space.
        // We perform it very infrequently though, in order to give operators time to diagnose
        // any potential issues they might be having with a particular container.
        if config.prune {
            tokio::spawn(prune_containers(
                prune_client,
                config.prune_interval_seconds,
            ));
        }

        let architecture = version.arch.clone().unwrap_or_default();
        if architecture != "amd64" {
            warn!(
                architecture,
                "Docker isn't running on amd64, but task images are always built for {TASK_PLATFORM}; tasks will \
                 run under emulation, which can be slow"
            );
        }

        debug!(
            version = version.version.unwrap_or_default(),
            "Successfully connected to Docker"
        );

        Ok(Self {
            client,
            loads: Arc::new(DashMap::new()),
            cli: task_cli(),
        })
    }

    /// Makes sure Docker has an image, calling `archive` for a tar in the format `docker load` takes when it doesn't.
    /// Returns the diff ids of the image's layers as Docker has them, which is how the caller can check that Docker's
    /// copy is the image it meant. With `replace`, Docker's copy goes first and the image is always loaded.
    ///
    /// Two loads of the same image run one after the other, so the second finds it already there. `loading` is called
    /// first when there's a wait for an image to load, whether this loads it or waits for another load to finish.
    pub async fn load_image<S>(
        &self,
        name: &str,
        replace: bool,
        loading: impl FnOnce(),
        archive: impl FnOnce() -> S,
    ) -> Result<Vec<String>, ContainerError>
    where
        S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
    {
        let mut loading = Some(loading);
        let mut waiting = || {
            if let Some(loading) = loading.take() {
                loading();
            }
        };

        let lock = self.loads.entry(name.to_string()).or_default().clone();
        let _guard = match lock.clone().try_lock_owned() {
            Ok(guard) => guard,
            Err(_) => {
                waiting();
                lock.lock_owned().await
            }
        };

        if replace {
            self.remove_image(name).await?;
        } else if let Some(layers) = self.image_layers(name).await? {
            return Ok(layers);
        }

        waiting();

        let mut progress = self.client.import_image_stream(
            bollard::query_parameters::ImportImageOptions {
                quiet: true,
                ..Default::default()
            },
            archive(),
            None,
        );

        while let Some(update) = progress.next().await {
            let update = update
                .map_err(|e| ContainerError::Unknown(format!("could not load image; {e}")))?;
            if let Some(error) = update.error_detail.and_then(|detail| detail.message) {
                return Err(ContainerError::Unknown(format!(
                    "could not load image; {error}"
                )));
            }
        }

        self.image_layers(name).await?.ok_or_else(|| {
            ContainerError::NoSuchImage(format!("'{name}' wasn't there after loading it"))
        })
    }

    /// Makes sure Docker has an image from Gofer's image store, loading it if it doesn't, and returns the name Docker
    /// knows it by. `loading` is called if it has to be loaded, or if another load of it has to be waited for.
    ///
    /// Docker's copy has to have exactly the layers the image was stored with. Docker finds a layer by its diff id, and
    /// its JSON parser reads `rootfs` and `RootFS` as the same field where ours doesn't, so only Docker's own answer
    /// settles what it would run. A copy that doesn't match, like something tagged with this name by hand, gets replaced
    /// once.
    pub async fn load_stored_image(
        &self,
        images: &crate::images::Images,
        image: &crate::images::StoredImage,
        loading: impl Fn(),
    ) -> Result<String, ContainerError> {
        let name = image.manifest.docker_name();

        for replace in [false, true] {
            let layers = self
                .load_image(&name, replace, &loading, || images.docker_archive(image))
                .await?;

            if layers == image.diff_ids {
                return Ok(name);
            }
        }

        Err(ContainerError::ImageMismatch)
    }

    /// The diff ids of an image's layers as Docker has them, or `None` if Docker doesn't have the image.
    async fn image_layers(&self, name: &str) -> Result<Option<Vec<String>>, ContainerError> {
        match self.client.inspect_image(name).await {
            Ok(image) => Ok(Some(
                image
                    .root_fs
                    .and_then(|root_fs| root_fs.layers)
                    .unwrap_or_default(),
            )),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(None),
            Err(e) => Err(ContainerError::Unknown(format!(
                "could not inspect image '{name}'; {e}"
            ))),
        }
    }

    /// The PATH an image's containers start with. Setting PATH for a container replaces the image's, so adding a
    /// folder to it means starting from this.
    async fn image_path(&self, name: &str) -> Result<String, ContainerError> {
        let image = self.client.inspect_image(name).await.map_err(|e| {
            ContainerError::Unknown(format!("could not inspect image '{name}'; {e}"))
        })?;

        Ok(image_path_from_env(
            &image
                .config
                .and_then(|config| config.env)
                .unwrap_or_default(),
        ))
    }

    /// The names of every task image Docker has, as [`crate::images::Digest::docker_name`] makes them.
    pub async fn list_task_images(&self) -> Result<Vec<String>, ContainerError> {
        let options = bollard::query_parameters::ListImagesOptions {
            filters: Some(HashMap::from([(
                "reference".to_string(),
                vec![crate::images::DOCKER_REPOSITORY.to_string()],
            )])),
            ..Default::default()
        };

        let images = (self.client.list_images(Some(options)).await)
            .map_err(|e| ContainerError::Unknown(format!("could not list images; {e}")))?;

        Ok(images
            .into_iter()
            .flat_map(|image| image.repo_tags)
            .filter(|name| crate::images::Digest::from_docker_name(name).is_some())
            .collect())
    }

    /// Removes an image even when stopped containers still use it, which Docker leaves running ones alone for.
    pub async fn remove_image(&self, name: &str) -> Result<(), ContainerError> {
        let options = bollard::query_parameters::RemoveImageOptions {
            force: true,
            ..Default::default()
        };

        match self.client.remove_image(name, Some(options), None).await {
            Ok(_)
            | Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(()),
            Err(e) => Err(ContainerError::Unknown(format!(
                "could not remove image '{name}'; {e}"
            ))),
        }
    }

    /// Creates a run's folder. Every task does this before its container is made, and Docker leaves a volume that's
    /// already there alone, so whichever task starts first is the one that makes it.
    async fn create_run_folder(&self, folder: &RunFolder) -> Result<(), ContainerError> {
        self.client
            .create_volume(bollard::models::VolumeCreateRequest {
                name: Some(folder.volume()),
                labels: Some(folder.labels()),
                ..Default::default()
            })
            .await
            .map_err(|e| {
                ContainerError::Unknown(format!("could not create the run's folder; {e}"))
            })?;

        Ok(())
    }

    /// Makes the run's folder writable by every user, in a container that's been created but not started.
    ///
    /// A new volume is owned by root and only root can write to it, so a task whose image runs as another user, like a
    /// distroless `nonroot` image, couldn't leave anything in it. Docker sets a folder's permissions to what a tar
    /// says when the tar is copied into a container, but not for the folder the tar goes into. That's why the volume
    /// is mounted one level up and the folder tasks use is one inside it, which this tar makes.
    ///
    /// Every task does this, so the folder is open again even if an earlier task changed its permissions.
    async fn open_run_folder(&self, container_id: &str) -> Result<(), ContainerError> {
        let tar = open_folder_tar().map_err(|e| {
            ContainerError::Unknown(format!("could not open the run's folder; {e}"))
        })?;

        let upload = || {
            self.client.upload_to_container(
                container_id,
                Some(bollard::query_parameters::UploadToContainerOptions {
                    path: RUN_VOLUME_PATH.into(),
                    ..Default::default()
                }),
                bollard::body_full(tar.clone().into()),
            )
        };

        // Docker only makes a folder from the tar if it doesn't see one there already. Two tasks that start at the same
        // moment can both see that `run` isn't there yet, and then the slower one fails with "mkdirat run: file
        // exists". The folder is there by then, so copying the tar again only sets its permissions.
        if let Err(e) = upload().await {
            debug!(container_id, error = %e, "could not open the run's folder; trying again");
            upload().await.map_err(|e| {
                ContainerError::Unknown(format!("could not open the run's folder; {e}"))
            })?;
        }

        Ok(())
    }

    /// Every run folder Docker has.
    pub async fn list_run_folders(&self) -> Result<Vec<RunFolder>, ContainerError> {
        let options = bollard::query_parameters::ListVolumesOptions {
            filters: Some(HashMap::from([(
                "label".to_string(),
                vec![RUN_LABEL.to_string()],
            )])),
        };

        let response = (self.client.list_volumes(Some(options)).await)
            .map_err(|e| ContainerError::Unknown(format!("could not list volumes; {e}")))?;

        Ok((response.volumes.unwrap_or_default().iter())
            .filter_map(|volume| RunFolder::from_labels(&volume.labels))
            .collect())
    }

    /// Removes a run's folder, along with the containers of the run's tasks. Docker won't remove a volume that any
    /// container has mounted, even one that's stopped. A folder that's already gone is fine.
    pub async fn remove_run_folder(&self, folder: &RunFolder) -> Result<(), ContainerError> {
        let volume = folder.volume();

        let options = bollard::query_parameters::ListContainersOptions {
            all: true,
            filters: Some(HashMap::from([(
                "volume".to_string(),
                vec![volume.clone()],
            )])),
            ..Default::default()
        };
        let containers = (self.client.list_containers(Some(options)).await)
            .map_err(|e| ContainerError::Unknown(format!("could not list containers; {e}")))?;

        for container in containers {
            let Some(id) = container.id else {
                continue;
            };

            let options = bollard::query_parameters::RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            match self.client.remove_container(&id, Some(options)).await {
                Ok(())
                | Err(bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                }) => {}
                Err(e) => {
                    return Err(ContainerError::Unknown(format!(
                        "could not remove container '{id}'; {e}"
                    )));
                }
            }
        }

        match (self.client)
            .remove_volume(
                &volume,
                None::<bollard::query_parameters::RemoveVolumeOptions>,
            )
            .await
        {
            Ok(())
            | Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(()),
            Err(e) => Err(ContainerError::Unknown(format!(
                "could not remove volume '{volume}'; {e}"
            ))),
        }
    }

    /// Start a container based on details passed. Pulls the container's image first if it's from a registry and needs
    /// to be.
    pub async fn start_container(
        &self,
        request: StartContainerRequest,
    ) -> Result<StartContainerResponse, ContainerError> {
        let report = |phase| {
            if let Some(timeline) = &request.timeline {
                _ = timeline.send(TimelineEntry::now(phase));
            }
        };

        let (image, platform) = match &request.image {
            Image::Registry {
                name,
                auth,
                always_pull,
            } => {
                if *always_pull || self.image_layers(name).await?.is_none() {
                    self.pull_image(name, auth.as_ref()).await?;
                }
                (name.clone(), String::new())
            }
            Image::Loaded(name) => (name.clone(), TASK_PLATFORM.to_string()),
        };

        report(Phase::StartingContainer);

        // We attempt to remove the container as a first step to running it. This enables the functionality that
        // Gofer can start the same extension container without the error that a container with that name
        // already exists.
        if let Err(e) = self
            .client
            .remove_container(
                &request.id,
                Some(bollard::query_parameters::RemoveContainerOptions {
                    v: true,
                    force: true,
                    ..Default::default() //link: true,
                }),
            )
            .await
        {
            trace!(container_name = &request.id, err = ?e, "could not remove previous container; \
                This is not an actual error unless there was a failure because of container names");
        }

        let mut host_config = bollard::models::HostConfig::default();
        let mut mounts = Vec::new();
        let mut variables = request.variables;

        if let Some(folder) = &request.run_folder {
            self.create_run_folder(folder).await?;

            mounts.push(bollard::models::Mount {
                target: Some(RUN_VOLUME_PATH.into()),
                source: Some(folder.volume()),
                typ: Some(bollard::models::MountType::VOLUME),
                // Otherwise Docker fills an empty volume with whatever the first task's image has at that path.
                volume_options: Some(bollard::models::MountVolumeOptions {
                    no_copy: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        if request.gofer_cli
            && let Some(cli) = &self.cli
        {
            // The binary can go missing while Gofer runs, like when an upgrade deletes it before Gofer restarts. Docker
            // would refuse to start the task then, and nothing about a task needs the CLI.
            if cli.exists() {
                mounts.push(bollard::models::Mount {
                    target: Some(TASK_CLI_PATH.into()),
                    source: Some(cli.to_string_lossy().into_owned()),
                    typ: Some(bollard::models::MountType::BIND),
                    read_only: Some(true),
                    ..Default::default()
                });

                let path = match variables.get("PATH") {
                    Some(path) => path.clone(),
                    None => self.image_path(&image).await?,
                };
                variables.insert("PATH".into(), format!("{path}:{TASK_CLI_FOLDER}"));
            } else {
                warn!(
                    path = %cli.display(),
                    "Gofer's binary isn't where it started from anymore, so this task won't get the gofer CLI"
                );
            }
        }

        if !mounts.is_empty() {
            host_config.mounts = Some(mounts);
        }

        let mut container_config = bollard::models::ContainerCreateBody {
            image: Some(image),
            env: Some(
                variables
                    .into_iter()
                    .map(|(key, value)| format_env_var(&key, &value))
                    .collect(),
            ),
            ..Default::default()
        };

        if let Some(entrypoint) = request.entrypoint {
            container_config.entrypoint = Some(entrypoint);
        }

        if let Some(command) = request.command {
            container_config.cmd = Some(command);
        }

        // In order to properly set up a container such that we can talk to it we need several things:
        //   1. We need to expose the port that the container is listening on. We are passed this port by the caller.
        //   2. We then need to bind one of our local/host ports to the port of local machine. This enables
        //      us to talk to direct traffic to the port. We set this to 127.0.0.1 to keep it purely local
        //      and then we omit the port so that the docker engine assigns us a random open port.
        //   3. Finally we create a binding in docker between the addresses in step 1 and 2.
        if let Some(port) = request.networking {
            container_config.exposed_ports = Some(vec![format!("{port}/tcp")]);

            let host_port_binding = bollard::models::PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                // A value of None for host_port conveys that the engine should automatically allocate a port from
                // freely available ephemeral port range (32768-61000)
                host_port: None,
            };
            let mut port_bindings = HashMap::new();
            port_bindings.insert(format!("{port}/tcp"), Some(vec![host_port_binding]));

            host_config.port_bindings = Some(port_bindings);
        }

        container_config.host_config = Some(host_config);

        self.client
            .create_container(
                Some(bollard::query_parameters::CreateContainerOptions {
                    name: Some(request.id.clone()),
                    platform,
                }),
                container_config,
            )
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        if request.run_folder.is_some() {
            self.open_run_folder(&request.id).await?;
        }

        self.client
            .start_container(&request.id, None)
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        let container_info = self
            .client
            .inspect_container(&request.id, None)
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        let mut response = StartContainerResponse { url: None };

        if let Some(port) = request.networking {
            let network_settings = container_info.network_settings.ok_or_else(|| {
                ContainerError::Unknown("could not get networking settings".to_string())
            })?;

            let ports = network_settings.ports.ok_or_else(|| {
                ContainerError::Unknown(
                    "could not get networking settings (ports struct)".to_string(),
                )
            })?;

            let ports = ports.get(&format!("{port}/tcp"));
            let ports = match ports {
                Some(ports) => ports,
                None => {
                    return Err(ContainerError::Unknown(format!(
                        "could not get network settings (ports binding); {:#?}",
                        ports
                    )));
                }
            };
            let ports = match ports {
                Some(ports) => ports,
                None => {
                    return Err(ContainerError::Unknown(format!(
                        "could not get network settings (ports binding); {:#?}",
                        ports
                    )));
                }
            };

            let port = ports.first().ok_or_else(|| {
                ContainerError::Unknown("could not get networking settings (port)".to_string())
            })?;

            response.url = Some(format!(
                "{}:{}",
                port.host_ip.as_ref().unwrap(),
                port.host_port.as_ref().unwrap()
            ));
        }

        Ok(response)
    }

    async fn pull_image(
        &self,
        name: &str,
        auth: Option<&RegistryAuth>,
    ) -> Result<(), ContainerError> {
        let credentials = auth.map(|auth| bollard::auth::DockerCredentials {
            username: Some(auth.user.clone()),
            password: Some(auth.pass.clone()),
            ..Default::default()
        });

        self.client
            .create_image(
                Some(bollard::query_parameters::CreateImageOptions {
                    from_image: Some(name.to_string()),
                    ..Default::default()
                }),
                None,
                credentials,
            )
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| ContainerError::NoSuchImage(format!("{e:?}")))?;

        Ok(())
    }

    /// Kill a container with an associated timeout if the container does not respond to graceful shutdown.
    pub async fn stop_container(&self, req: StopContainerRequest) -> Result<(), ContainerError> {
        self.client
            .stop_container(
                &req.id,
                Some(bollard::query_parameters::StopContainerOptions {
                    t: Some(i32::try_from(req.timeout).unwrap_or(i32::MAX)),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        Ok(())
    }

    pub async fn cancel_task_execution(
        &self,
        execution: TaskExecution,
        timeout: i64,
    ) -> anyhow::Result<()> {
        let container_id = task_execution_container_id(
            &execution.namespace_id,
            &execution.pipeline_id,
            execution.run_id,
            &execution.task_id,
        );

        self.stop_container(StopContainerRequest {
            id: container_id.clone(),
            timeout,
        })
        .await
        .with_context(|| format!("Could not stop container while attempting to cancel task execution; container_id = {}",
                container_id))?;

        Ok(())
    }

    /// Returns a stream of logs from the container.
    pub fn get_logs(
        &self,
        req: GetLogsRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<Log, ContainerError>> + Send>> {
        let logs_options = bollard::query_parameters::LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let logs = self.client.logs(&req.id, Some(logs_options));

        let logs = logs
            .map_ok(|log| match log {
                bollard::container::LogOutput::StdOut { message } => Log::Stdout(message.to_vec()),
                bollard::container::LogOutput::StdErr { message } => Log::Stderr(message.to_vec()),
                _ => Log::Unknown,
            })
            .map_err(|e| ContainerError::NoSuchContainer(e.to_string()));

        Box::pin(logs)
    }

    /// Get the current state of container and potential exit code.
    pub async fn get_state(
        &self,
        req: GetStateRequest,
    ) -> Result<GetStateResponse, ContainerError> {
        let container_info = self
            .client
            .inspect_container(&req.id, None)
            .await
            .map_err(|e| ContainerError::NoSuchContainer(e.to_string()))?;

        match container_info.state.as_ref().unwrap().status.unwrap() {
            bollard::models::ContainerStateStatusEnum::CREATED
            | bollard::models::ContainerStateStatusEnum::RUNNING => Ok(GetStateResponse {
                exit_code: None,
                state: ContainerState::Running,
            }),
            bollard::models::ContainerStateStatusEnum::EXITED => Ok(GetStateResponse {
                exit_code: Some(container_info.state.unwrap().exit_code.unwrap() as u8),
                state: ContainerState::Exited,
            }),
            _ => Ok(GetStateResponse {
                exit_code: None,
                state: ContainerState::Unknown,
            }),
        }
    }

    /// Attach to a running container for debugging or other purposes.
    pub async fn attach_container(
        &self,
        req: AttachContainerRequest,
    ) -> Result<AttachContainerResponse, ContainerError> {
        let create_exec_options = CreateExecOptions::<String> {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(true),
            env: None,
            cmd: Some(req.command),
            privileged: None,
            user: None,
            working_dir: None,
            ..Default::default()
        };

        let create_results = self
            .client
            .create_exec(&req.id, create_exec_options)
            .await
            .map_err(|err| {
                ContainerError::Unknown(format!(
                    "Could not execute command to container: {:#?}",
                    err
                ))
            })?;

        let start_exec_options = StartExecOptions {
            detach: false,
            tty: true,
            ..Default::default()
        };

        let results = self
            .client
            .start_exec(&create_results.id, Some(start_exec_options))
            .await
            .map_err(|err| {
                ContainerError::Unknown(format!(
                    "Could not attach to exec for container: {:#?}",
                    err
                ))
            })?;

        match results {
            bollard::exec::StartExecResults::Attached { output, input } => {
                let output = output
                    .map_ok(|log| match log {
                        bollard::container::LogOutput::StdOut { message } => {
                            Log::Stdout(message.to_vec())
                        }
                        bollard::container::LogOutput::StdErr { message } => {
                            Log::Stderr(message.to_vec())
                        }
                        bollard::container::LogOutput::StdIn { message } => {
                            Log::Stdin(message.to_vec())
                        }
                        bollard::container::LogOutput::Console { message } => {
                            Log::Console(message.to_vec())
                        }
                    })
                    .map_err(|e| ContainerError::Unknown(e.to_string()));

                Ok(AttachContainerResponse {
                    input,
                    output: Box::pin(output),
                })
            }
            bollard::exec::StartExecResults::Detached => Err(ContainerError::Unknown(
                "Could not properly attach to container".into(),
            )),
        }
    }
}

/// A tar of just the folder tasks use, which anyone can write to. See [`Containers::open_run_folder`].
fn open_folder_tar() -> std::io::Result<Vec<u8>> {
    let mut header = tar::Header::new_ustar();
    header.set_path(format!("{RUN_FOLDER_NAME}/"))?;
    header.set_entry_type(tar::EntryType::Directory);
    header.set_mode(0o777);
    header.set_uid(0);
    header.set_gid(0);
    header.set_size(0);
    header.set_mtime(crate::api::epoch_milli() / 1000);
    header.set_cksum();

    let mut builder = tar::Builder::new(vec![]);
    builder.append(&header, std::io::empty())?;
    builder.into_inner()
}

// We periodically need to clean up docker assets so we don't run out of disk space.
// We perform it very infrequently though, in order to give operators time to diagnose
// any potential issues they might be having with a particular container.
#[instrument(fields(origin = "containers"))]
async fn prune_containers(client: bollard::Docker, interval: u64) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        debug!(interval = interval, "Started docker pruning");

        let result = match client.prune_containers(None).await {
            Ok(result) => result,
            Err(e) => {
                error!(err = ?e, "could not successfully prune containers");
                continue;
            }
        };

        info!(
            containers_deleted = ?result.containers_deleted,
            space_reclaimed = result.space_reclaimed,
            "Pruned containers"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Users' tasks are written against this path too, so it can't move.
    #[test]
    fn test_cli_is_at_gofer_bin() {
        assert_eq!(TASK_CLI_PATH, format!("{TASK_CLI_FOLDER}/gofer"));
        assert_eq!(TASK_CLI_PATH, "/gofer/bin/gofer");
    }

    #[test]
    fn test_image_path_from_env() {
        let env = [
            "HOME=/root".to_string(),
            "PATH=/opt/tool/bin:/usr/bin".to_string(),
        ];
        assert_eq!(image_path_from_env(&env), "/opt/tool/bin:/usr/bin");

        // Scratch and distroless images often don't set one, and Docker gives them its own.
        assert_eq!(
            image_path_from_env(&["HOME=/root".to_string()]),
            DEFAULT_PATH
        );
    }

    // Users' tasks are written against this path, so it can't move.
    #[test]
    fn test_run_folder_is_at_gofer_run() {
        assert_eq!(format!("{RUN_VOLUME_PATH}/{RUN_FOLDER_NAME}"), "/gofer/run");

        let tar = open_folder_tar().unwrap();
        let mut archive = tar::Archive::new(tar.as_slice());
        let entries: Vec<(String, u32)> = (archive.entries().unwrap())
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path().unwrap().display().to_string();
                (path, entry.header().mode().unwrap())
            })
            .collect();
        assert_eq!(entries, vec![("run/".to_string(), 0o777)]);
    }

    #[test]
    fn test_run_folders_are_found_by_their_labels() {
        let folder = RunFolder {
            namespace_id: "some-namespace".into(),
            pipeline_id: "some-pipeline".into(),
            run_id: 12,
        };

        assert_eq!(folder.volume(), "gofer_some-namespace_some-pipeline_12");
        assert_eq!(RunFolder::from_labels(&folder.labels()), Some(folder));

        // A volume someone else labeled with only part of it isn't one of ours.
        let labels = HashMap::from([(RUN_LABEL.to_string(), "12".to_string())]);
        assert_eq!(RunFolder::from_labels(&labels), None);
    }
}
