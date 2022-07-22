use crate::api::{validate, Api};
use crate::storage;
use gofer_models::{event, pipeline};
use gofer_models::{Variable, VariableOwner, VariableSensitivity};
use gofer_proto::{
    CreatePipelineRequest, CreatePipelineResponse, DeletePipelineRequest, DeletePipelineResponse,
    DisablePipelineRequest, DisablePipelineResponse, EnablePipelineRequest, EnablePipelineResponse,
    GetPipelineRequest, GetPipelineResponse, ListPipelinesRequest, ListPipelinesResponse, Pipeline,
    UpdatePipelineRequest, UpdatePipelineResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Response, Status};

/// Converts a HashMap of variables(usually supplied by the user) into a Vec of type Variable that
/// can be used throughout Gofer.
pub fn variables_to_vec(
    map: HashMap<String, String>,
    owner: VariableOwner,
    sensitivity: VariableSensitivity,
) -> Vec<Variable> {
    let mut variables = vec![];

    for (key, value) in map {
        variables.push(Variable {
            key,
            value,
            owner: owner.clone(),
            sensitivity: sensitivity.clone(),
        })
    }

    variables
}

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

        self.storage
            .update_pipeline_state(&args.namespace_id, &args.id, pipeline::State::Active)
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

        self.storage
            .update_pipeline_state(&args.namespace_id, &args.id, pipeline::State::Disabled)
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

        self.storage
            .delete_pipeline(&args.namespace_id, &args.id)
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
