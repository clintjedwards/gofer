use crate::api::{fmt, validate, Api, GOFER_EOF};
use crate::{scheduler, storage};
use anyhow::Result;
use futures::Stream;
use gofer_models::task_run;
use gofer_proto::{
    CancelTaskRunRequest, CancelTaskRunResponse, DeleteTaskRunLogsRequest,
    DeleteTaskRunLogsResponse, GetTaskRunLogsRequest, GetTaskRunLogsResponse, GetTaskRunRequest,
    GetTaskRunResponse, ListTaskRunsRequest, ListTaskRunsResponse, TaskRun,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use slog_scope::error;
use std::sync::Arc;
use std::{path::Path, pin::Pin};
use tokio::sync::mpsc::{self, channel, Sender};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

type GetTaskRunLogsStream =
    Pin<Box<dyn Stream<Item = Result<GetTaskRunLogsResponse, Status>> + Send>>;

pub async fn stream_task_run_logs(
    path: String,
    input: Sender<Result<GetTaskRunLogsResponse, Status>>,
) {
    let file = match File::open(&path).await {
        Ok(file) => file,
        Err(e) => {
            error!("could not open log file; {:?}", e);
            return;
        }
    };
    let mut reader = BufReader::new(file);
    let mut line_num = 1;

    // Read the file until we're at the end or our buffer is empty.
    let mut empty_buffer = false;
    while !empty_buffer {
        let mut line = String::new();
        let resp = reader.read_line(&mut line).await.unwrap_or_default();

        // We have reached the EOF and there will be no more lines
        // to stream.
        if line == GOFER_EOF {
            return;
        }

        if input
            .send(Result::<GetTaskRunLogsResponse, Status>::Ok(
                GetTaskRunLogsResponse {
                    log_line: line,
                    line_num,
                },
            ))
            .await
            .is_err()
        {
            // If we have received an error the client must have hung up;
            // stop streaming lines.
            return;
        }
        line_num += 1;

        if resp == 0 {
            empty_buffer = true;
        }
    }

    // Set up an event stream that watches the file for any changes and
    // streams them back to the client.
    let (event_tx, mut event_rx) = channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |result: std::result::Result<notify::Event, notify::Error>| {
            event_tx.blocking_send(result).expect("failed");
        },
    )
    .unwrap();

    if let Err(e) = watcher.watch(Path::new(&path), RecursiveMode::NonRecursive) {
        error!("could not monitor log file for more writes; {:?}", e);
        return;
    }

    while event_rx.recv().await.is_some() {
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap_or_default();
        // We have reached the EOF and there will be no more lines
        // to stream.
        if line == GOFER_EOF {
            return;
        }

        if input
            .send(Result::<GetTaskRunLogsResponse, Status>::Ok(
                GetTaskRunLogsResponse {
                    log_line: line,
                    line_num,
                },
            ))
            .await
            .is_err()
        {
            // If we have received an error the client must have hung up;
            // stop streaming lines.
            return;
        }
        line_num += 1;
    }
}

impl Api {
    /// Calls upon the scheduler to terminate a specific container.
    pub async fn cancel_task_run(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
        run_id: u64,
        task_run_id: &str,
        timeout: u64,
    ) -> Result<()> {
        self.scheduler
            .stop_container(scheduler::StopContainerRequest {
                name: fmt::task_container_id(namespace_id, pipeline_id, run_id, task_run_id),
                timeout: timeout as i64,
            })
            .await?;

        Ok(())
    }

    pub async fn list_task_runs_handler(
        &self,
        args: ListTaskRunsRequest,
    ) -> Result<Response<ListTaskRunsResponse>, Status> {
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

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::task_runs::list(
            &mut conn,
            0,
            0,
            &args.namespace_id,
            &args.pipeline_id,
            args.run_id,
        )
        .await
        .map(|task_runs| {
            Response::new(ListTaskRunsResponse {
                task_runs: task_runs.into_iter().map(TaskRun::from).collect(),
            })
        })
        .map_err(|e| Status::internal(e.to_string()))
    }

