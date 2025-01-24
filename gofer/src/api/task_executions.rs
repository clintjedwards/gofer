use super::permissioning::{Action, Resource};
use crate::{
    api::{
        epoch_milli,
        event_utils::{self, EventListener},
        format_duration, listen_for_terminate_signal, tasks, websocket_error, ApiState,
        PreflightOptions, Variable, GOFER_EOF,
    },
    http_error, scheduler, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    channel, endpoint, HttpError, HttpResponseDeleted, HttpResponseOk, Path, Query, RequestContext,
    WebsocketChannelResult, WebsocketConnection,
};
use futures::{SinkExt, StreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr, sync::Arc};
use strum::{Display, EnumString};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    sync::Mutex,
};
use tokio_tungstenite::tungstenite::{protocol::Role, Message};
use tracing::{debug, error};
use tungstenite::protocol::frame::coding::CloseCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TaskExecutionPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TaskExecutionPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,

    /// The unique identifier for the target task execution.
    pub task_id: String,
}

/// Correctly formats the task execution container_id. This is passed to the container orchestrator to uniquely identify
/// the referenced container. Because namespaces, pipelines, and task_executions ids support hypen only, the result of
/// this will be a mix between underscores (which designate a different part of the name and) and hypens (which are
/// just parts of the ID). This distinct naming scheme should actually be helpful since it gives any parsers a good
/// way to seperate different parts of the name.
pub fn task_execution_container_id(
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    task_execution_id: &str,
) -> String {
    format!("{namespace_id}_{pipeline_id}_{run_id}_{task_execution_id}")
}

pub fn task_execution_log_path(
    dir: &str,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    task_id: &str,
) -> PathBuf {
    let mut path = PathBuf::new();
    path.push(dir);
    path.push(format!(
        "{namespace_id}_{pipeline_id}_{run_id}_{task_id}.log"
    ));

    path
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "task_execution_state")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum State {
    /// Should never be in this state.
    #[default]
    Unknown,

    /// Pre-scheduler validation and prep.
    Processing,

    /// Waiting to be scheduled.
    Waiting,

    /// Currently running as reported by scheduler.
    Running,

    Complete,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "task_execution_status")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum Status {
    #[default]
    Unknown,

    /// Has encountered an issue, either container issue or scheduling issue.
    Failed,

    /// Finished with a proper exit code.
    Successful,

    /// Cancelled mid run due to user requested cancellation.
    Cancelled,

    /// Not run due to dependencies not being met.
    Skipped,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "task_execution_status_reason_type")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum StatusReasonType {
    /// Gofer has no fucking clue how the run got into this state.
    #[default]
    Unknown,

    /// A non-zero exit code has been received.
    AbnormalExit,

    /// Encountered an error with the container scheduler.
    SchedulerError,

    /// User error in task execution parameters.
    FailedPrecondition,

    /// User invoked cancellation.k
    Cancelled,

    /// Task execution was lost due to extreme internal error.
    Orphaned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "task_execution_status_reason")]
pub struct StatusReason {
    /// The specific type of task execution failure.
    pub reason: StatusReasonType,

    /// A description of why the task execution might have failed and what was going on at the time.
    pub description: String,
}

/// a task execution is a specific execution of a task/container. It represents a 4th level unit in the hierarchy.
/// namespace -> pipeline -> run -> task execution. It is the last and most specific object in Gofer's execution model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct TaskExecution {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Unique identifier of the target run.
    pub run_id: u64,

    /// Unique identifier of the current task being executed.
    pub task_id: String,

    /// Time of task execution creation in epoch milliseconds.
    pub created: u64,

    /// Time of task execution start in epoch milliseconds.
    pub started: u64,

    /// Time of task execution end in epoch milliseconds.
    pub ended: u64,

    /// The exit code of the task execution completion if it is finished.
    pub exit_code: Option<u8>,

    /// Whether the logs have past their retention time.
    pub logs_expired: bool,

    /// If the logs for this execution have been removed.
    /// This can be due to user request or automatic action based on expiry time.
    pub logs_removed: bool,

    /// The current state of the task execution within the Gofer execution model.
    /// Describes if the execution is in progress or not.
    pub state: State,

    /// The final result of the task execution.
    pub status: Status,

    /// More information on the circumstances around a particular task execution's status.
    pub status_reason: Option<StatusReason>,

    /// The environment variables injected during this particular task execution.
    pub variables: Vec<Variable>,

    /// Information about the underlying task this task execution ran.
    pub task: tasks::Task,
}

