mod docker;

use crate::conf;
use crate::models::TaskRunState;
use async_trait::async_trait;
use futures::Stream;
use std::{collections::HashMap, pin::Pin};

/// Represents different scheduler failure possibilities.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    /// Failed to start scheduled due to misconfigured settings, usually from a misconfigured settings file.
    #[error("could not init scheduler; {0}")]
    #[allow(dead_code)]
    FailedPrecondition(String),

    /// Failed to communicate with scheduler due to network error or other.
    #[error("could not connect to scheduler; {0}")]
    #[allow(dead_code)]
    Connection(String),

    /// Container requested by name could not be found.
    #[error("container not found; {0}")]
    NoSuchContainer(String),

    /// Image requested by name could not be found.
    #[error("docker image not found; {0}")]
    NoSuchImage(String),

    /// An expected and unknown error has occurred.
    #[error("unexpected scheduler error occurred; {0}")]
    Unknown(String),
}

/// It is sometimes desirable for someone to run a different entrypoint with their container.
/// This represents the shell and script of that entrypoint.
#[derive(Debug)]
pub struct Exec {
    pub shell: String,
    pub script: String,
}

/// Private repositories sometimes require authentication.
#[derive(Debug)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

#[derive(Debug)]
pub struct StartContainerRequest {
    /// A unique identifier to identify the container with.
    pub name: String,
    /// The docker image repository and docker image name; tag can be included.
    pub image_name: String,
    /// Environment variables to be passed to the container.
    pub variables: HashMap<String, String>,
    /// Registry authentication details.
    pub registry_auth: Option<RegistryAuth>,
    /// Attempt to pull the container from the upstream repository even if it exists already locally.
    /// This is useful if your containers don't use proper tagging or versioning.
    pub always_pull: bool,
    /// Only needed by triggers; used to spin the container up with networking on so that Gofer can tal
    /// to it.
    pub enable_networking: bool,
    /// Replaces the container's entrypoint with a custom passed in script.
    pub exec: Option<Exec>,
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
    pub name: String,
    /// The total time the scheduler should wait for a graceful stop before issuing a SIGKILL.
    pub timeout: i64,
}

#[derive(Debug)]
pub struct GetStateRequest {
    /// Unique identifier for container to stop.
    pub name: String,
}

#[derive(Debug)]
pub struct GetStateResponse {
    /// In the event that the container is in a "complete" state; the exit code of that container.
    pub exit_code: Option<u8>,
    /// The current state of the container, state referencing how complete the container process of running is.
    pub state: TaskRunState,
}

#[derive(Debug)]
pub struct GetLogsRequest {
    /// Unique identifier for container to stop.
    pub name: String,
}

/// Represents a single log line/entry from a particular container.
#[derive(Debug)]
pub enum Log {
    Unknown,
    Stdout(bytes::Bytes),
    Stderr(bytes::Bytes),
}

/// The scheduler trait defines what the interface between Gofer and a container scheduler should look like.
#[async_trait]
pub trait Scheduler {
    /// Start a container based on details passed; Should implement automatically pulling and registry auth
    /// of container if necessary.
    async fn start_container(
        &self,
        req: StartContainerRequest,
    ) -> Result<StartContainerResponse, SchedulerError>;

    /// Kill a container with an associated timeout if the container does not response to graceful shutdown.
    async fn stop_container(&self, req: StopContainerRequest) -> Result<(), SchedulerError>;

    /// Get the current state of container and potential exit code.
    async fn get_state(&self, req: GetStateRequest) -> Result<GetStateResponse, SchedulerError>;

    /// Returns a stream of logs from the container.
    fn get_logs(
        &self,
        req: GetLogsRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<Log, SchedulerError>>>>;
}

pub enum SchedulerEngine {
    Docker,
}

pub async fn init_scheduler(
    engine: SchedulerEngine,
    config: conf::api::Scheduler,
) -> Result<Box<dyn Scheduler>, SchedulerError> {
    #[allow(clippy::match_single_binding)]
    match engine {
        SchedulerEngine::Docker => {
            if let Some(config) = config.docker {
                let engine = docker::Engine::new(config.prune, config.prune_interval).await?;
                Ok(Box::new(engine))
            } else {
                Err(SchedulerError::FailedPrecondition(
                    "docker engine settings not found in config".into(),
                ))
            }
        }
    }
}
