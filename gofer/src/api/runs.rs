use super::permissioning::{Action, Resource};
use crate::{
    api::{
        epoch_milli, event_utils, orchestrator::OrchestratorError, ApiState, PreflightOptions,
        Variable,
    },
    http_error, storage,
};

use anyhow::{Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk, Path, Query,
    RequestContext, TypedBody,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use strum::{Display, EnumString};
use tracing::error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,
}

/// The current state of the run. The state is described as the progress of the run towards completion.
#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "run_state")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum State {
    #[default]
    Unknown,

    /// Before the tasks in a run are sent to the scheduler it must complete various steps like validation checking.
    /// This state represents that step where the run and task executions are pre-checked.
    Pending,

    /// Currently running.
    Running,

    /// All tasks have been resolved and the run is no longer being executed.
    Complete,
}

/// The current status of the run. Status is described as if the run succeeded or not.
#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "run_status")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum Status {
    /// Could not determine the current state of the status. Should only be in this state
    /// if the run has not yet completed.
    #[default]
    Unknown,

    /// One or more tasks in run have failed.
    Failed,

    /// All tasks in a run have completed with a non-failure state.
    Successful,

    /// One or more tasks in a run have been cancelled.
    Cancelled,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "run_status_reason_type")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum StatusReasonType {
    /// Gofer has no fucking clue how the run got into this state.
    #[default]
    Unknown,

    /// While executing the run, one or more tasks exited with an abnormal exit code.
    AbnormalExit,

    /// While executing the run, one or more tasks returned errors from the scheduler or could not be scheduled.
    SchedulerError,

    /// The run could not be executed as requested due to user defined attributes given.
    FailedPrecondition,

    /// One or more tasks could not be completed due to a user cancelling the run.
    UserCancelled,

    /// One or more tasks could not be completed due to the system or admin cancelling the run.
    AdminCancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "run_status_reason")]
pub struct StatusReason {
    /// The specific type of run failure.
    pub reason: StatusReasonType,

    /// A description of why the run might have failed and what was going on at the time.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Initiator {
    /// The unique identifier for the token that initiated the request.
    pub id: String,

    /// The plaintext username for of the token.
    pub user: String,
}

/// A run is one or more tasks being executed on behalf of some extension.
/// Run is a third level unit containing tasks and being contained in a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Run {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Which version of the pipeline did this run execute.
    pub pipeline_config_version: u64,

    /// Unique identifier of the target run.
    pub run_id: u64,

    /// Time of run start in epoch milliseconds.
    pub started: u64,

    /// Time of run end in epoch milliseconds.
    pub ended: u64,

    /// The current state of the run within the Gofer execution model. Describes if the run is in progress or not.
    pub state: State,

    /// The final result of the run.
    pub status: Status,

    /// More information on the circumstances around a particular run's status.
    pub status_reason: Option<StatusReason>,

    /// Information about what started the run.
    pub initiator: Initiator,

    /// Run level environment variables to be passed to each task execution.
    pub variables: Vec<Variable>,

    /// The unique identifier for Gofer's auto-inject token. This feature is so that users can easily use Gofer's API
    /// with a ready injected token into the run just-in-time. If this is None this run had no tasks with the
    /// `inject_api_token` setting enabled.
    ///
    /// These tokens automatically expire after a pre-determined time.
    pub token_id: Option<String>,

    /// Whether run level objects are deleted.
    pub store_objects_expired: bool,

    /// The UUID of the QueuedRun event for this run. Essentially pointing to the start of the run in the event stream.
    ///
    /// This is used internally to help with run recovery.
    pub event_id: Option<String>,
}

impl Run {
    pub fn new(
        namespace_id: &str,
        pipeline_id: &str,
        version: u64,
        run_id: u64,
        initiator: Initiator,
        variables: Vec<Variable>,
        token_id: Option<String>,
    ) -> Self {
        Run {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            pipeline_config_version: version,
            run_id,
            started: epoch_milli(),
            ended: 0,
            state: State::Pending,
            status: Status::Unknown,
            status_reason: None,
            initiator,
            variables,
            token_id,
            store_objects_expired: false,
            event_id: None,
        }
    }
}

impl TryFrom<storage::runs::Run> for Run {
    type Error = anyhow::Error;

    fn try_from(value: storage::runs::Run) -> Result<Self> {
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

        let initiator = serde_json::from_str(&value.initiator).with_context(|| {
            format!(
                "Could not parse field 'initiator' from storage value; '{:#?}'",
                value.initiator
            )
        })?;

        let variables = serde_json::from_str(&value.variables).with_context(|| {
            format!(
                "Could not parse field 'variables' from storage value; '{:#?}'",
                value.variables
            )
        })?;

        Ok(Run {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            pipeline_config_version: value.pipeline_config_version.try_into()?,
            run_id: value.run_id.try_into()?,
            started,
            ended,
            state,
            status,
            status_reason,
            initiator,
            variables,
            token_id: value.token_id,
            store_objects_expired: value.store_objects_expired,
            event_id: value.event_id,
        })
    }
}

impl TryFrom<Run> for storage::runs::Run {
    type Error = anyhow::Error;