impl TaskExecution {
    pub fn new(namespace_id: &str, pipeline_id: &str, run_id: u64, task: tasks::Task) -> Self {
        TaskExecution {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            run_id,
            task_id: task.id.clone(),
            created: epoch_milli(),
            started: 0,
            ended: 0,
            exit_code: None,
            logs_expired: false,
            logs_removed: false,
            state: State::Processing,
            status: Status::Unknown,
            status_reason: None,
            variables: vec![],
            task,
        }
    }
}

impl TryFrom<storage::task_executions::TaskExecution> for TaskExecution {
    type Error = anyhow::Error;

    fn try_from(value: storage::task_executions::TaskExecution) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let started = value.started.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'started' from storage value '{}'",
                value.started
            )
        })?;

        let ended = value.ended.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'ended' from storage value '{}'",
                value.ended
            )
        })?;

        let state = State::from_str(&value.state).with_context(|| {
            format!(
                "Could not parse field 'state' from storage value '{}'",
                value.state
            )
        })?;

        let status = Status::from_str(&value.status).with_context(|| {
            format!(
                "Could not parse field 'status' from storage value '{}'",
                value.state
            )
        })?;

        let status_reason = serde_json::from_str(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        let variables = serde_json::from_str(&value.variables).with_context(|| {
            format!(
                "Could not parse field 'variables' from storage value; '{:#?}'",
                value.variables
            )
        })?;

        let task = serde_json::from_str(&value.task).with_context(|| {
            format!(
                "Could not parse field 'task' from storage value; '{:#?}'",
                value.task
            )
        })?;

        let exit_code = value.exit_code.and_then(|value| match u8::try_from(value) {
            Ok(v) => Some(v),
            Err(e) => {
                debug!(
                    value = value,
                    error = %e,
                    "Could not parse field 'exit code' from storage value; Defaulting to None type"
                );
                None
            }
        });

        Ok(TaskExecution {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            run_id: value.run_id.try_into()?,
            task_id: value.task_id,
            created,
            started,
            ended,
            exit_code,
            logs_expired: value.logs_expired,
            logs_removed: value.logs_removed,
            state,
            status,
            status_reason,
            variables,
            task,
        })
    }
}

impl TryFrom<TaskExecution> for storage::task_executions::TaskExecution {
    type Error = anyhow::Error;

    fn try_from(value: TaskExecution) -> Result<Self> {
        let status_reason = serde_json::to_string(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' to storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        let task = serde_json::to_string(&value.task).with_context(|| {
            format!(
                "Could not parse field 'task' to storage value; '{:#?}'",
                value.task
            )
        })?;

        let variables = serde_json::to_string(&value.variables).with_context(|| {
            format!(
                "Could not parse field 'variables' to storage value; '{:#?}'",
                value.variables
            )
        })?;

        let exit_code = value.exit_code.map(i64::from);

        Ok(Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            run_id: value.run_id.try_into()?,
            task_id: value.task_id,
            created: value.created.to_string(),
            started: value.started.to_string(),
            ended: value.ended.to_string(),
            exit_code,
            logs_expired: value.logs_expired,
            logs_removed: value.logs_removed,
            state: value.state.to_string(),
            status: value.status.to_string(),
            status_reason,
            variables,
            task,
        })
    }
}

