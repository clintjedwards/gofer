mod service;
mod trigger;
mod validate;

use crate::{conf, events, frontend, scheduler, storage};
use gofer_proto::{
    gofer_server::{Gofer, GoferServer},
    *,
};

use dashmap::DashMap;
use futures::Stream;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tonic::{Request, Response, Status};

const BUILD_SEMVER: &str = env!("BUILD_SEMVER");
const BUILD_COMMIT: &str = env!("BUILD_COMMIT");

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("could not successfully validate arguments; {0}")]
    InvalidArguments(String),
}

fn epoch() -> u64 {
    let current_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    u64::try_from(current_epoch).unwrap()
}

/// Checks expressions passed (currently restricted to those that are is_empty-able) to make sure they aren't the
/// zero value.
macro_rules! require_args {
    ($var:expr) => {
        if $var.is_empty() {
            return Err(Status::failed_precondition(format!("missing/empty required argument '{}'", {
                if stringify!($var).contains('.') {
                    let (_, var) = stringify!($var).rsplit_once('.').unwrap();
                    var
                } else {
                    stringify!($var)
                }
            })));
        }
    };
    ($var:expr, $($var_m:expr),+) => {
        require_args! {$var}
        require_args! {$($var_m),+}
    }
}

pub struct Api {
    /// Various configurations needed by the api
    conf: conf::api::Config,

    /// The main backend storage implementation. Gofer stores most of its critical state information here.
    storage: storage::Db,

    /// The mechanism in which Gofer uses to run individual containers.
    scheduler: Box<dyn scheduler::Scheduler + Sync + Send>,

    /// Used throughout the whole application in order to allow functions to wait on state changes in Gofer.
    event_bus: events::EventBus,

    /// Triggers is an in-memory map of currently registered and started triggers.
    /// This is necessary due to triggers being based on containers and their state needing to be constantly
    /// updated and maintained.
    triggers: DashMap<String, gofer_models::Trigger>,
}

#[tonic::async_trait]
impl Gofer for Api {
    async fn get_system_info(
        &self,
        _: Request<GetSystemInfoRequest>,
    ) -> Result<Response<GetSystemInfoResponse>, Status> {
        Ok(Response::new(GetSystemInfoResponse {
            commit: BUILD_COMMIT.to_string(),
            dev_mode_enabled: self.conf.general.dev_mode,
            semver: BUILD_SEMVER.to_string(),
        }))
    }

    async fn list_namespaces(
        &self,
        request: Request<ListNamespacesRequest>,
    ) -> Result<Response<ListNamespacesResponse>, Status> {
        let args = &request.into_inner();

        self.storage
            .list_namespaces(args.offset, args.limit)
            .await
            .map(|namespaces| {
                Response::new(ListNamespacesResponse {
                    namespaces: namespaces.into_iter().map(Namespace::from).collect(),
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn create_namespace(
        &self,
        request: Request<CreateNamespaceRequest>,
    ) -> Result<Response<CreateNamespaceResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.id, args.name);

        if let Err(e) = validate::identifier(&args.id) {
            return Err(Status::failed_precondition(e.to_string()));
        }

        let new_namespace = gofer_models::Namespace::new(&args.id, &args.name, &args.description);

        self.storage
            .create_namespace(&new_namespace)
            .await
            .map_err(|e| match e {
                storage::StorageError::Exists => Status::already_exists(format!(
                    "namespace with id '{}' already exists",
                    new_namespace.id
                )),
                _ => Status::internal(e.to_string()),
            })?;

        self.event_bus
            .publish(gofer_models::EventKind::CreatedNamespace {
                namespace_id: new_namespace.id.clone(),
            })
            .await;
        Ok(Response::new(CreateNamespaceResponse {
            namespace: Some(new_namespace.into()),
        }))
    }

    async fn get_namespace(
        &self,
        request: Request<GetNamespaceRequest>,
    ) -> Result<Response<GetNamespaceResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.id);

        self.storage
            .get_namespace(&args.id)
            .await
            .map(|namespace| {
                Response::new(GetNamespaceResponse {
                    namespace: Some(namespace.into()),
                })
            })
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("namespace with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })
    }

    async fn update_namespace(
        &self,
        request: Request<UpdateNamespaceRequest>,
    ) -> Result<Response<UpdateNamespaceResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.id, args.name);

        self.storage
            .update_namespace(&gofer_models::Namespace {
                id: args.id.clone(),
                name: args.name.clone(),
                description: args.description.clone(),
                created: 0,
                modified: epoch(),
            })
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("namespace with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(UpdateNamespaceResponse {}))
    }

    async fn delete_namespace(
        &self,
        request: Request<DeleteNamespaceRequest>,
    ) -> Result<Response<DeleteNamespaceResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.id);

