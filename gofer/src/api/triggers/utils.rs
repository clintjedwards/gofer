use crate::api::{epoch, fmt, get_tls_client_config, Api};
use crate::{scheduler, storage};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use gofer_models::trigger::Trigger;
use gofer_proto::{
    trigger_service_client::TriggerServiceClient, TriggerInfoRequest, TriggerInfoResponse,
    TriggerShutdownRequest,
};
use nanoid::nanoid;
use slog_scope::{debug, error, info};
use std::{collections::HashMap, sync::Arc};
use tonic::{metadata::MetadataValue, transport::Channel, Request};

pub struct TriggerInfo {
    pub scheduler_id: Option<String>,
    pub url: String,
    pub documentation: String,
    pub key: String,
}

impl Api {
    /// Attempts to start the trigger given via scheduler. The function itself attempts to do
    /// everything needed so that the resulting trigger is ready for use by Gofer.
    ///
    /// A list of responsibilities:
    /// 1) Convert passed trigger config to properly named envvars
    /// 2) Pass in config envvars and Gofer provided envars.
    /// 3) Starts the container and checks that the container can communicate.
    /// 4) Launches the thread that collects container logs and outputs it into stdout.
    pub async fn start_trigger(
        &self,
        settings: &gofer_models::trigger::Registration,
    ) -> Result<TriggerInfo> {
        let config = self.conf.clone();
        let tls_cert = config.triggers.tls_cert.clone();
        let tls_key = config.triggers.tls_key.clone();
        let trigger_key = nanoid!(32);

        // Convert trigger environment variables to be properly structured;
        // GOFER_TRIGGER_<trigger_name>_<variable_key>
        let settings_variables = settings.variables.clone();
        let settings_variables = settings_variables.into_iter().map(|(key, value)| {
            let key = format!(
                "GOFER_TRIGGER_{}_{}",
                &settings.name.to_uppercase(),
                key.to_uppercase()
            );
            (key, value)
        });

        let mut gofer_variables: HashMap<String, String> = [
            ("GOFER_TRIGGER_TLS_CERT".into(), tls_cert),
            ("GOFER_TRIGGER_TLS_KEY".into(), tls_key),
            ("GOFER_TRIGGER_NAME".into(), settings.name.clone()),
            ("GOFER_TRIGGER_LOG_LEVEL".into(), config.general.log_level),
            ("GOFER_TRIGGER_KEY".into(), trigger_key.clone()),
        ]
        .iter()
        .cloned()
        .collect();

        // We need to combine all the variables together to send to the trigger container.
        // Order is important here as the later maps added will overwrite conflicting keys
        // on earlier maps.
        //
        // The reasoning for the order below is that user inserted keys should probably
        // have priority over Gofer inserted keys. This unfortunately gives the user a
        // foot-gun since they can accidentally overwrite a variable, but in the case
        // that the Gofer supplied variable actually needs to be changed for a good reason
        // the user has full control over the final variables that are injected.
        gofer_variables.extend(settings_variables);

        // This is the name we use to identify our container to the scheduler. We need it to be unique
        // potentially among ALL other containers running on the system.
        let container_name = fmt::trigger_container_id(&settings.name);

        let resp = self
            .scheduler
            .start_container(scheduler::StartContainerRequest {
                name: container_name.clone(),
                image: settings.image.clone(),
                variables: gofer_variables,
                registry_auth: {
                    if settings.user.is_some() {
                        Some(scheduler::RegistryAuth {
                            user: settings.user.clone().unwrap_or_default(),
                            pass: settings.pass.clone().unwrap_or_default(),
                        })
                    } else {
                        None
                    }
                },
                always_pull: true,
                enable_networking: true,
                entrypoint: vec![],
                command: vec![],
            })
            .await?;

        let url = resp.url.as_ref().ok_or_else(|| {
            anyhow!(
                "could not start trigger, scheduler did not return proper networking information"
            )
        })?;

        let info = self
            .healthcheck_trigger(&container_name, url, &trigger_key, 30)
            .await?;

        Ok(TriggerInfo {
            scheduler_id: resp.scheduler_id,
            url: url.to_string(),
            documentation: info.documentation,
            key: trigger_key,
        })
    }