impl dyn scheduler::Scheduler {
    pub async fn cancel_task_execution(
        &self,
        execution: TaskExecution,
        timeout: i64,
    ) -> Result<()> {
        let container_id = task_execution_container_id(
            &execution.namespace_id,
            &execution.pipeline_id,
            execution.run_id,
            &execution.task_id,
        );

        self.stop_container(scheduler::StopContainerRequest {
            id: container_id.clone(),
            timeout,
        })
        .await
        .with_context(|| format!("Could not stop container while attempting to cancel task execution; container_id = {}",
                container_id))?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListTaskExecutionsResponse {
    /// A list of all task executions.
    pub task_executions: Vec<TaskExecution>,
}

/// List all task executions.
///
/// Returns a list of all task executions by run.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks",
    tags = ["Tasks"],
)]
pub async fn list_task_executions(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TaskExecutionPathArgsRoot>,
) -> Result<HttpResponseOk<ListTaskExecutionsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::TaskExecutions,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let run_id = path.run_id.try_into().map_err(|err| {
        HttpError::for_bad_request(
            None,
            format!("Could not successfully parse 'run_id'. Must be a positive integer; {err}"),
        )
    })?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let storage_task_executions = match storage::task_executions::list(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
    )
    .await
    {
        Ok(task_executions) => task_executions,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut task_executions: Vec<TaskExecution> = vec![];

    for storage_task_execution in storage_task_executions {
        let task_execution = TaskExecution::try_from(storage_task_execution).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        task_executions.push(task_execution);
    }

    let resp = ListTaskExecutionsResponse { task_executions };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskExecutionResponse {
    /// The task execution requested.
    pub task_execution: TaskExecution,
}

/// Get task execution by id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}",
    tags = ["Tasks"],
)]
pub async fn get_task_execution(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TaskExecutionPathArgs>,
) -> Result<HttpResponseOk<GetTaskExecutionResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::TaskExecutions,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let run_id = path.run_id.try_into().map_err(|err| {
        HttpError::for_bad_request(
            None,
            format!("Could not successfully parse 'run_id'. Must be a positive integer; {err}"),
        )
    })?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_task_execution = match storage::task_executions::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
        &path.task_id,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let task_execution = TaskExecution::try_from(storage_task_execution).map_err(|err| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    let resp = GetTaskExecutionResponse { task_execution };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CancelTaskExecutionQueryArgs {
    /// Period of time to wait the task before forcing it to cancel. 0 means send SIGKILL instantly.
    pub wait_for: u64,
}

/// Cancel a task execution by id.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}",
    tags = ["Tasks"],
)]
pub async fn cancel_task_execution(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TaskExecutionPathArgs>,
    query_params: Query<CancelTaskExecutionQueryArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::TaskExecutions,
                ],
                action: Action::Delete,
            },
        )
        .await?;

    let run_id = path.run_id.try_into().map_err(|err| {
        HttpError::for_bad_request(
            None,
            format!("Could not successfully parse 'run_id'. Must be a positive integer; {err}"),
        )
    })?;

    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::task_executions::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
        &path.task_id,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get task execution from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::StartedTaskExecutionCancellation {
            namespace_id: path.namespace_id,
            pipeline_id: path.pipeline_id,
            run_id: run_id as u64,
            task_execution_id: path.task_id,
            timeout: query.wait_for,
        });

    Ok(HttpResponseDeleted())
}

