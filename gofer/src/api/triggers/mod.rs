mod utils;

use crate::{
    api::{epoch, validate, Api},
    scheduler, storage,
};
use futures::stream::StreamExt;
use gofer_models::{event, trigger};
use gofer_proto::{
    DisableTriggerRequest, DisableTriggerResponse, EnableTriggerRequest, EnableTriggerResponse,
    GetTriggerInstallInstructionsRequest, GetTriggerInstallInstructionsResponse, GetTriggerRequest,
    GetTriggerResponse, InstallTriggerRequest, InstallTriggerResponse, ListTriggersRequest,
    ListTriggersResponse, UninstallTriggerRequest, UninstallTriggerResponse,
};
use nanoid::nanoid;
use slog_scope::info;
use std::collections::HashMap;
use tonic::{Response, Status};

impl Api {
    pub async fn install_trigger_handler(
        &self,
        args: InstallTriggerRequest,
    ) -> Result<Response<InstallTriggerResponse>, Status> {
        validate::arg("name", args.name.clone(), vec![validate::not_empty_str])?;
        validate::arg("image", args.image.clone(), vec![validate::not_empty_str])?;

        // Check to see if this trigger has been registered already.
        if self.triggers.contains_key(&args.name) {
            return Err(Status::already_exists(format!(
                "trigger '{}' already exists",
                &args.name
            )));
        }

        let trigger_info = self
            .start_trigger(&args.clone().into())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::trigger_registrations::insert(&mut conn, &args.clone().into())
            .await
            .map_err(|e| match e {
                storage::StorageError::Exists => Status::already_exists(format!(
                    "trigger with name '{}' already exists",
                    &args.name
                )),
                _ => Status::internal(e.to_string()),
            })?;

        self.triggers.insert(
            args.name.clone(),
            trigger::Trigger {
                registration: args.clone().into(),
                url: Some(trigger_info.url.clone()),
                scheduler_id: trigger_info.scheduler_id.clone(),
                started: epoch(),
                state: trigger::State::Running,
                status: trigger::Status::Enabled,
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

        self.event_bus
            .publish(event::Kind::InstalledTrigger {
                name: args.name.clone(),
                image: args.image.clone(),
            })
            .await;

        info!("installed trigger"; "name" => &args.name, "image" => &args.image, "url" => trigger_info.url);

        Ok(Response::new(InstallTriggerResponse {}))
    }

    pub async fn get_trigger_install_instructions_handler(
        &self,
        args: GetTriggerInstallInstructionsRequest,
    ) -> Result<Response<GetTriggerInstallInstructionsResponse>, Status> {
        validate::arg("image", args.image.clone(), vec![validate::not_empty_str])?;

        let config = self.conf.clone();
        let tls_cert = config.triggers.tls_cert.clone();
        let tls_key = config.triggers.tls_key.clone();
        let trigger_key = nanoid!(32);
        let trigger_name = nanoid!(10);

        let gofer_variables: HashMap<String, String> = [
            ("GOFER_TRIGGER_TLS_CERT".into(), tls_cert),
            ("GOFER_TRIGGER_TLS_KEY".into(), tls_key),
            ("GOFER_TRIGGER_NAME".into(), trigger_name.clone()),
            ("GOFER_TRIGGER_LOG_LEVEL".into(), config.general.log_level),
            ("GOFER_TRIGGER_KEY".into(), trigger_key.clone()),
        ]
        .iter()
        .cloned()
        .collect();

        // This is the name we use to identify our container to the scheduler. We need it to be unique
        // potentially among ALL other containers running on the system.
        let fmtted_container_name = format!("trigger_{}", trigger_name);

        self.scheduler
            .start_container(scheduler::StartContainerRequest {
                name: fmtted_container_name.clone(),
                image: args.image.clone(),
                variables: gofer_variables,
                registry_auth: {
                    if !args.user.is_empty() {
                        Some(scheduler::RegistryAuth {
                            user: args.user.clone(),
                            pass: args.pass.clone(),
                        })
                    } else {
                        None
                    }
                },
                always_pull: true,
                enable_networking: false,
                entrypoint: vec!["./trigger".into(), "installer".into()],
                command: vec![],
            })
            .await
            .map_err(|e| Status::internal(format!("could not start container: {}", e)))?;

        let log_stream = self.scheduler.get_logs(scheduler::GetLogsRequest {
            name: fmtted_container_name,
        });

        let log_output = log_stream
            .collect::<Vec<Result<scheduler::Log, scheduler::SchedulerError>>>()
            .await;
        let log_output = log_output.last().ok_or_else(|| {
            Status::internal(
                "could not get installation instructions;
                empty string found when container run in installer mode"
                    .to_string(),
            )
        })?;

        let last_line = log_output.as_ref().map_err(|e| {
            Status::internal(format!("could not get last line of container logs: {}", e))
        })?;

        let last_line = match last_line {
            scheduler::Log::Unknown => {
                return Err(Status::internal(
                    "could not get last line of container logs; log type unknown".to_string(),
                ));
            }
            scheduler::Log::Stdout(bytes) | scheduler::Log::Stderr(bytes) => {
                String::from_utf8_lossy(bytes).to_string()
            }
        };

        Ok(Response::new(GetTriggerInstallInstructionsResponse {
            instructions: last_line,
        }))
    }

    pub async fn get_trigger_handler(
        &self,
        args: GetTriggerRequest,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        validate::arg(
            "name",
            args.name.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        match self.triggers.get(&args.name) {
            Some(entry) => {
                let trigger = entry.value();
                Ok(Response::new(GetTriggerResponse {
                    trigger: Some(trigger.clone().into()),
                }))
            }
            None => Err(Status::failed_precondition("trigger does not exist")),
        }
    }

    pub async fn list_triggers_handler(
        &self,
        _: ListTriggersRequest,
    ) -> Result<Response<ListTriggersResponse>, Status> {
        let triggers: Vec<gofer_proto::Trigger> = self
            .triggers
            .iter()
            .map(|trigger| trigger.value().clone().into())
            .collect();

        Ok(Response::new(ListTriggersResponse { triggers }))
    }

    pub async fn uninstall_trigger_handler(
        &self,
        args: UninstallTriggerRequest,
    ) -> Result<Response<UninstallTriggerResponse>, Status> {
        validate::arg(
            "name",
            args.name.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        // We create an inner scope here so we don't deadlock on the remove call
        // because we have a reference into the map.
        {
            let trigger = match self.triggers.get(&args.name) {
                Some(trigger) => trigger,
                None => return Err(Status::failed_precondition("trigger does not exist")),
            };
            let trigger = trigger.value();

            if let Err(e) = self.stop_trigger(trigger).await {
                return Err(Status::internal(format!("could not stop trigger; {:?}", e)));
            };
        }

        self.triggers.remove(&args.name);

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if let Err(e) = storage::trigger_registrations::delete(&mut conn, &args.name).await {
            return Err(Status::internal(format!(
                "could not remove trigger registration {:?}",
                e
            )));
        };

        Ok(Response::new(UninstallTriggerResponse {}))
    }

    pub async fn enable_trigger_handler(
        &self,
        args: EnableTriggerRequest,
    ) -> Result<Response<EnableTriggerResponse>, Status> {
        validate::arg(
            "name",
            args.name.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        self.triggers.alter(&args.name, |_, mut value| {
            value.status = trigger::Status::Enabled;
            value
        });

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if let Err(e) = storage::trigger_registrations::update(
            &mut conn,
            &args.name,
            storage::trigger_registrations::UpdatableFields {
                status: Some(trigger::Status::Enabled),
                ..Default::default()
            },
        )
        .await
        {
            return Err(Status::internal(format!(
                "could not update trigger registration status; {:?}",
                e
            )));
        };

        Ok(Response::new(EnableTriggerResponse {}))
    }

    pub async fn disable_trigger_handler(
        &self,
        args: DisableTriggerRequest,
    ) -> Result<Response<DisableTriggerResponse>, Status> {
        validate::arg(
            "name",
            args.name.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        self.triggers.alter(&args.name, |_, mut value| {
            value.status = trigger::Status::Disabled;
            value
        });

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if let Err(e) = storage::trigger_registrations::update(
            &mut conn,
            &args.name,
            storage::trigger_registrations::UpdatableFields {
                status: Some(trigger::Status::Disabled),
                ..Default::default()
            },
        )
        .await
        {
            return Err(Status::internal(format!(
                "could not update trigger registration status; {:?}",
                e
            )));
        };

        Ok(Response::new(DisableTriggerResponse {}))
    }
}
