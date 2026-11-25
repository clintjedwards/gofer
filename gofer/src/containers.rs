//! Starts, stops, and watches the Docker containers that tasks and extensions run in.

use crate::api::task_executions::{
    Phase, TaskExecution, TimelineEntry, task_execution_container_id,
};
use anyhow::Context;
use bollard::exec::{CreateExecOptions, StartExecOptions};
use futures::Stream;
use futures::stream::TryStreamExt;
use serde::Deserialize;
use std::{collections::HashMap, pin::Pin};
use strum::{Display, EnumString};
use tokio::io::AsyncWrite;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info, instrument, trace};

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
            .field("pass", &crate::api::REDACTED)
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

    /// Where to report the phases of starting the container (pulling the image, starting the container) as they
    /// begin. Lets task executions show where their time went before the container was running. Skips any phase
    /// that doesn't happen, like pulling an image that's already present.
    pub timeline: Option<UnboundedSender<TimelineEntry>>,
}

#[derive(Debug)]
pub struct StartContainerResponse {
    /// An endpoint that only is returned for containers with networking set to on.
    pub url: Option<String>,

    /// The exact image the container was started with. Usually the repo digest, or the image ID for local images.
    pub image_digest: Option<String>,
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

        debug!(
            version = version.version.unwrap_or_default(),
            "Successfully connected to Docker"
        );

        Ok(Self { client })
    }

    /// Start a container based on details passed. Pulls the container's image first if it needs to.
    pub async fn start_container(
        &self,
        request: StartContainerRequest,
    ) -> Result<StartContainerResponse, ContainerError> {
        let credentials =
            request
                .registry_auth
                .as_ref()
                .map(|ra| bollard::auth::DockerCredentials {
                    username: Some(ra.user.clone()),
                    password: Some(ra.pass.clone()),
                    ..Default::default()
                });

        let report = |phase| {
            if let Some(timeline) = &request.timeline {
                _ = timeline.send(TimelineEntry::now(phase));
            }
        };

        if request.always_pull {
            report(Phase::PullingImage);
            self.client
                .create_image(
                    Some(bollard::query_parameters::CreateImageOptions {
                        from_image: Some(request.image.clone()),
                        ..Default::default()
                    }),
                    None,
                    credentials,
                )
                .try_collect::<Vec<_>>()
                .await
                .map_err(|e| ContainerError::NoSuchImage(format!("{:?}", e)))?;
        } else {
            let mut filters = HashMap::new();
            filters.insert("reference".to_string(), vec![request.image.clone()]);

            let images = self
                .client
                .list_images(Some(bollard::query_parameters::ListImagesOptions {
                    all: true,
                    filters: Some(filters),
                    ..Default::default()
                }))
                .await
                .map_err(|e| {
                    ContainerError::Unknown(format!("could not list local images; {:?}", e))
                })?;

            if images.is_empty() {
                report(Phase::PullingImage);
                self.client
                    .create_image(
                        Some(bollard::query_parameters::CreateImageOptions {
                            from_image: Some(request.image.clone()),
                            ..Default::default()
                        }),
                        None,
                        credentials,
                    )
                    .try_collect::<Vec<_>>()
                    .await
                    .map_err(|e| ContainerError::NoSuchImage(format!("{:?}", e)))?;
            }
        }

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

        let mut container_config = bollard::models::ContainerCreateBody {
            image: Some(request.image.clone()),
            env: Some(
                request
                    .variables
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

            container_config.host_config = Some(bollard::models::HostConfig {
                port_bindings: Some(port_bindings),
                ..Default::default()
            })
        }

        self.client
            .create_container(
                Some(bollard::query_parameters::CreateContainerOptions {
                    name: Some(request.id.clone()),
                    ..Default::default()
                }),
                container_config,
            )
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        self.client
            .start_container(&request.id, None)
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        let container_info = self
            .client
            .inspect_container(&request.id, None)
            .await
            .map_err(|e| ContainerError::Unknown(e.to_string()))?;

        // A tag like 'latest' can point at a different image from one run to the next, so we record the exact image
        // the container started with. Locally built images have no repo digest, so we fall back to the image ID.
        let image_digest = match &container_info.image {
            Some(image_id) => match self.client.inspect_image(image_id).await {
                Ok(image) => image
                    .repo_digests
                    .and_then(|digests| digests.into_iter().next())
                    .or_else(|| Some(image_id.clone())),
                Err(e) => {
                    debug!(container_name = &request.id, error = %e, "could not inspect image for digest");
                    Some(image_id.clone())
                }
            },
            None => None,
        };

        let mut response = StartContainerResponse {
            url: None,
            image_digest,
        };

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