/// Retrieves logs from a task execution.
#[channel(
    protocol = WEBSOCKETS,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs",
    tags = ["Tasks"],
)]
pub async fn get_logs(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TaskExecutionPathArgs>,
    conn: WebsocketConnection,
) -> WebsocketChannelResult {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::TaskExecutions,
                ],
                action: Action::Read,
            },
        )
        .await?;

    let start_time = std::time::Instant::now();

    let ws =
        tokio_tungstenite::WebSocketStream::from_raw_socket(conn.into_inner(), Role::Server, None)
            .await;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(websocket_error(
                "Could not open connection to database",
                CloseCode::Error,
                rqctx.request_id.clone(),
                ws,
                Some(e.to_string()),
            )
            .await
            .into());
        }
    };

    let run_id = match path.run_id.try_into() {
        Ok(run_id) => run_id,
        Err(err) => {
            return Err(websocket_error(
                "Could not successfully parse 'run_id'. Must be a positive integer.",
                CloseCode::Policy,
                rqctx.request_id.clone(),
                ws,
                Some(format!("{}", err)),
            )
            .await
            .into());
        }
    };

    let storage_task_execution = match storage::task_executions::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
        &path.task_id,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(websocket_error(
                    "Could not get task execution from database; Not found",
                    CloseCode::Away,
                    rqctx.request_id.clone(),
                    ws,
                    Some(e.to_string()),
                )
                .await
                .into());
            }
            _ => {
                return Err(websocket_error(
                    "Could not get task execution from database",
                    CloseCode::Error,
                    rqctx.request_id.clone(),
                    ws,
                    Some(e.to_string()),
                )
                .await
                .into());
            }
        },
    };

    if storage_task_execution.logs_expired {
        return Err(websocket_error(
            "Could not retrieve logs; logs expired",
            CloseCode::Policy,
            rqctx.request_id.clone(),
            ws,
            None,
        )
        .await
        .into());
    }

    if storage_task_execution.logs_removed {
        return Err(websocket_error(
            "Could not retrieve logs; logs removed",
            CloseCode::Policy,
            rqctx.request_id.clone(),
            ws,
            None,
        )
        .await
        .into());
    }

    // We stream from the log file so that we can show the full logs to the user and stream it.
    // We we read in the GOFER_EOF string that is a sign to stop processing as we've reached the end of the file.

    let task_execution_log_path = task_execution_log_path(
        &api_state.config.api.task_execution_logs_dir,
        &path.namespace_id,
        &path.pipeline_id,
        path.run_id,
        &path.task_id,
    );

    let file = match tokio::fs::File::open(task_execution_log_path).await {
        Ok(file) => file,
        Err(err) => {
            return Err(websocket_error(
                "Could not open task execution log file to stream contents",
                CloseCode::Policy,
                rqctx.request_id.clone(),
                ws,
                Some(err.to_string()),
            )
            .await
            .into());
        }
    };

    // We need to launch two async functions to:
    // * Push logs to the user.
    // * Listen for the user closing the connection.
    // * Listen for a terminal signal from the main process.
    //
    // The JoinSet below allows us to launch all of the functions and then
    // wait for one of them to return. Since all need to be running
    // or they are all basically useless, we wait for any one of them to finish
    // and then we simply abort the others and then close the stream.

    let mut set: tokio::task::JoinSet<std::result::Result<(), String>> =
        tokio::task::JoinSet::new();

    let (client_write, mut client_read) = ws.split();
    let client_writer = Arc::new(Mutex::new(client_write));
    let client_writer_handle = client_writer.clone();

    // Listen for a terminal signal from the main process.
    set.spawn(async move {
        listen_for_terminate_signal().await;
        Err("Server is shutting down".into())
    });

    set.spawn(async move {
        let mut reader = tokio::io::BufReader::new(file);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer).await {
                Ok(0) => {
                    // No more data to read, wait for a bit and try again
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                Ok(_) => match String::from_utf8(buffer.clone()) {
                    Ok(line) => {
                        if line.contains(GOFER_EOF) {
                            break;
                        };

                        let mut locked_writer = client_writer_handle.lock().await;

                        if let Err(err) = locked_writer.send(Message::text(line)).await {
                            error!(error = %err,"Could not process log line");
                            return Err("Could not process log line".into());
                        }
                    }
                    Err(err) => {
                        error!(error = %err,"Non UTF-8 data encountered in log file");
                        return Err("Non UTF-8 data encountered in log file".into());
                    }
                },
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }

        Ok(())
    });

    set.spawn(async move {
        loop {
            if let Some(output) = client_read.next().await {
                match output {
                    Ok(message) => match message {
                        tungstenite::protocol::Message::Close(_) => {
                            break;
                        }
                        _ => {
                            continue;
                        }
                    },
                    Err(_) => {
                        break;
                    }
                }
            }
        }

        Ok(())
    });

    // The first one to finish will return here. We can unwrap the option safely because it only returns a None if there
    // was nothing in the set.
    let result = set.join_next().await.unwrap()?;
    if let Err(err) = result {
        let mut locked_writer = client_writer.lock().await;

        let close_message = Message::Close(Some(tungstenite::protocol::CloseFrame {
            code: tungstenite::protocol::frame::coding::CloseCode::Error,
            reason: err.clone().into(),
        }));

        let _ = locked_writer.send(close_message).await;
        let _ = locked_writer.close().await;
        return Err(err.into());
    }

    set.shutdown().await; // When one finishes we no longer have use for the others, make sure they all shutdown.

    let mut locked_writer = client_writer.lock().await;

    let close_message = Message::Close(Some(tungstenite::protocol::CloseFrame {
        code: tungstenite::protocol::frame::coding::CloseCode::Normal,
        reason: "out of events".into(),
    }));

    let _ = locked_writer.send(close_message).await;
    let _ = locked_writer.close().await;

    debug!(
        duration = format_duration(start_time.elapsed()),
        request_id = rqctx.request_id.clone(),
        "Finished get_logs",
    );

    Ok(())
}

