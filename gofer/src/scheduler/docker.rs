use super::{
    AttachContainerRequest, AttachContainerResponse, ContainerState, GetLogsRequest,
    GetStateRequest, GetStateResponse, Log, SchedulerError, StartContainerRequest,
    StartContainerResponse, StopContainerRequest,
};
use async_trait::async_trait;
use bollard::exec::{CreateExecOptions, StartExecOptions};
use dashmap::DashMap;
use futures::stream::TryStreamExt;
use futures::Stream;
use serde::Deserialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, instrument, trace};

fn format_env_var(key: &str, value: &str) -> String {
    format!("{key}={value}")
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub prune: bool,
    pub prune_interval: u64, // in seconds
    /// The total amount of time any request is allowed to be pending for in seconds.
    pub timeout: u64,
}

#[derive(Debug, Clone)]
pub struct Scheduler {
    client: bollard::Docker,
    /// Cancelled keeps track of cancelled containers. This is needed due to there being no way to differentiate a
    /// container that was stopped in docker from a container that exited naturally.
    /// When we cancel a container we insert it into this map so that downstream readers of GetState can relay the
    /// cancellation to its users. After a predetermined about of time we clear the cancelled container from this list.
    ///
    /// Map takes in <ContainerID, ExpiryTimeInEpochSeconds>
    cancelled: Arc<DashMap<String, u64>>,
}

impl Scheduler {
    #[instrument(fields(origin = "scheduler::docker"))]
    pub async fn new(config: &Config) -> Result<Self, SchedulerError> {
        let config = config.clone();
        let client = bollard::Docker::connect_with_socket_defaults().map_err(|e| {
            SchedulerError::Connection(format!(
                "{}; Make sure the Docker daemon is installed and running.",
                e
            ))
        })?;
        let client = client.with_timeout(tokio::time::Duration::from_secs(config.timeout));
        let prune_client = client.clone();
        let cancellations = Arc::new(DashMap::new());

        // Check that we can actually get a connection.
        let version = client.version().await.map_err(|e| {
            SchedulerError::Connection(format!(
                "{}; Make sure the Docker daemon is installed and running.",
                e
            ))
        })?;

        // We periodically need to clean up docker assets so we don't run out of disk space.
        // We perform it very infrequently though, in order to give operators time to diagnose
        // any potential issues they might be having with a particular container.
        if config.prune {
            tokio::spawn(prune_containers(prune_client, config.prune_interval));
        }

        tokio::spawn(prune_cancellations(cancellations.clone()));

        debug!(
            version = version.version.unwrap_or_default(),
            "Local docker scheduler successfully connected"
        );

        Ok(Self {
            client,
            cancelled: cancellations,
        })
    }
}

#[instrument(fields(origin = "scheduler::docker"))]
async fn prune_cancellations(cancellations: Arc<DashMap<String, u64>>) {
    loop {
        for cancellation in cancellations.iter() {
            let (container_id, expiry) = cancellation.pair();

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if *expiry < now {
                cancellations.remove(container_id);
                debug!(container_id, "Removed cancelled container reference")
            };
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(259200)).await; // Sleep for three days.
    }
}

