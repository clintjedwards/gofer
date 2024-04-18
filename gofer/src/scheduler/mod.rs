pub mod docker;

use async_trait::async_trait;
use futures::Stream;
use serde::Deserialize;
use std::fmt::Debug;
use std::{collections::HashMap, pin::Pin};
use strum::{Display, EnumString};
use tokio::io::AsyncWrite;

/// Represents different scheduler failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    /// Failed to start scheduler due to misconfigured settings, usually from a misconfigured settings file.
    #[error("could not init scheduler; {0}")]
    FailedSchedulerPrecondition(String),

    /// Failed to communicate with scheduler due to network error or other.
    #[error("could not connect to scheduler; {0}")]
    #[allow(dead_code)]
    Connection(String),

    /// Container requested by name could not be found.
    #[error("container not found; {0}")]
    NoSuchContainer(String),

    /// Image requested by name could not be found.
    #[error("image not found; {0}")]
    NoSuchImage(String),

    /// An expected and unknown error has occurred.
    #[error("unexpected scheduler error occurred; {0}")]
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
    Cancelled,
}

/// Private repositories sometimes require authentication.
#[derive(Debug)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

impl From<crate::api::RegistryAuth> for RegistryAuth {
    fn from(ra: crate::api::RegistryAuth) -> Self {
        Self {
            user: ra.user,
            pass: ra.pass,
        }
    }
}

#[derive(Debug)]
pub struct StartContainerRequest {
    /// A unique identifier to identify the container with.
    pub id: String,

    /// The docker image repository and docker image name; tag can be included.
    pub image: String,

    /// Environment variables to be passed to the container.
    pub variables: HashMap<String, String>,

    /// Registry authentication details.
    pub registry_auth: Option<RegistryAuth>,

    /// Attempt to pull the container from the upstream repository even if it exists already locally.
    /// This is useful if your containers don't use proper tagging or versioning.
    pub always_pull: bool,

    /// Only needed by extensions; spin the container up with appropriate networking settings such that Gofer can
    /// connect to it.
    pub networking: Option<u16>,

    /// Replaces container's entrypoint with a custom one.
    pub entrypoint: Option<Vec<String>>,

    /// Replaces container's cmd instruction with a custom one.
    pub command: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct StartContainerResponse {
    /// An optional, unique way for the scheduler to identify the container. Sometimes the scheduler
    /// will not be able to use the client provided container name as a unique identifier and will
    /// return it's own identifier. In these cases the client will have to store the scheduler's id
    /// for further use.
    pub scheduler_id: Option<String>,

    /// An endpoint that only is returned for containers with networking set to on.
    pub url: Option<String>,
}

#[derive(Debug)]
pub struct StopContainerRequest {
    /// A unique identifier to identify the container with.
    pub id: String,

    /// The total time the scheduler should wait for a graceful stop before issuing a SIGKILL in seconds.
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
    pub output: Pin<Box<dyn Stream<Item = Result<Log, SchedulerError>> + Send>>,
    pub input: Pin<Box<dyn AsyncWrite + Send>>,
}

/// Represents a single log line/entry from a particular container.
#[derive(Debug)]
pub enum Log {
    Unknown,
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Stdin(Vec<u8>),
    Console(Vec<u8>),
}

/// The scheduler trait defines what the interface between Gofer and a container scheduler.
#[async_trait]
pub trait Scheduler: Debug + Send + Sync + 'static {
    /// Start a container based on details passed; Should implement automatically pulling and registry auth
    /// of container if necessary.
    async fn start_container(
        &self,
        req: StartContainerRequest,
    ) -> Result<StartContainerResponse, SchedulerError>;

    /// Kill a container with an associated timeout if the container does not respond to graceful shutdown.
    async fn stop_container(&self, req: StopContainerRequest) -> Result<(), SchedulerError>;

    /// Get the current state of container and potential exit code.
    async fn get_state(&self, req: GetStateRequest) -> Result<GetStateResponse, SchedulerError>;

    /// Returns a stream of logs from the container.
    fn get_logs(
        &self,
        req: GetLogsRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<Log, SchedulerError>> + Send>>;

    /// Attach to a running container for debugging or other purposes.
    async fn attach_container(
        &self,
        req: AttachContainerRequest,
    ) -> Result<AttachContainerResponse, SchedulerError>;
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Display, EnumString)]
#[serde(rename_all = "snake_case")] // This handles case insensitivity during deserialization
pub enum Engine {
    #[default]
    Docker,
}

pub async fn new(
    config: &crate::conf::api::Scheduler,
) -> Result<Box<dyn Scheduler>, SchedulerError> {
    #[allow(clippy::match_single_binding)]
    match config.engine {
        Engine::Docker => {
            if config.docker.is_none() {
                return Err(SchedulerError::FailedSchedulerPrecondition(
                    "Docker engine settings not found in config".into(),
                ));
            }

            let engine = docker::Scheduler::new(&config.clone().docker.unwrap()).await?;
            Ok(Box::new(engine))
        }
    }
}