/// Removes a task execution's associated log object.
///
/// This is useful for if logs mistakenly contain sensitive data.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs",
    tags = ["Tasks"],
)]
pub async fn delete_logs(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TaskExecutionPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::TaskExecutions,
                ],
                action: Action::Delete,
            },
        )
        .await?;

    let run_id = path.run_id.try_into().map_err(|err| {
        HttpError::for_bad_request(
            None,
            format!("Could not successfully parse 'run_id'. Must be a positive integer; {err}"),
        )
    })?;

    let mut conn = match api_state.storage.write_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_task_execution = match storage::task_executions::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
        &path.task_id,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                error!(message = "Could not get task execution from database", error = %e);
                return Err(HttpError::for_internal_error(format!(
                    "Could not get task execution from database; {:#?}",
                    e
                )));
            }
        },
    };

    let task_execution = TaskExecution::try_from(storage_task_execution).map_err(|err| {
        error!(message = "Could not serialize task execution from storage", error = %err);
        HttpError::for_internal_error(format!(
            "Could not serialize task execution from storage; {:#?}",
            err
        ))
    })?;

    if task_execution.state != State::Complete {
        return Err(HttpError::for_bad_request(
            None,
            "Can not delete logs for a task current in progress".into(),
        ));
    };

    let log_file_path = task_execution_log_path(
        &api_state.config.api.task_execution_logs_dir.clone(),
        &task_execution.namespace_id,
        &task_execution.pipeline_id,
        task_execution.run_id,
        &task_execution.task_id,
    );

    tokio::fs::remove_file(log_file_path).await.map_err(|err| {
        error!(message = "Could not remove log file from filesystem", error = %err);
        HttpError::for_internal_error(format!(
            "Could not remove log file from filesystem; {:#?}",
            err
        ))
    })?;

    if let Err(e) = storage::task_executions::update(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
        &path.task_id,
        storage::task_executions::UpdatableFields {
            logs_removed: Some(true),
            ..Default::default()
        },
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "task execution for id given does not exist".into(),
                ));
            }
            _ => {
                error!(message = "Could not remove task execution from database", error = %e);
                return Err(HttpError::for_internal_error(
                    "Could not remove task execution from database".into(),
                ));
            }
        }
    };

    Ok(HttpResponseDeleted())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AttachTaskExecutionQueryParams {
    pub command: String,
}

