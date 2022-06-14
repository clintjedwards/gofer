mod service;
mod validate;

use crate::{conf, frontend, models, storage};
use gofer_proto::{
    gofer_server::{Gofer, GoferServer},
    *,
};

use futures::Stream;
use slog_scope::info;
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

#[derive(Clone)]
pub struct Api {
    conf: conf::api::Config,
    storage: storage::Db,
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

        let result = self.storage.list_namespaces(args.offset, args.limit).await;

        match result {
            Ok(namespaces_raw) => {
                let namespaces = namespaces_raw
                    .into_iter()
                    .map(gofer_proto::Namespace::from)
                    .collect();
                return Ok(Response::new(ListNamespacesResponse { namespaces }));
            }
            Err(storage_err) => return Err(Status::internal(storage_err.to_string())),
        }
    }

    async fn create_namespace(
        &self,
        request: Request<CreateNamespaceRequest>,
    ) -> Result<Response<CreateNamespaceResponse>, Status> {
        let args = &request.into_inner();

        if let Err(e) = validate::identifier(&args.id) {
            return Err(Status::failed_precondition(e.to_string()));
        }

        let new_namespace = models::Namespace::new(&args.id, &args.name, &args.description);

        let result = self.storage.create_namespace(&new_namespace).await;
        match result {
            Ok(_) => (),
            Err(e) => match e {
                storage::StorageError::Exists => {
                    return Err(Status::already_exists(format!(
                        "namespace with id '{}' already exists",
                        new_namespace.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        info!("Created new namespace"; "namespace" => format!("{:?}", new_namespace));
        Ok(Response::new(CreateNamespaceResponse {
            namespace: Some(new_namespace.into()),
        }))
    }

    async fn get_namespace(
        &self,
        request: Request<GetNamespaceRequest>,
    ) -> Result<Response<GetNamespaceResponse>, Status> {
        let args = &request.into_inner();

        let result = self.storage.get_namespace(&args.id).await;
        let namespace = match result {
            Ok(namespace) => namespace,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "namespace with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(GetNamespaceResponse {
            namespace: Some(namespace.into()),
        }))
    }

    async fn update_namespace(
        &self,
        request: Request<UpdateNamespaceRequest>,
    ) -> Result<Response<UpdateNamespaceResponse>, Status> {
        let args = &request.into_inner();

        let result = self
            .storage
            .update_namespace(&models::Namespace {
                id: args.id.clone(),
                name: args.name.clone(),
                description: args.description.clone(),
                created: 0,
                modified: epoch(),
            })
            .await;

        match result {
            Ok(_) => (),
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "namespace with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(UpdateNamespaceResponse {}))
    }

    async fn delete_namespace(
        &self,
        request: Request<DeleteNamespaceRequest>,
    ) -> Result<Response<DeleteNamespaceResponse>, Status> {
        let args = &request.into_inner();

        let result = self.storage.delete_namespace(&args.id).await;
        match result {
            Ok(_) => (),
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "namespace with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        info!("Deleted namespace"; "id" => &args.id);
        Ok(Response::new(DeleteNamespaceResponse {}))
    }

    async fn list_pipelines(
        &self,
        request: Request<ListPipelinesRequest>,
    ) -> Result<Response<ListPipelinesResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        let result = self
            .storage
            .list_pipelines(args.offset as u64, args.limit as u64, &args.namespace_id)
            .await;

        match result {
            Ok(pipelines_raw) => {
                let pipelines = pipelines_raw
                    .into_iter()
                    .map(gofer_proto::Pipeline::from)
                    .collect();
                return Ok(Response::new(ListPipelinesResponse { pipelines }));
            }
            Err(storage_err) => return Err(Status::internal(storage_err.to_string())),
        }
    }

    async fn create_pipeline(
        &self,
        request: Request<CreatePipelineRequest>,
    ) -> Result<Response<CreatePipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        let pipeline_config = match &args.pipeline_config {
            Some(config) => config,
            None => {
                return Err(Status::failed_precondition(
                    "must include valid pipeline config",
                ));
            }
        };

        let new_pipeline =
            models::Pipeline::new(&args.namespace_id, pipeline_config.to_owned().into());

        let result = self.storage.create_pipeline(&new_pipeline).await;
        match result {
            Ok(_) => (),
            Err(e) => match e {
                storage::StorageError::Exists => {
                    return Err(Status::already_exists(format!(
                        "pipeline with id '{}' already exists",
                        new_pipeline.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        info!("Created new pipeline"; "pipeline" => format!("{:?}", new_pipeline));
        Ok(Response::new(CreatePipelineResponse {
            pipeline: Some(new_pipeline.into()),
        }))
    }

    async fn get_pipeline(
        &self,
        request: Request<GetPipelineRequest>,
    ) -> Result<Response<GetPipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        let result = self
            .storage
            .get_pipeline(&args.namespace_id, &args.id)
            .await;
        let pipeline = match result {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "pipeline with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(GetPipelineResponse {
            pipeline: Some(pipeline.into()),
        }))
    }

    async fn run_pipeline(
        &self,
        request: Request<RunPipelineRequest>,
    ) -> Result<Response<RunPipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        let result = self
            .storage
            .get_pipeline(&args.namespace_id, &args.id)
            .await;

        if let Err(e) = result {
            match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "pipeline with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            }
        }

        unimplemented!();

        //Ok(Response::new(RunPipelineResponse {}))
    }

    async fn enable_pipeline(
        &self,
        request: Request<EnablePipelineRequest>,
    ) -> Result<Response<EnablePipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        let result = self
            .storage
            .update_pipeline_state(&args.namespace_id, &args.id, models::PipelineState::Active)
            .await;
        match result {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "pipeline with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(EnablePipelineResponse {}))
    }

    async fn disable_pipeline(
        &self,
        request: Request<DisablePipelineRequest>,
    ) -> Result<Response<DisablePipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        let result = self
            .storage
            .update_pipeline_state(
                &args.namespace_id,
                &args.id,
                models::PipelineState::Disabled,
            )
            .await;
        match result {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "pipeline with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(DisablePipelineResponse {}))
    }

    async fn update_pipeline(
        &self,
        request: Request<UpdatePipelineRequest>,
    ) -> Result<Response<UpdatePipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        let pipeline_config = match &args.pipeline_config {
            Some(config) => config,
            None => {
                return Err(Status::failed_precondition(
                    "must include valid pipeline config",
                ));
            }
        };

        let new_pipeline =
            models::Pipeline::new(&args.namespace_id, pipeline_config.to_owned().into());

        let result = self.storage.update_pipeline(&new_pipeline).await;
        match result {
            Ok(_) => (),
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "pipeline with id '{}' does not exist",
                        &new_pipeline.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(UpdatePipelineResponse {
            pipeline: Some(new_pipeline.into()),
        }))
    }

    async fn delete_pipeline(
        &self,
        request: Request<DeletePipelineRequest>,
    ) -> Result<Response<DeletePipelineResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        let result = self
            .storage
            .delete_pipeline(&args.namespace_id, &args.id)
            .await;
        match result {
            Ok(_) => (),
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "pipeline with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        info!("Deleted pipeline"; "id" => &args.id);
        Ok(Response::new(DeletePipelineResponse {}))
    }

    async fn get_run(
        &self,
        request: Request<GetRunRequest>,
    ) -> Result<Response<GetRunResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.pipeline_id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        if args.id == 0 {
            return Err(Status::failed_precondition("must include target run id"));
        }

        let result = self
            .storage
            .get_run(&args.namespace_id, &args.pipeline_id, args.id)
            .await;

        let run = match result {
            Ok(run) => run,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "run with id '{}' does not exist",
                        &args.id
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };

        Ok(Response::new(GetRunResponse {
            run: Some(run.into()),
        }))
    }

    async fn batch_get_runs(
        &self,
        request: Request<BatchGetRunsRequest>,
    ) -> Result<Response<BatchGetRunsResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.pipeline_id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        if args.ids.is_empty() {
            return Err(Status::failed_precondition("must include target run ids"));
        }

        let result = self
            .storage
            .batch_get_runs(&args.namespace_id, &args.pipeline_id, &args.ids)
            .await;

        match result {
            Ok(runs) => {
                return Ok(Response::new(BatchGetRunsResponse {
                    runs: runs.into_iter().map(gofer_proto::Run::from).collect(),
                }));
            }
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(Status::not_found(format!(
                        "run with id '{:?}' does not exist",
                        &args.ids
                    )))
                }
                _ => return Err(Status::internal(e.to_string())),
            },
        };
    }

    async fn list_runs(
        &self,
        request: Request<ListRunsRequest>,
    ) -> Result<Response<ListRunsResponse>, Status> {
        let args = &request.into_inner();

        if args.namespace_id.is_empty() {
            return Err(Status::failed_precondition("must include target namespace"));
        }

        if args.pipeline_id.is_empty() {
            return Err(Status::failed_precondition(
                "must include target pipeline id",
            ));
        }

        let result = self
            .storage
            .list_runs(
                args.offset as u64,
                args.limit as u64,
                &args.namespace_id,
                &args.pipeline_id,
            )
            .await;

        match result {
            Ok(runs) => {
                return Ok(Response::new(ListRunsResponse {
                    runs: runs.into_iter().map(gofer_proto::Run::from).collect(),
                }));
            }
            Err(storage_err) => return Err(Status::internal(storage_err.to_string())),
        }
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
}

impl Api {
    /// Create new API object. Subsequently you can run start_service to start the server.
    pub async fn new(conf: conf::api::Config) -> Self {
        let storage = storage::Db::new(&conf.server.storage_path).await.unwrap();

        let api = Api { conf, storage };

        api.create_default_namespace().await.unwrap();

        api
    }

    /// Gofer starts with a default namespace that all users have access to.
    async fn create_default_namespace(&self) -> Result<(), storage::StorageError> {
        const DEFAULT_NAMESPACE_ID: &str = "default";
        const DEFAULT_NAMESPACE_NAME: &str = "Default";
        const DEFAULT_NAMESPACE_DESCRIPTION: &str =
            "The default namespace when no other namespace is specified.";

        let default_namespace = models::Namespace::new(
            DEFAULT_NAMESPACE_ID,
            DEFAULT_NAMESPACE_NAME,
            DEFAULT_NAMESPACE_DESCRIPTION,
        );

        match self.storage.create_namespace(&default_namespace).await {
            Ok(_) => Ok(()),
            Err(e) => match e {
                storage::StorageError::Exists => Ok(()),
                _ => Err(e),
            },
        }
    }

    /// Start a TLS enabled, multiplexed, grpc/http server.
    pub async fn start_service(&self) {
        let rest =
            axum::Router::new().route("/*path", axum::routing::any(frontend::frontend_handler));
        let grpc = GoferServer::new(self.clone());

        let service = service::MultiplexService { rest, grpc };

        let cert = self.conf.server.tls_cert.clone().into_bytes();
        let key = self.conf.server.tls_key.clone().into_bytes();

        if self.conf.general.dev_mode {
            service::start_server(service, &self.conf.server.url).await;
            return;
        }

        service::start_tls_server(service, &self.conf.server.url, cert, key).await;
    }
}