        self.storage
            .delete_namespace(&args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("namespace with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        self.event_bus
            .publish(gofer_models::EventKind::DeletedNamespace {
                namespace_id: args.id.clone(),
            })
            .await;
        Ok(Response::new(DeleteNamespaceResponse {}))
    }

    async fn list_pipelines(
        &self,
        request: Request<ListPipelinesRequest>,
    ) -> Result<Response<ListPipelinesResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id);

        self.storage
            .list_pipelines(args.offset as u64, args.limit as u64, &args.namespace_id)
            .await
            .map(|pipelines| {
                Response::new(ListPipelinesResponse {
                    pipelines: pipelines.into_iter().map(Pipeline::from).collect(),
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn create_pipeline(
        &self,
        request: Request<CreatePipelineRequest>,
    ) -> Result<Response<CreatePipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id);

        let pipeline_config = match &args.pipeline_config {
            Some(config) => config,
            None => {
                return Err(Status::failed_precondition(
                    "must include valid pipeline config",
                ));
            }
        };

        let new_pipeline =
            gofer_models::Pipeline::new(&args.namespace_id, pipeline_config.to_owned().into());

        self.storage
            .create_pipeline(&new_pipeline)
            .await
            .map_err(|e| match e {
                storage::StorageError::Exists => Status::already_exists(format!(
                    "pipeline with id '{}' already exists",
                    new_pipeline.id
                )),
                _ => Status::internal(e.to_string()),
            })?;

        self.event_bus
            .publish(gofer_models::EventKind::CreatedPipeline {
                namespace_id: new_pipeline.namespace.clone(),
                pipeline_id: new_pipeline.id.clone(),
            })
            .await;
        Ok(Response::new(CreatePipelineResponse {
            pipeline: Some(new_pipeline.into()),
        }))
    }

    async fn get_pipeline(
        &self,
        request: Request<GetPipelineRequest>,
    ) -> Result<Response<GetPipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.id);

        self.storage
            .get_pipeline(&args.namespace_id, &args.id)
            .await
            .map(|pipeline| {
                Response::new(GetPipelineResponse {
                    pipeline: Some(pipeline.into()),
                })
            })
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })
    }

    async fn run_pipeline(
        &self,
        request: Request<RunPipelineRequest>,
    ) -> Result<Response<RunPipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.id);

        // check parrellism here through events

        self.storage
            .get_pipeline(&args.namespace_id, &args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        unimplemented!();

        //Ok(Response::new(RunPipelineResponse {}))
    }

    async fn enable_pipeline(
        &self,
        request: Request<EnablePipelineRequest>,
    ) -> Result<Response<EnablePipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.id);

        self.storage
            .update_pipeline_state(
                &args.namespace_id,
                &args.id,
                gofer_models::PipelineState::Active,
            )
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(EnablePipelineResponse {}))
    }

    async fn disable_pipeline(
        &self,
        request: Request<DisablePipelineRequest>,
    ) -> Result<Response<DisablePipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.id);

        self.storage
            .update_pipeline_state(
                &args.namespace_id,
                &args.id,
                gofer_models::PipelineState::Disabled,
            )
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(DisablePipelineResponse {}))
    }

    async fn update_pipeline(
        &self,
        request: Request<UpdatePipelineRequest>,
    ) -> Result<Response<UpdatePipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id);

        let pipeline_config = match &args.pipeline_config {
            Some(config) => config,
            None => {
                return Err(Status::failed_precondition(
                    "must include valid pipeline config",
                ));
            }
        };

        let new_pipeline =
            gofer_models::Pipeline::new(&args.namespace_id, pipeline_config.to_owned().into());

        self.storage
            .update_pipeline(&new_pipeline)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => Status::not_found(format!(
                    "pipeline with id '{}' does not exist",
                    &new_pipeline.id
                )),
                _ => Status::internal(e.to_string()),
            })?;

        Ok(Response::new(UpdatePipelineResponse {
            pipeline: Some(new_pipeline.into()),
        }))
    }

    async fn delete_pipeline(
        &self,
        request: Request<DeletePipelineRequest>,
    ) -> Result<Response<DeletePipelineResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.id);

        self.storage
            .delete_pipeline(&args.namespace_id, &args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        self.event_bus
            .publish(gofer_models::EventKind::DeletedPipeline {
                namespace_id: args.namespace_id.clone(),
                pipeline_id: args.id.clone(),
            })
            .await;
        Ok(Response::new(DeletePipelineResponse {}))
    }

    async fn get_run(
        &self,
        request: Request<GetRunRequest>,
    ) -> Result<Response<GetRunResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.pipeline_id);

        if args.id == 0 {
            return Err(Status::failed_precondition("must include target run id"));
        }

        self.storage
            .get_run(&args.namespace_id, &args.pipeline_id, args.id)
            .await
            .map(|run| {
                Response::new(GetRunResponse {
                    run: Some(run.into()),
                })
            })
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("run with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })
    }

    async fn list_runs(
        &self,
        request: Request<ListRunsRequest>,
    ) -> Result<Response<ListRunsResponse>, Status> {
        let args = &request.into_inner();
        require_args!(args.namespace_id, args.pipeline_id);

        self.storage
            .list_runs(
                args.offset as u64,
                args.limit as u64,
                &args.namespace_id,
                &args.pipeline_id,
            )
            .await
            .map(|runs| {
                Response::new(ListRunsResponse {
                    runs: runs.into_iter().map(Run::from).collect(),
                })
            })
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn retry_run(
        &self,
        request: Request<RetryRunRequest>,
    ) -> Result<Response<RetryRunResponse>, Status> {
        todo!()
    }

    async fn cancel_run(
        &self,
        request: Request<CancelRunRequest>,
    ) -> Result<Response<CancelRunResponse>, Status> {
        todo!()
    }

    async fn cancel_all_runs(
        &self,
        request: Request<CancelAllRunsRequest>,
    ) -> Result<Response<CancelAllRunsResponse>, Status> {
        todo!()
    }

    async fn get_task_run(
        &self,
        request: Request<GetTaskRunRequest>,
    ) -> Result<Response<GetTaskRunResponse>, Status> {
        todo!()
    }

    async fn list_task_runs(
        &self,
        request: Request<ListTaskRunsRequest>,
    ) -> Result<Response<ListTaskRunsResponse>, Status> {
        todo!()
    }

    async fn cancel_task_run(
        &self,
        request: Request<CancelTaskRunRequest>,
    ) -> Result<Response<CancelTaskRunResponse>, Status> {
        todo!()
    }

    type GetTaskRunLogsStream =
        Pin<Box<dyn Stream<Item = Result<GetTaskRunLogsResponse, Status>> + Send>>;

    async fn get_task_run_logs(
        &self,
        request: Request<GetTaskRunLogsRequest>,
    ) -> Result<Response<Self::GetTaskRunLogsStream>, Status> {
        todo!()
    }

    async fn delete_task_run_logs(
        &self,
        request: Request<DeleteTaskRunLogsRequest>,
    ) -> Result<Response<DeleteTaskRunLogsResponse>, Status> {
        todo!()
    }

    async fn get_trigger(
        &self,
        request: Request<GetTriggerRequest>,
    ) -> Result<Response<GetTriggerResponse>, Status> {
        todo!()
    }

    async fn list_triggers(
        &self,
        request: Request<ListTriggersRequest>,
    ) -> Result<Response<ListTriggersResponse>, Status> {
        todo!()
    }

    async fn install_trigger(
        &self,
        request: Request<InstallTriggerRequest>,
    ) -> Result<Response<InstallTriggerResponse>, Status> {
        let args = request.into_inner();
        require_args!(args.name, args.image);

        if self.triggers.contains_key(&args.name) {
            return Err(Status::already_exists(format!(
                "trigger '{}' already exists",
                &args.name
            )));
        }

        self.storage
            .create_trigger_registration(&args.clone().into())
            .await
            .map_err(|e| match e {
                storage::StorageError::Exists => Status::already_exists(format!(
                    "trigger with name '{}' already exists",
                    &args.name
                )),
                _ => Status::internal(e.to_string()),
            })?;

        self.start_trigger(&args.clone().into())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(InstallTriggerResponse {}))
    }

    async fn uninstall_trigger(
        &self,
        request: Request<UninstallTriggerRequest>,
    ) -> Result<Response<UninstallTriggerResponse>, Status> {
        todo!()
    }

    async fn get_notifier(
        &self,
        request: Request<GetNotifierRequest>,
    ) -> Result<Response<GetNotifierResponse>, Status> {
        todo!()
    }

    async fn list_notifiers(
        &self,
        request: Request<ListNotifiersRequest>,
    ) -> Result<Response<ListNotifiersResponse>, Status> {
        todo!()
    }

    async fn install_notifier(
        &self,
        request: Request<InstallNotifierRequest>,
    ) -> Result<Response<InstallNotifierResponse>, Status> {
        todo!()
    }

    async fn uninstall_notifier(
        &self,
        request: Request<UninstallNotifierRequest>,
    ) -> Result<Response<UninstallNotifierResponse>, Status> {
        todo!()
    }
}