    pub async fn get_task_run_handler(
        &self,
        args: GetTaskRunRequest,
    ) -> Result<Response<GetTaskRunResponse>, Status> {
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

        validate::arg(
            "id",
            args.id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::task_runs::get(
            &mut conn,
            &args.namespace_id,
            &args.pipeline_id,
            args.run_id,
            &args.id,
        )
        .await
        .map(|task_run| {
            Response::new(GetTaskRunResponse {
                task_run: Some(task_run.into()),
            })
        })
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("task_run with id '{}' does not exist", &args.id))
            }
            _ => Status::internal(e.to_string()),
        })
    }

    pub async fn cancel_task_run_handler(
        &self,
        args: CancelTaskRunRequest,
    ) -> Result<Response<CancelTaskRunResponse>, Status> {
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

        validate::arg(
            "id",
            args.id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut timeout = self.conf.general.task_run_stop_timeout;
        if args.force {
            timeout = 0;
        }

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        storage::task_runs::get(
            &mut conn,
            &args.namespace_id,
            &args.pipeline_id,
            args.run_id,
            &args.id,
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("task_run with id '{}' does not exist", &args.id))
            }
            _ => Status::internal(e.to_string()),
        })?;

        if let Err(e) = self
            .cancel_task_run(
                &args.namespace_id,
                &args.pipeline_id,
                args.run_id,
                &args.id,
                timeout,
            )
            .await
        {
            return Err(Status::internal(e.to_string()));
        }

        Ok(Response::new(CancelTaskRunResponse {}))
    }

    pub async fn get_task_run_logs_handler(
        self: Arc<Self>,
        args: GetTaskRunLogsRequest,
    ) -> Result<Response<GetTaskRunLogsStream>, Status> {
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

        validate::arg(
            "id",
            args.id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let task_run = storage::task_runs::get(
            &mut conn,
            &args.namespace_id,
            &args.pipeline_id,
            args.run_id,
            &args.id,
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("task_run with id '{}' does not exist", &args.id))
            }
            _ => Status::internal(e.to_string()),
        })?;

        if task_run.logs_expired {
            return Err(Status::failed_precondition(
                "task run logs have expired and are no longer available",
            ));
        }

        if task_run.logs_removed {
            return Err(Status::failed_precondition(
                "task run logs have been removed and are no longer available",
            ));
        }

        // We create a channel we will eventually turn into
        // a stream we can use and pass back to the client.
        let (tx, rx) = mpsc::channel(128);
        let output_stream = ReceiverStream::new(rx);
        let log_path = fmt::task_run_log_path(&self.conf.general.task_run_logs_dir, &task_run);

        tokio::spawn(async move { stream_task_run_logs(log_path, tx).await });

        Ok(Response::new(Box::pin(output_stream)))
    }

    pub async fn delete_task_run_logs_handler(
        &self,
        args: DeleteTaskRunLogsRequest,
    ) -> Result<Response<DeleteTaskRunLogsResponse>, Status> {
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

        validate::arg(
            "id",
            args.id.clone(),
            vec![validate::is_valid_identifier, validate::not_empty_str],
        )?;

        let mut conn = self
            .storage
            .conn()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let task_run = storage::task_runs::get(
            &mut conn,
            &args.namespace_id,
            &args.pipeline_id,
            args.run_id,
            &args.id,
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => {
                Status::not_found(format!("task_run with id '{}' does not exist", &args.id))
            }
            _ => Status::internal(e.to_string()),
        })?;

        if task_run.state != task_run::State::Complete {
            return Err(Status::failed_precondition(
                "could not delete logs for a task run currently in progress",
            ));
        }

        if task_run.logs_expired || task_run.logs_removed {
            return Ok(Response::new(DeleteTaskRunLogsResponse {}));
        }

        if let Err(e) = tokio::fs::remove_file(fmt::task_run_log_path(
            &self.conf.general.task_run_logs_dir,
            &task_run,
        ))
        .await
        {
            return Err(Status::internal(format!(
                "error attempting to delete the file: {:?}",
                e
            )));
        };

        storage::task_runs::update(
            &mut conn,
            &task_run,
            storage::task_runs::UpdatableFields {
                logs_removed: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| match e {
            storage::StorageError::NotFound => Status::not_found(format!(
                "task_run with id '{}' does not exist",
                &task_run.id
            )),
            _ => Status::internal(e.to_string()),
        })?;

        Ok(Response::new(DeleteTaskRunLogsResponse {}))
    }
}