    pub async fn start_triggers(self: Arc<Self>) -> Result<()> {
        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("could not start trigger"; "error" => format!("{:?}", e));
                return Err(anyhow!(
                    "could not start trigger; connection error; {:?}",
                    e
                ));
            }
        };

        let registrations = storage::trigger_registrations::list(&mut conn, 0, 0).await?;

        for trigger in registrations {
            let trigger_info = match self.start_trigger(&trigger).await {
                Ok(info) => info,
                Err(e) => {
                    error!("could not start trigger"; "name" => trigger.name, "error" => format!("{:?}", e));
                    continue;
                }
            };

            self.triggers.insert(
                trigger.name.clone(),
                gofer_models::trigger::Trigger {
                    registration: trigger.clone(),
                    url: Some(trigger_info.url.clone()),
                    scheduler_id: trigger_info.scheduler_id.clone(),
                    started: epoch(),
                    state: gofer_models::trigger::State::Running,
                    status: gofer_models::trigger::Status::Enabled,
                    documentation: {
                        if !trigger_info.documentation.is_empty() {
                            Some(trigger_info.documentation.clone())
                        } else {
                            None
                        }
                    },
                    key: Some(trigger_info.key.clone()),
                },
            );

            let self_clone = self.clone();
            let trigger_name = trigger.name.clone();
            tokio::spawn(async move {
                self_clone
                    .echo_logs_to_console(fmt::trigger_container_id(&trigger_name))
                    .await
            });

            info!("Started trigger"; "name" => trigger.name, "image" => trigger.image, "url" => trigger_info.url);
        }

        Ok(())
    }

    /// Attempt to stop a trigger by name. Attempts to perform a graceful shutdown by calling the trigger's
    /// shutdown method.
    pub async fn stop_trigger(&self, trigger: &Trigger) -> Result<()> {
        let url = match &trigger.url {
            Some(url) => url,
            None => return Err(anyhow::anyhow!("trigger has no url")),
        };

        let key = match &trigger.key {
            Some(key) => key,
            None => return Err(anyhow::anyhow!("trigger has no auth key")),
        };

        let tls_config = get_tls_client_config(url, self.conf.triggers.tls_ca.clone())?;
        let channel = Channel::from_shared(url.clone())?
            .tls_config(tls_config.clone())?
            .connect()
            .await?;

        let token: MetadataValue<_> = format!("Bearer {}", key).parse()?;

        let mut client =
            TriggerServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
                req.metadata_mut().insert("authorization", token.clone());
                Ok(req)
            });

        let request = tonic::Request::new(TriggerShutdownRequest {});
        if let Err(e) = client.shutdown(request).await {
            return Err(anyhow!("failed to call shutdown on trigger; {:?}", e));
        }

        Ok(())
    }

    /// Sends a shutdown request for each trigger Gofer knows about.
    pub async fn stop_all_triggers(&self) {
        for entry in &self.triggers {
            if let Err(e) = self.stop_trigger(entry.value()).await {
                error!("could not stop trigger; {}", e);
                continue;
            }
        }

        self.triggers.clear()
    }

    /// Check that the trigger's container has started and check that we can send the trigger the initial info packet.
    /// Returns when trigger is in state running and a successful info request has been made.
    /// attempts_limit allows the caller to set how many times they would like to retry.
    async fn healthcheck_trigger(
        &self,
        name: &str,
        url: &str,
        trigger_key: &str,
        attempts_limit: u8,
    ) -> Result<TriggerInfoResponse> {
        loop {
            let resp = self
                .scheduler
                .get_state(scheduler::GetStateRequest {
                    name: name.to_string(),
                })
                .await?;

            if resp.state != scheduler::ContainerState::Exited
                && resp.state != scheduler::ContainerState::Running
            {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                continue;
            }

            break;
        }

        let tls_config = get_tls_client_config(url, self.conf.triggers.tls_ca.clone())?;

        let mut attempts: u8 = 0;
        loop {
            let url = url.to_owned();
            let channel = Channel::from_shared(url.clone())?
                .tls_config(tls_config.clone())?
                .connect()
                .await?;

            let token: MetadataValue<_> = format!("Bearer {}", trigger_key).parse()?;

            let mut client =
                TriggerServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
                    req.metadata_mut().insert("authorization", token.clone());
                    Ok(req)
                });

            let request = tonic::Request::new(TriggerInfoRequest {});
            match client.info(request).await {
                Ok(response) => return Ok(response.into_inner()),
                Err(e) => {
                    attempts += 1;

                    if attempts >= attempts_limit {
                        return Err(anyhow!(
                            "failed to connect to trigger as part of startup validation checks; {:?}",
                            e.to_string()
                        ));
                    }

                    debug!("failed to connect to trigger as part of startup validation checks; retrying";
                           "attempt_num" => attempts,
                           "url" => format!("{:?}", &url), "error" => format!("{:?}", e.message()));
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    continue;
                }
            };
        }
    }

    /// Take a container and print the logs to stdout. Used to allow triggers to print into server logs.
    pub async fn echo_logs_to_console(&self, container_name: String) {
        let mut log_stream = self.scheduler.get_logs(scheduler::GetLogsRequest {
            name: container_name,
        });

        while let Some(log) = log_stream.next().await {
            let log = match log {
                Ok(log) => log,
                Err(e) => {
                    error!("encountered error while echoing log file to console"; "error" => format!("{:?}", e));
                    return;
                }
            };

            match log {
                scheduler::Log::Unknown => {
                    error!("encountered error while echoing log file to console; log line unknown but should be stdout/stderr");
                    return;
                }
                scheduler::Log::Stderr(log) | scheduler::Log::Stdout(log) => {
                    debug!("{}", String::from_utf8_lossy(&log))
                }
            }
        }
    }
}