// We periodically need to clean up docker assets so we don't run out of disk space.
// We perform it very infrequently though, in order to give operators time to diagnose
// any potential issues they might be having with a particular container.
#[instrument(fields(origin = "scheduler::docker"))]
async fn prune_containers(client: bollard::Docker, interval: u64) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        debug!(interval = interval, "Started docker pruning");

        let result = match client.prune_containers::<String>(None).await {
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

#[async_trait]
impl super::Scheduler for Scheduler {
    async fn start_container(
        &self,
        request: StartContainerRequest,
    ) -> Result<StartContainerResponse, SchedulerError> {
        let credentials =
            request
                .registry_auth
                .as_ref()
                .map(|ra| bollard::auth::DockerCredentials {
                    username: Some(ra.user.clone()),
                    password: Some(ra.pass.clone()),
                    ..Default::default()
                });

        if request.always_pull {
            self.client
                .create_image(
                    Some(bollard::image::CreateImageOptions {
                        from_image: request.image.clone(),
                        ..Default::default()
                    }),
                    None,
                    credentials,
                )
                .try_collect::<Vec<_>>()
                .await
                .map_err(|e| SchedulerError::NoSuchImage(format!("{:?}", e)))?;
        } else {
            let mut filters = HashMap::new();
            filters.insert("reference".to_string(), vec![request.image.clone()]);

            let images = self
                .client
                .list_images(Some(bollard::image::ListImagesOptions {
                    all: true,
                    filters,
                    ..Default::default()
                }))
                .await
                .unwrap();

            if images.is_empty() {
                self.client
                    .create_image(
                        Some(bollard::image::CreateImageOptions {
                            from_image: request.image.clone(),
                            ..Default::default()
                        }),
                        None,
                        credentials,
                    )
                    .try_collect::<Vec<_>>()
                    .await
                    .map_err(|e| SchedulerError::NoSuchImage(format!("{:?}", e)))?;
            }
        }

        // We attempt to remove the container as a first step to running it. This enables the functionality that
        // Gofer can start the same extension container without the error that a container with that name
        // already exists.
        if let Err(e) = self
            .client
            .remove_container(
                &request.id,
                Some(bollard::container::RemoveContainerOptions {
                    v: true,
                    force: true,
                    ..Default::default() //link: true,
                }),
            )
            .await
        {
            trace!(container_name = &request.id, err = ?e, "could not remove previous container");
        }

        let mut container_config = bollard::container::Config {
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

        if request.entrypoint.is_some() {
            container_config.entrypoint = Some(request.entrypoint.unwrap());
        }

        if request.command.is_some() {
            container_config.cmd = Some(request.command.unwrap());
        }

        // In order to properly set up a container such that we can talk to it we need several things:
        //   1. We need to expose the port that the container is listening on. We are passed this port by the caller.
        //   2. We then need to bind one of our local/host ports to the port of local machine. This enables
        //      us to talk to direct traffic to the port. We set this to 127.0.0.1 to keep it purely local
        //      and then we omit the port so that the docker engine assigns us a random open port.
        //   3. Finally we create a binding in docker between the addresses in step 1 and 2.
        if let Some(port) = request.networking {
            let mut exposed_ports = HashMap::new();
            exposed_ports.insert(format!("{port}/tcp"), HashMap::new());
            container_config.exposed_ports = Some(exposed_ports);

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

        let created_container = self
            .client
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: &request.id,
                    platform: None,
                }),
                container_config,
            )
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        self.client
            .start_container::<String>(&request.id, None)
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        let container_info = self
            .client
            .inspect_container(&request.id, None)
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        let mut response = StartContainerResponse {
            scheduler_id: Some(created_container.id),
            url: None,
        };

        if let Some(port) = request.networking {
            let network_settings = container_info.network_settings.ok_or_else(|| {
                SchedulerError::Unknown("could not get networking settings".to_string())
            })?;

            let ports = network_settings.ports.ok_or_else(|| {
                SchedulerError::Unknown(
                    "could not get networking settings (ports struct)".to_string(),
                )
            })?;

            let ports = ports[&format!("{port}/tcp")].as_ref().ok_or_else(|| {
                SchedulerError::Unknown(
                    "could not get networking settings (ports binding)".to_string(),
                )
            })?;

            let port = ports.first().ok_or_else(|| {
                SchedulerError::Unknown("could not get networking settings (port)".to_string())
            })?;

            response.url = Some(format!(
                "{}:{}",
                port.host_ip.as_ref().unwrap(),
                port.host_port.as_ref().unwrap()
            ));
        }

        Ok(response)
    }

    async fn stop_container(&self, req: StopContainerRequest) -> Result<(), SchedulerError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expiry = now + 86400;

        self.cancelled.insert(req.id.clone(), expiry);

        self.client
            .stop_container(
                &req.id,
                Some(bollard::container::StopContainerOptions { t: req.timeout }),
            )
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        Ok(())
    }

    fn get_logs(
        &self,
        req: GetLogsRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<Log, SchedulerError>> + Send>> {
        let logs_options = bollard::container::LogsOptions::<String> {
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
            .map_err(|e| SchedulerError::NoSuchContainer(e.to_string()));

        Box::pin(logs)
    }

    async fn get_state(&self, req: GetStateRequest) -> Result<GetStateResponse, SchedulerError> {
        let container_info = self
            .client
            .inspect_container(&req.id, None)
            .await
            .map_err(|e| SchedulerError::NoSuchContainer(e.to_string()))?;

        match container_info.state.as_ref().unwrap().status.unwrap() {
            bollard::models::ContainerStateStatusEnum::CREATED
            | bollard::models::ContainerStateStatusEnum::RUNNING => {
                return Ok(GetStateResponse {
                    exit_code: None,
                    state: ContainerState::Running,
                });
            }
            bollard::models::ContainerStateStatusEnum::EXITED => {
                if self.cancelled.contains_key(&req.id) {
                    return Ok(GetStateResponse {
                        exit_code: Some(container_info.state.unwrap().exit_code.unwrap() as u8),
                        state: ContainerState::Cancelled,
                    });
                }

                return Ok(GetStateResponse {
                    exit_code: Some(container_info.state.unwrap().exit_code.unwrap() as u8),
                    state: ContainerState::Exited,
                });
            }
            _ => {
                return Ok(GetStateResponse {
                    exit_code: None,
                    state: ContainerState::Unknown,
                })
            }
        }
    }

    async fn attach_container(
        &self,
        req: AttachContainerRequest,
    ) -> Result<AttachContainerResponse, SchedulerError> {
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
                SchedulerError::Unknown(format!(
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
                SchedulerError::Unknown(format!(
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
                    .map_err(|e| SchedulerError::Unknown(e.to_string()));

                Ok(AttachContainerResponse {
                    input,
                    output: Box::pin(output),
                })
            }
            bollard::exec::StartExecResults::Detached => Err(SchedulerError::Unknown(
                "Could not properly attach to container".into(),
            )),
        }
    }
}