/// Run command on a running task execution container.
///
/// This allows you to run a command on a task execution container and connect to the stdin and stdout/err for said
/// container.
///
/// Useful for debugging.
#[channel(
    protocol = WEBSOCKETS,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/attach",
    tags = ["Tasks"],
)]
pub async fn attach_task_execution(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TaskExecutionPathArgs>,
    query_params: Query<AttachTaskExecutionQueryParams>,
    socket_conn: WebsocketConnection,
) -> WebsocketChannelResult {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                allow_anonymous: false,
                bypass_auth: false,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                    Resource::TaskExecutions,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        socket_conn.into_inner(),
        Role::Server,
        None,
    )
    .await;

    let run_id = match path.run_id.try_into() {
        Ok(run_id) => run_id,
        Err(err) => {
            return Err(websocket_error(
                "Could not successfully parse 'run_id'. Must be a positive integer.",
                CloseCode::Policy,
                rqctx.request_id.clone(),
                ws,
                Some(format!("{}", err)),
            )
            .await
            .into());
        }
    };

    let container_id = task_execution_container_id(
        &path.namespace_id,
        &path.pipeline_id,
        path.run_id,
        &path.task_id,
    );

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(websocket_error(
                "Could not open connection to database",
                CloseCode::Error,
                rqctx.request_id.clone(),
                ws,
                Some(e.to_string()),
            )
            .await
            .into());
        }
    };

    let storage_task_execution = match storage::task_executions::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        run_id,
        &path.task_id,
    )
    .await
    {
        Ok(task_execution) => task_execution,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(Box::new(HttpError::for_not_found(None, String::new())));
            }
            _ => {
                return Err(websocket_error(
                    "Could not get object from database",
                    CloseCode::Error,
                    rqctx.request_id.clone(),
                    ws,
                    Some(e.to_string()),
                )
                .await
                .into());
            }
        },
    };

    let state = match State::from_str(&storage_task_execution.state) {
        Ok(state) => state,
        Err(e) => {
            return Err(websocket_error(
                "Could not get state from database",
                CloseCode::Error,
                rqctx.request_id.clone(),
                ws,
                Some(e.to_string()),
            )
            .await
            .into());
        }
    };

    if state != State::Running {
        return Err(websocket_error(
            "Task execution not in running state; could not connect",
            CloseCode::Policy,
            rqctx.request_id.clone(),
            ws,
            None,
        )
        .await
        .into());
    }

    let attach_response = match api_state
        .scheduler
        .attach_container(scheduler::AttachContainerRequest {
            id: container_id,
            command: query
                .command
                .split(' ')
                .map(|val| val.to_string())
                .collect(),
        })
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            return Err(websocket_error(
                "Could not attach to container; scheduler error",
                CloseCode::Error,
                rqctx.request_id.clone(),
                ws,
                Some(err.to_string()),
            )
            .await
            .into());
        }
    };

    let mut container_output = attach_response.output;
    let mut container_input = attach_response.input;

    let (client_write, mut client_read) = ws.split();
    let client_writer = Arc::new(Mutex::new(client_write));
    let client_writer_handle = client_writer.clone();

    // Below we launch the async functions that enable us to:
    // * Write from the user into the container.
    // * Collect container output and stream it back to the user.
    // * Monitor for the completion of the container.
    // * Listen for a terminal signal from the main process.
    //
    // The JoinSet below allows us to launch all of the functions and then
    // wait for one of them to return. Since all three need to be running
    // or they are all basically useless, we wait for any one of them to finish
    // and then we simply abort the others and then close the stream.

    let mut set = tokio::task::JoinSet::new();

    // Listen for a terminal signal from the main process.
    set.spawn(listen_for_terminate_signal());

    // Launch thread to collect messages from the user and write them to the container.
    set.spawn(async move {
        loop {
                if let Some(output) = client_read.next().await {
                    match output {
                        Ok(message) => {
                            match message {
                                tungstenite::protocol::Message::Text(mut text) => {

                                    // Carriage return is needed in the case that the user is communicating
                                    // with a terminal.
                                    text.push('\r');

                                    let result = container_input.write_all(text.as_bytes()).await;


                                    if let Err(e) = result {
                                        debug!(error = %e, "Error occurred while attempting to write message from client to container");
                                        continue;
                                    }
                                },
                                tungstenite::protocol::Message::Close(_) => {
                                    break;
                                },
                                _ => {
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            debug!(error = %e, "Error occurred while attempting to unpack message from client to container");
                            break;
                        }
                    }
                }
        }
    });

    // Launch thread to pass messages from the container back to the user.
    set.spawn(async move {
        loop {
            if let Some(output) = container_output.next().await {
                match output {
                    Ok(message)=> {
                        match message {
                            scheduler::Log::Unknown  => {continue},
                            scheduler::Log::Stdout(text) | scheduler::Log::Stderr(text) | scheduler::Log::Console(text) => {
                                let mut locked_write = client_writer_handle.lock().await;

                                if let Err(e) = locked_write.send(tungstenite::Message::Binary(text)).await {
                                    debug!(error = %e, "Error occurred while attempting to write message from container to client");
                                    continue;
                                }
                            },
                            scheduler::Log::Stdin(_) => {continue},
                        }
                    },
                    Err(e) => {
                        debug!(error = %e, "Error occurred while attempting to unpack message from container to client");
                        continue;
                    }
                }
            }
        }

    });

    // Launch thread to wait for the container to finish and clean up both the container write and container read threads.
    let mut event_receiver = api_state.event_bus.subscribe_live();

    set.spawn(async move {
        loop {
            if let Ok(event) = event_receiver.next().await {
                match &event.kind {
                    event_utils::Kind::CompletedTaskExecution {
                        namespace_id,
                        pipeline_id,
                        run_id,
                        task_execution_id,
                        ..
                    } => {
                        if *namespace_id != path.namespace_id.clone()
                            || *pipeline_id != path.pipeline_id.clone()
                            || *run_id != path.run_id
                            || *task_execution_id != path.task_id.clone()
                        {
                            continue;
                        }

                        break;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
    });

    set.join_next().await; // The first one to finish will return here.
    set.shutdown().await; // When one finishes we no longer have use for the others, make sure they all shutdown.

    // Close the websocket connection.
    let mut locked_write = client_writer.lock().await;
    let _ = locked_write.close().await;

    Ok(())
}
