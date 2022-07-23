mod cancel_run_handler;
mod start_run_handler;

use crate::api::{validate, Api};
use crate::storage;
use gofer_models::{Variable, VariableOwner, VariableSensitivity};
use gofer_proto::{
    CancelAllRunsRequest, CancelAllRunsResponse, CancelRunRequest, CancelRunResponse,
    GetRunRequest, GetRunResponse, ListRunsRequest, ListRunsResponse, RetryRunRequest,
    RetryRunResponse, Run, StartRunRequest,
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
    pub async fn get_run_handler(
        &self,
        args: GetRunRequest,
    ) -> Result<Response<GetRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

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

    pub async fn list_runs_handler(
        &self,
        args: ListRunsRequest,
    ) -> Result<Response<ListRunsResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::runs::list_runs(
            &mut conn,
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

    pub async fn retry_run_handler(
        self: Arc<Self>,
        args: RetryRunRequest,
    ) -> Result<Response<RetryRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("run_id", args.run_id, vec![validate::not_zero_num])?;

        let run = self
            .storage
            .get_run(&args.namespace_id, &args.pipeline_id, args.run_id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("run with id '{}' does not exist", &args.run_id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        let resp = self
            .start_run_handler(StartRunRequest {
                namespace_id: run.namespace,
                pipeline_id: run.pipeline,
                variables: run
                    .variables
                    .into_iter()
                    .map(|variable| (variable.key, variable.value))
                    .collect(),
            })
            .await?;

        Ok(Response::new(RetryRunResponse {
            run: Some(resp.into_inner().run.unwrap()),
        }))
    }
    pub async fn cancel_run_handler(
        &self,
        args: CancelRunRequest,
    ) -> Result<Response<CancelRunResponse>, Status> {
        validate::arg(
            "namespace_id",
            args.namespace_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg(
            "pipeline_id",
            args.pipeline_id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;
        validate::arg("run_id", args.run_id, vec![validate::not_zero_num])?;

        let run = self
            .storage
            .get_run(&args.namespace_id, &args.pipeline_id, args.run_id)
            .await
            .map_err(|e| match e {
                storage::StorageError::NotFound => {
                    Status::not_found(format!("run with id '{}' does not exist", &args.run_id))
                }
                _ => Status::internal(e.to_string()),
            })?;

        //TODO(clintjedwards): cancel run function
        //self.cancel_run();

        unimplemented!()
    }
    pub async fn cancel_all_runs() {}
}
