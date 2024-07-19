use crate::{
    api::{epoch_milli, event_utils, ApiState, PreflightOptions},
    http_error, storage,
};
use anyhow::{Context, Result};
use dropshot::{endpoint, HttpError, HttpResponseOk, Path, RequestContext};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use strum::{Display, EnumString};
use tracing::error;

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "deployment_state")]
pub enum State {
    /// Should never be in this state.
    #[default]
    Unknown,

    Running,

    Complete,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "deployment_status")]
pub enum Status {
    /// Should only be in this state if the deployment is not yet complete.
    #[default]
    Unknown,

    /// Has encountered an issue, either container issue or scheduling issue.
    Failed,

    /// Finished with a proper exit code.
    Successful,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "deployment_status_reason_type")]
pub enum StatusReasonType {
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "deployment_status_reason")]
pub struct StatusReason {
    /// The specific type of deployment failure.
    pub reason: StatusReasonType,

    /// A description of why the deployment might have failed and what was going on at the time.
    pub description: String,
}

/// A deployment represents a transition between two pipeline versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Deployment {
    /// Unique identifier for the target namespace.
    pub namespace_id: String,

    /// Unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// Unique identifier for the deployment.
    pub deployment_id: u64,

    /// Version of the pipeline is being deprecated.
    pub start_version: u64,

    /// Version of the pipeline being promoted.
    pub end_version: u64,

    /// Time of deployment start in epoch milliseconds.
    pub started: u64,

    /// Time of deployment end in epoch milliseconds.
    pub ended: u64,

    /// The current state of the deployment as it exists within Gofer's operating model.
    pub state: State,

    /// The final status of the deployment.
    pub status: Status,

    /// Details about a deployment's specific status
    pub status_reason: Option<StatusReason>,

    /// The event logs from the deployment.
    pub logs: Vec<event_utils::Event>,
}

impl Deployment {
    pub fn new(
        namespace_id: &str,
        pipeline_id: &str,
        deployment_id: u64,
        start_version: u64,
        end_version: u64,
    ) -> Self {
        Deployment {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            deployment_id,
            start_version,
            end_version,
            started: epoch_milli(),
            ended: 0,
            state: State::Running,
            status: Status::Unknown,
            status_reason: None,
            logs: vec![],
        }
    }
}

impl TryFrom<storage::deployments::Deployment> for Deployment {
    type Error = anyhow::Error;

    fn try_from(value: storage::deployments::Deployment) -> Result<Self> {
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

        let deployment_id = value.deployment_id.try_into().with_context(|| {
            format!(
                "Could not parse field 'deployment_id' from storage value '{}'",
                value.deployment_id
            )
        })?;

        let start_version = value.start_version.try_into().with_context(|| {
            format!(
                "Could not parse field 'start_version' from storage value '{}'",
                value.start_version
            )
        })?;

        let end_version = value.end_version.try_into().with_context(|| {
            format!(
                "Could not parse field 'end_version' from storage value '{}'",
                value.end_version
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
                value.status
            )
        })?;

        let status_reason = serde_json::from_str(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value '{}'",
                value.status_reason
            )
        })?;

        let logs = serde_json::from_str(&value.logs).with_context(|| {
            format!(
                "Could not parse field 'logs' from storage value '{}'",
                value.logs
            )
        })?;

        Ok(Deployment {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            deployment_id,
            start_version,
            end_version,
            started,
            ended,
            state,
            status,
            status_reason,
            logs,
        })
    }
}

impl TryFrom<Deployment> for storage::deployments::Deployment {
    type Error = anyhow::Error;

    fn try_from(value: Deployment) -> Result<Self> {
        let deployment_id = value.deployment_id.try_into().with_context(|| {
            format!(
                "Could not parse field 'deployment_id' from storage value '{}'",
                value.deployment_id
            )
        })?;

        let start_version = value.start_version.try_into().with_context(|| {
            format!(
                "Could not parse field 'start_version' from storage value '{}'",
                value.start_version
            )
        })?;

        let end_version = value.end_version.try_into().with_context(|| {
            format!(
                "Could not parse field 'end_version' from storage value '{}'",
                value.end_version
            )
        })?;

        let status_reason = serde_json::to_string(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value '{:#?}'",
                value.status_reason
            )
        })?;

        let logs = serde_json::to_string(&value.logs).with_context(|| {
            format!(
                "Could not parse field 'logs' from storage value '{:#?}'",
                value.logs
            )
        })?;

        Ok(Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            deployment_id,
            start_version,
            end_version,
            started: value.started.to_string(),
            ended: value.ended.to_string(),
            state: value.state.to_string(),
            status: value.status.to_string(),
            status_reason,
            logs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeploymentPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeploymentPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target deployment.
    pub deployment_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListDeploymentsResponse {
    /// A list of all deployments.
    pub deployments: Vec<Deployment>,
}

/// List all deployments.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments",
    tags = ["Deployments"],
)]
pub async fn list_deployments(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<DeploymentPathArgsRoot>,
) -> Result<HttpResponseOk<ListDeploymentsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_deployments =
        match storage::deployments::list(&mut conn, &path.namespace_id, &path.pipeline_id).await {
            Ok(deployments) => deployments,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let mut deployments: Vec<Deployment> = vec![];

    for storage_deployment in storage_deployments {
        let deployment = Deployment::try_from(storage_deployment).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        deployments.push(deployment);
    }

    let resp = ListDeploymentsResponse { deployments };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetDeploymentResponse {
    /// The target deployment.
    pub deployment: Deployment,
}

/// Get api deployment by id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments/{deployment_id}",
    tags = ["Deployments"],
)]
pub async fn get_deployment(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<DeploymentPathArgs>,
) -> Result<HttpResponseOk<GetDeploymentResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let deployment_id_i64 = path.deployment_id.try_into().map_err(|err| {
        error!(message = "Could not serialize deployment_id to integer value", error = %err);
        HttpError::for_bad_request(
            None,
            format!(
                "Could not serialize deployment_id to integer value; {:#?}",
                err
            ),
        )
    })?;

    let storage_deployment = match storage::deployments::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        deployment_id_i64,
    )
    .await
    {
        Ok(deployment) => deployment,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(e.into())
                ));
            }
        },
    };

    let deployment = Deployment::try_from(storage_deployment).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = GetDeploymentResponse { deployment };
    Ok(HttpResponseOk(resp))
}
