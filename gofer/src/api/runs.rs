use super::permissioning::{Action, Resource};
use crate::{
    api::{
        epoch_milli, event_utils, pipeline_configs, pipelines, run_utils, ApiState,
        PreflightOptions, Variable, VariableSource,
    },
    http_error, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    endpoint, ClientErrorStatusCode, HttpError, HttpResponseCreated, HttpResponseDeleted,
    HttpResponseOk, Path, Query, RequestContext, TypedBody,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{atomic::Ordering, Arc},
};
use strum::{Display, EnumString};
use tracing::{debug, error};

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

    if api_state.ignore_pipeline_run_events.load(Ordering::SeqCst) {
        debug!(
            "Ignoring pipeline run due to api setting 'ignore_pipeline_run_events' in state 'true'"
        );
        return Err(http_error!(
            "Pipeline run request ignored due to api setting 'ignore_pipeline_run_events' in state 'true'",
            hyper::StatusCode::SERVICE_UNAVAILABLE,
            rqctx.request_id.clone(),
            None
        ));
    }

    let mut tx = match api_state.storage.open_tx().await {
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

    let storage_pipeline_metadata =
        match storage::pipeline_metadata::get(&mut tx, &path.namespace_id, &path.pipeline_id).await
        {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(HttpError::for_not_found(None, String::new()));
                }
                _ => {
                    return Err(http_error!(
                        "Could not get objects from database",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            },
        };

    let pipeline_metadata =
        pipelines::Metadata::try_from(storage_pipeline_metadata).map_err(|err| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(err.into())
            )
        })?;

    if pipeline_metadata.state != pipelines::PipelineState::Active {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "Pipeline is not in state '{}'; cannot start run",
                pipelines::PipelineState::Active
            ),
        ));
    };

    let latest_pipeline_config_storage =
        match storage::pipeline_configs::get_latest(&mut tx, &path.namespace_id, &path.pipeline_id)
            .await
        {
            Ok(config) => config,
            Err(e) => {
                return Err(http_error!(
                    "Could not get latest pipeline config from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let pipeline_tasks = match storage::tasks::list(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        latest_pipeline_config_storage.version,
    )
    .await
    {
        Ok(tasks) => tasks,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let pipeline_config = pipeline_configs::Config::from_storage(
        latest_pipeline_config_storage.clone(),
        pipeline_tasks,
    )
    .map_err(|err| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    let latest_run_id =
        match storage::runs::get_latest(&mut tx, &path.namespace_id, &path.pipeline_id).await {
            Ok(latest_run) => latest_run.run_id,
            Err(err) => match err {
                storage::StorageError::NotFound => 0,
                _ => {
                    return Err(http_error!(
                        "Could not get last run object from database",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(err.into())
                    ));
                }
            },
        };

    let new_run_id = latest_run_id + 1;

    let initiator = Initiator {
        id: req_metadata.auth.token_id,
        user: req_metadata.auth.token_user,
    };

    //TODO(): Implement run_api_token
    let new_run = Run::new(
        &path.namespace_id,
        &path.pipeline_id,
        latest_pipeline_config_storage.version.try_into().unwrap(),
        new_run_id.try_into().unwrap(),
        initiator,
        body.variables
            .into_iter()
            .map(|(key, value)| Variable {
                key,
                value,
                source: VariableSource::RunOptions,
            })
            .collect(),
        None,
    );

    let new_run_storage = new_run.clone().try_into().map_err(|err: anyhow::Error| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    if let Err(e) = storage::runs::insert(&mut tx, &new_run_storage).await {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    ClientErrorStatusCode::CONFLICT,
                    "run entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    // We emit the QueuedRun event so that the event stream is aware of the impending run. Then we insert that event_id
    // into the database for the associated run. This helps us with detecting which runs are incomplete due to
    // a crash within Gofer and helps us restore those runs.
    let queued_run_event = api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::QueuedRun {
            namespace_id: path.namespace_id.clone(),
            pipeline_id: path.pipeline_id.clone(),
            run_id: new_run.run_id,
        });

    let fields = storage::runs::UpdatableFields {
        event_id: Some(queued_run_event.id),
        ..Default::default()
    };

    if let Err(e) = storage::runs::update(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        new_run.run_id as i64,
        fields,
    )
    .await
    {
        return Err(http_error!(
            "Could not insert event_id for new run",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    // Now that the run has been inserted into the database we start it's tracking and execution.
    let new_run_shepard = run_utils::Shepherd::new(
        api_state.clone(),
        pipelines::Pipeline {
            metadata: pipeline_metadata,
            config: pipeline_config,
        },
        new_run.clone(),
    );

    // Make sure the pipeline is read for a new run.
    while new_run_shepard.parallelism_limit_exceeded().await {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Finally, launch the thread that will launch all the task executions for the run.
    tokio::spawn(new_run_shepard.start_run());

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
