use crate::api::{validate, Api};
use crate::storage;
use gofer_proto::{GetRunRequest, GetRunResponse, ListRunsRequest, ListRunsResponse, Run};
use tonic::{Response, Status};

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
}