impl Api {
    /// Create a new instance of API with all services started.
    pub async fn start(conf: conf::api::Config) {
        let storage = storage::Db::new(&conf.server.storage_path).await.unwrap();
        let scheduler = scheduler::init_scheduler(&conf.scheduler).await.unwrap();
        let event_bus = events::EventBus::new(
            storage.clone(),
            conf.general.event_retention,
            conf.general.event_prune_interval,
        );

        let api = Api {
            conf,
            storage,
            scheduler,
            event_bus,
            triggers: DashMap::new(),
        };

        api.create_default_namespace().await.unwrap();
        api.start_service().await;
    }

    /// Gofer starts with a default namespace that all users have access to.
    async fn create_default_namespace(&self) -> Result<(), storage::StorageError> {
        const DEFAULT_NAMESPACE_ID: &str = "default";
        const DEFAULT_NAMESPACE_NAME: &str = "Default";
        const DEFAULT_NAMESPACE_DESCRIPTION: &str =
            "The default namespace when no other namespace is specified.";

        let default_namespace = gofer_models::Namespace::new(
            DEFAULT_NAMESPACE_ID,
            DEFAULT_NAMESPACE_NAME,
            DEFAULT_NAMESPACE_DESCRIPTION,
        );

        match self.storage.create_namespace(&default_namespace).await {
            Ok(_) => {
                self.event_bus
                    .publish(gofer_models::EventKind::CreatedNamespace {
                        namespace_id: DEFAULT_NAMESPACE_ID.to_string(),
                    })
                    .await;
                Ok(())
            }
            Err(e) => match e {
                storage::StorageError::Exists => Ok(()),
                _ => Err(e),
            },
        }
    }

    /// Start a TLS enabled, multiplexed, grpc/http server.
    async fn start_service(self) {
        let rest =
            axum::Router::new().route("/*path", axum::routing::any(frontend::frontend_handler));

        let config = self.conf.clone();
        let cert = config.server.tls_cert.clone().into_bytes();
        let key = config.server.tls_key.clone().into_bytes();

        let grpc = GoferServer::new(self);

        let service = service::MultiplexService { rest, grpc };

        if config.general.dev_mode {
            service::start_server(service, &config.server.url).await;
            return;
        }

        service::start_tls_server(service, &config.server.url, cert, key).await;
    }
}