    fn try_from(value: Run) -> Result<Self> {
        let status_reason = serde_json::to_string(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' to storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        let initiator = serde_json::to_string(&value.initiator).with_context(|| {
            format!(
                "Could not parse field 'initiator' to storage value; '{:#?}'",
                value.initiator
            )
        })?;

        let variables = serde_json::to_string(&value.variables).with_context(|| {
            format!(
                "Could not parse field 'variables' to storage value; '{:#?}'",
                value.variables
            )
        })?;

        Ok(Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            pipeline_config_version: value.pipeline_config_version.try_into()?,
            run_id: value.run_id.try_into()?,
            started: value.started.to_string(),
            ended: value.ended.to_string(),
            state: value.state.to_string(),
            status: value.status.to_string(),
            status_reason,
            initiator,
            variables,
            token_id: value.token_id,
            store_objects_expired: value.store_objects_expired,
            event_id: value.event_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListRunsResponse {
    /// A list of all runs.
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListRunsQueryArgs {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub reverse: Option<bool>,
}

/// List all runs.
///
/// Returns a list of all runs by pipeline id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs",
    tags = ["Runs"],
)]
pub async fn list_runs(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunPathArgsRoot>,
    query_params: Query<ListRunsQueryArgs>,
) -> Result<HttpResponseOk<ListRunsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: true,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                ],
                action: Action::Read,
            },
        )
        .await?;

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

    let storage_runs = match storage::runs::list(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        query.offset.unwrap_or_default() as i64,
        query.limit.unwrap_or(50) as i64,
        query.reverse.unwrap_or_default(),
    )
    .await
    {
        Ok(runs) => runs,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut runs: Vec<Run> = vec![];

    for storage_run in storage_runs {
        let run = Run::try_from(storage_run).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        runs.push(run);
    }

    let resp = ListRunsResponse { runs };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetRunResponse {
    /// The run requested.
    pub run: Run,
}

/// Get run by id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}",
    tags = ["Runs"],
)]
pub async fn get_run(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunPathArgs>,
) -> Result<HttpResponseOk<GetRunResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: true,
                admin_only: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
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

    let storage_runs =
        match storage::runs::get(&mut conn, &path.namespace_id, &path.pipeline_id, run_id).await {
            Ok(run) => run,
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

    let run = Run::try_from(storage_runs).map_err(|err| {
        error!(message = "Could not serialize run from storage", error = %err);
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    let resp = GetRunResponse { run };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StartRunRequest {
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StartRunResponse {
    /// Information about the run started.
    pub run: Run,
}

/// Start a run of a particular pipeline.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs",
    tags = ["Runs"],
)]
pub async fn start_run(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunPathArgsRoot>,
    body: TypedBody<StartRunRequest>,
) -> Result<HttpResponseCreated<StartRunResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let body = body.into_inner();
    let req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let new_run = match api_state
        .orchestrator
        .launch_new_run(
            api_state.clone(),
            &path.namespace_id,
            &path.pipeline_id,
            Initiator {
                id: req_metadata.auth.token_id,
                user: req_metadata.auth.token_user,
            },
            body.variables,
        )
        .await
    {
        Ok(run) => run,
        Err(err) => match err {
            OrchestratorError::PipelineRunIgnored => {
                return Err(http_error!(
                    "Runs for this pipeline are currently paused",
                    hyper::StatusCode::SERVICE_UNAVAILABLE,
                    rqctx.request_id,
                    Some(err.into())
                ));
            }
            OrchestratorError::DatabaseGeneralError(storage_error) => {
                return Err(http_error!(
                    "Could not communicate with database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(storage_error.into())
                ));
            }
            OrchestratorError::DatabaseSerializationError(error) => {
                return Err(http_error!(
                    "Could not successfully serialize response",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(error.into())
                ));
            }
            OrchestratorError::PipelineMetadataNotFound => {
                return Err(http_error!(
                    "Could not find pipeline metadata",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(err.into())
                ));
            }
            OrchestratorError::PipelineInactive => {
                return Err(http_error!(
                    "Pipeline is currently inactive",
                    hyper::StatusCode::SERVICE_UNAVAILABLE,
                    rqctx.request_id,
                    Some(err.into())
                ));
            }
            OrchestratorError::UnrecoverableError(err) => {
                return Err(http_error!(
                    "Could not process run due to unforeseen circumstances",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(err.into())
                ));
            }
            OrchestratorError::ObjectStoreGeneralError(error) => {
                return Err(http_error!(
                    "Could not process run due to error in communication with object store",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(error.into())
                ));
            }
            OrchestratorError::SecretStoreGeneralError(secret_store_error) => {
                return Err(http_error!(
                    "Could not process run due to error in communication with secret store",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(secret_store_error.into())
                ));
            }
        },
    };

    let resp = StartRunResponse { run: new_run };

    Ok(HttpResponseCreated(resp))
}

/// Cancel a run by id.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}",
    tags = ["Runs"],
)]
pub async fn cancel_run(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<RunPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Runs,
                ],
                action: Action::Read,
            },
        )
        .await?;

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

    if let Err(e) = storage::runs::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        path.run_id as i64,
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

    // Send an event to the run monitor so it can start the process of cancellations.
    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::StartedRunCancellation {
            namespace_id: path.namespace_id,
            pipeline_id: path.pipeline_id,
            run_id: path.run_id,
        });

    Ok(HttpResponseDeleted())
}
