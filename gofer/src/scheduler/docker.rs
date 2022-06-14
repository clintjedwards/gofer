use super::*;
use crate::models::TaskRunState;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use futures::Stream;
use slog_scope::{debug, error};
use std::pin::Pin;
use std::{collections::HashMap, sync::Arc};

fn format_env_var(key: &str, value: &str) -> String {
    return format!("{}={}", key, value);
}

pub struct Engine {
    client: Arc<bollard::Docker>,
}

impl Engine {
    pub async fn new(prune: bool, prune_interval: u64) -> Result<Self, SchedulerError> {
        let client = bollard::Docker::connect_with_socket_defaults().map_err(|e| {
            SchedulerError::Connection(format!(
                "{}; Make sure the Docker daemon is installed and running.",
                e
            ))
        })?;
        let client = Arc::new(client);
        let prune_client = Arc::clone(&client);

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
        if prune {
            tokio::spawn(async move {
                match prune_client.prune_containers::<String>(None).await {
                    Ok(response) => {
                        debug!("pruned containers";
                               "containers_deleted" => format!("{:?}", response.containers_deleted),
                               "space_reclaimed" => response.space_reclaimed);
                    }
                    Err(e) => {
                        error!("could not successfully prune containers"; "error" => e.to_string())
                    }
                };

                tokio::time::sleep(std::time::Duration::new(prune_interval, 0)).await;
            });

            debug!("started docker pruning"; "interval" => format!("{:?}",prune_interval));
        }

        debug!("local docker scheduler successfully connected"; "version" => format!("{:?}", version));

        Ok(Self { client })
    }
}

