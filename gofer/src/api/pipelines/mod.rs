use crate::api::{epoch, validate, Api};
use crate::storage;
use gofer_models::{event, pipeline};
use gofer_models::{Variable, VariableOwner, VariableSensitivity};
use gofer_proto::{
    CreatePipelineRequest, CreatePipelineResponse, DeletePipelineRequest, DeletePipelineResponse,
    DisablePipelineRequest, DisablePipelineResponse, EnablePipelineRequest, EnablePipelineResponse,
    GetPipelineRequest, GetPipelineResponse, ListPipelinesRequest, ListPipelinesResponse, Pipeline,
    UpdatePipelineRequest, UpdatePipelineResponse,
};
use std::{ops::Not, sync::Arc};
use tonic::{Response, Status};

impl Api {
    pub async fn list_pipelines_handler(
        &self,
        args: ListPipelinesRequest,
    ) -> Result<Response<ListPipelinesResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::list(
            &mut conn,
            args.offset as u64,
            args.limit as u64,
            &args.namespace_id,
        )
        .await
        .map(|pipelines| {
            Response::new(ListPipelinesResponse {
                pipelines: pipelines.into_iter().map(Pipeline::from).collect(),
            })
        })
        .map_err(|e| Status::internal(e.to_string()))
    }

    pub async fn create_pipeline_handler(
        self: Arc<Self>,
        args: CreatePipelineRequest,
    ) -> Result<Response<CreatePipelineResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let pipeline_config = match &args.pipeline_config {
            Some(config) => config,
            None => {
                return Err(Status::failed_precondition(
                    "must include valid pipeline config",
                ));
            }
        };

        let new_pipeline =
            pipeline::Pipeline::new(&args.namespace_id, pipeline_config.to_owned().into());

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::insert(&mut conn, &new_pipeline)
            .await
            .map_err(|e| match e {
                storage::StorageError::Exists => Status::already_exists(format!(
                    "pipeline with id '{}' already exists",
                    new_pipeline.id
                )),
                _ => Status::internal(e.to_string()),
            })?;

        let namespace_id = new_pipeline.namespace.clone();
        let pipeline_id = new_pipeline.id.clone();

        tokio::spawn(async move {
            self.event_bus
                .publish(event::Kind::CreatedPipeline {
                    namespace_id,
                    pipeline_id,
                })
                .await;
        });

        Ok(Response::new(CreatePipelineResponse {
            pipeline: Some(new_pipeline.into()),
        }))
    }

    pub async fn get_pipeline_handler(
        &self,
        args: GetPipelineRequest,
    ) -> Result<Response<GetPipelineResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::get(&mut conn, &args.namespace_id, &args.id)
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

    pub async fn enable_pipeline_handler(
        self: Arc<Self>,
        args: EnablePipelineRequest,
    ) -> Result<Response<EnablePipelineResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::update(
            &mut conn,
            &args.namespace_id,
            &args.id,
            storage::pipelines::UpdatableFields {
                state: Some(pipeline::State::Active),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
            }
            _ => Status::internal(e.to_string()),
        })?;

        tokio::spawn(async move {
            self.event_bus
                .publish(event::Kind::EnabledPipeline {
                    namespace_id: args.namespace_id.clone(),
                    pipeline_id: args.id.clone(),
                })
                .await;
        });

        Ok(Response::new(EnablePipelineResponse {}))
    }

    pub async fn disable_pipeline_handler(
        self: Arc<Self>,
        args: DisablePipelineRequest,
    ) -> Result<Response<DisablePipelineResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::update(
            &mut conn,
            &args.namespace_id,
            &args.id,
            storage::pipelines::UpdatableFields {
                state: Some(pipeline::State::Disabled),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
            }
            _ => Status::internal(e.to_string()),
        })?;

        tokio::spawn(async move {
            self.event_bus
                .publish(event::Kind::DeletedPipeline {
                    namespace_id: args.namespace_id.clone(),
                    pipeline_id: args.id.clone(),
                })
                .await;
        });

        Ok(Response::new(DisablePipelineResponse {}))
    }

    pub async fn update_pipeline_handler(
        &self,
        args: UpdatePipelineRequest,
    ) -> Result<Response<UpdatePipelineResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let pipeline_config = match &args.pipeline_config {
            Some(config) => config,
            None => {
                return Err(Status::failed_precondition(
                    "must include valid pipeline config",
                ));
            }
        };

        let new_pipeline =
            pipeline::Pipeline::new(&args.namespace_id, pipeline_config.to_owned().into());

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::update(
            &mut conn,
            &args.namespace_id,
            &new_pipeline.id,
            storage::pipelines::UpdatableFields {
                name: new_pipeline
                    .name
                    .is_empty()
                    .not()
                    .then(|| new_pipeline.name.clone()),
                description: new_pipeline
                    .description
                    .is_empty()
                    .not()
                    .then(|| new_pipeline.description.clone()),
                parallelism: new_pipeline
                    .parallelism
                    .eq(&0)
                    .not()
                    .then(|| new_pipeline.parallelism),
                modified: Some(epoch()),
                ..Default::default()
            },
        )
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

    pub async fn delete_pipeline_handler(
        self: Arc<Self>,
        args: DeletePipelineRequest,
    ) -> Result<Response<DeletePipelineResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("id", args.id.clone(), vec![validate::is_valid_identifier])?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::pipelines::get(&mut conn, &args.namespace_id, &args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        storage::pipelines::delete(&mut conn, &args.namespace_id, &args.id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("pipeline with id '{}' does not exist", &args.id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        tokio::spawn(async move {
            self.event_bus
                .publish(event::Kind::DeletedPipeline {
                    namespace_id: args.namespace_id.clone(),
                    pipeline_id: args.id.clone(),
                })
                .await;
        });

        Ok(Response::new(DeletePipelineResponse {}))
    }
}