#[async_trait]
impl Scheduler for Engine {
    async fn start_container(
        &self,
        req: StartContainerRequest,
    ) -> Result<StartContainerResponse, SchedulerError> {
        let credentials = req
            .registry_auth
            .as_ref()
            .map(|ra| bollard::auth::DockerCredentials {
                username: Some(ra.user.clone()),
                password: Some(ra.pass.clone()),
                ..Default::default()
            });

        if req.always_pull {
            self.client
                .create_image(
                    Some(bollard::image::CreateImageOptions {
                        from_image: req.image_name.clone(),
                        ..Default::default()
                    }),
                    None,
                    credentials,
                )
                .try_collect::<Vec<_>>()
                .await
                .map_err(|e| SchedulerError::NoSuchImage(e.to_string()))?;
        } else {
            let mut filters = HashMap::new();
            filters.insert("reference".to_string(), vec![req.image_name.clone()]);

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
                            from_image: req.image_name.clone(),
                            ..Default::default()
                        }),
                        None,
                        credentials,
                    )
                    .try_collect::<Vec<_>>()
                    .await
                    .map_err(|e| SchedulerError::NoSuchImage(e.to_string()))?;
            }
        }

        if let Err(e) = self
            .client
            .remove_container(
                &req.name,
                Some(bollard::container::RemoveContainerOptions {
                    v: true,
                    force: true,
                    link: true,
                }),
            )
            .await
        {
            debug!("could not remove previous container"; "name" => &req.name, "error" => e.to_string());
        }

        let mut container_config = bollard::container::Config {
            image: Some(req.image_name.clone()),
            env: Some(
                req.variables
                    .into_iter()
                    .map(|(key, value)| format_env_var(&key, &value))
                    .collect(),
            ),
            ..Default::default()
        };

        if let Some(exec) = req.exec {
            let script = base64::decode(exec.script).unwrap();
            container_config.entrypoint = Some(vec![
                exec.shell,
                "-c".to_string(),
                String::from_utf8_lossy(&script).to_string(),
            ]);
        }

        if req.enable_networking {
            let mut exposed_ports = HashMap::new();
            exposed_ports.insert("tcp/8080".to_string(), HashMap::new());
            container_config.exposed_ports = Some(exposed_ports);

            let host_port_binding = bollard::models::PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                // a value of 0 conveys that the engine should automatically allocate a port from freely
                // available ephemeral port range (32768-61000)
                host_port: Some("0".to_string()),
            };
            let mut port_bindings = HashMap::new();
            port_bindings.insert("8080/tcp".to_string(), Some(vec![host_port_binding]));

            container_config.host_config = Some(bollard::models::HostConfig {
                port_bindings: Some(port_bindings),
                ..Default::default()
            })
        }

        let created_container = self
            .client
            .create_container(
                Some(bollard::container::CreateContainerOptions { name: &req.name }),
                container_config,
            )
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        self.client
            .start_container::<String>(&req.name, None)
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        let container_info = self
            .client
            .inspect_container(&req.name, None)
            .await
            .map_err(|e| SchedulerError::Unknown(e.to_string()))?;

        let mut response = StartContainerResponse {
            scheduler_id: Some(created_container.id),
            url: None,
        };

        if req.enable_networking {
            let network_settings = container_info.network_settings.ok_or_else(|| {
                SchedulerError::Unknown("could not get networking settings".to_string())
            })?;

            let ports = network_settings.ports.ok_or_else(|| {
                SchedulerError::Unknown("could not get networking settings".to_string())
            })?;

            let ports = ports["8080/tcp"].as_ref().ok_or_else(|| {
                SchedulerError::Unknown("could not get networking settings".to_string())
            })?;

            let port = ports.get(0).ok_or_else(|| {
                SchedulerError::Unknown("could not get networking settings".to_string())
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
        self.client
            .stop_container(
                &req.name,
                Some(bollard::container::StopContainerOptions { t: req.timeout }),
            )
            .await
            .map_err(|e| SchedulerError::NoSuchContainer(e.to_string()))?;

        Ok(())
    }

    fn get_logs(
        &self,
        req: GetLogsRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<Log, SchedulerError>>>> {
        let logs_options = bollard::container::LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let logs = self.client.logs(&req.name, Some(logs_options));

        let logs = logs
            .map_ok(|log| match log {
                bollard::container::LogOutput::StdOut { message } => Log::Stdout(message),
                bollard::container::LogOutput::StdErr { message } => Log::Stderr(message),
                _ => Log::Unknown,
            })
            .map_err(|e| SchedulerError::NoSuchContainer(e.to_string()));

        Box::pin(logs)
    }

    async fn get_state(&self, req: GetStateRequest) -> Result<GetStateResponse, SchedulerError> {
        let container_info = self
            .client
            .inspect_container(&req.name, None)
            .await
            .map_err(|e| SchedulerError::NoSuchContainer(e.to_string()))?;

        match container_info.state.as_ref().unwrap().status.unwrap() {
            bollard::models::ContainerStateStatusEnum::CREATED
            | bollard::models::ContainerStateStatusEnum::RUNNING => {
                return Ok(GetStateResponse {
                    exit_code: None,
                    state: TaskRunState::Running,
                });
            }
            bollard::models::ContainerStateStatusEnum::EXITED => {
                dbg!(&container_info.state);
                return Ok(GetStateResponse {
                    exit_code: Some(container_info.state.unwrap().exit_code.unwrap() as u8),
                    state: TaskRunState::Complete,
                });
            }
            _ => {
                return Ok(GetStateResponse {
                    exit_code: None,
                    state: TaskRunState::Unknown,
                })
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::scheduler::GetLogsRequest;
//     use futures::stream::StreamExt;
//     use std::time::Duration;

//     use super::*;

//     #[tokio::test]
//     async fn hello() {
//         let engine = Engine::new(true, 100).await.unwrap();
//         engine
//             .start_container(StartContainerRequest {
//                 name: "container_test".to_string(),
//                 image_name: "ghcr.io/clintjedwards/gofer-containers/debug/log:latest".to_string(),
//                 variables: HashMap::new(),
//                 registry_auth: None,
//                 always_pull: true,
//                 enable_networking: false,
//                 exec: None,
//             })
//             .await
//             .unwrap();

//         // engine
//         //     .stop_container(StopContainerRequest {
//         //         name: "container_test".to_string(),
//         //         timeout: 100,
//         //     })
//         //     .await
//         //     .unwrap();

//         // let status = engine
//         //     .get_state(GetStateRequest {
//         //         name: "container_test".to_string(),
//         //     })
//         //     .await
//         //     .unwrap();

//         let mut logs = engine.get_logs(GetLogsRequest {
//             name: "container_test".to_string(),
//         });

//         while let Some(foo) = logs.next().await {
//             dbg!(foo);
//         }
//     }
// }
