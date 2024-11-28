use super::permissioning::{Action, Resource};
use crate::{
    api::{epoch_milli, pipeline_configs, ApiState, PreflightOptions},
    http_error, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseDeleted, HttpResponseOk, HttpResponseUpdatedNoContent, Path,
    RequestContext, TypedBody,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelinePathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelinePathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum PipelineState {
    #[default]
    Unknown,

    Active,

    Disabled,
}

/// Details about the pipeline itself, not including the configuration that the user can change.
/// All these values are changed by the system or never changed at all. This sits in contrast to
/// the config which the user can change freely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Metadata {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Time of pipeline creation in epoch milliseconds.
    pub created: u64,

    /// Time pipeline was updated to a new version in epoch milliseconds.
    pub modified: u64,

    /// The current running state of the pipeline. This is used to determine if the pipeline should run or not.
    pub state: PipelineState,
}

impl Metadata {
    pub fn new(namespace_id: &str, pipeline_id: &str) -> Self {
        Metadata {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            created: epoch_milli(),
            modified: 0,
            state: PipelineState::Active,
        }
    }
}

impl TryFrom<storage::pipeline_metadata::PipelineMetadata> for Metadata {
    type Error = anyhow::Error;

    fn try_from(value: storage::pipeline_metadata::PipelineMetadata) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let modified = value.modified.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'modified' from storage value '{}'",
                value.modified
            )
        })?;

        let state = PipelineState::from_str(&value.state).with_context(|| {
            format!(
                "Could not parse field 'token type' from storage value '{}'",
                value.state
            )
        })?;

        Ok(Metadata {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            created,
            modified,
            state,
        })
    }
}

impl From<Metadata> for storage::pipeline_metadata::PipelineMetadata {
    fn from(value: Metadata) -> Self {
        Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            created: value.created.to_string(),
            modified: value.modified.to_string(),
            state: value.state.to_string(),
        }
    }
}

/// A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
/// Pipeline is a secondary level unit being contained within namespaces and containing runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Pipeline {
    /// Macro level details on the targeted pipeline.
    pub metadata: Metadata,

    /// User controlled data for the targeted pipeline.
    pub config: pipeline_configs::Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelinesResponse {
    /// A list of all pipelines metadata.
    pub pipelines: Vec<Metadata>,
}

/// List all pipelines.
///
/// Returns the metadata for all pipelines. If you want a more complete picture of the pipeline details
/// combine this endpoint with the configs endpoint to grab the metadata AND the user's pipeline configuration.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines",
    tags = ["Pipelines"],
)]
pub async fn list_pipelines(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelinePathArgsRoot>,
) -> Result<HttpResponseOk<ListPipelinesResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: true,
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines("".into()),
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_pipelines =
        match storage::pipeline_metadata::list(&mut conn, &path.namespace_id).await {
            Ok(pipelines) => pipelines,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let mut pipelines: Vec<Metadata> = vec![];

    for storage_pipeline in storage_pipelines {
        let pipeline = Metadata::try_from(storage_pipeline).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        pipelines.push(pipeline);
    }

    let resp = ListPipelinesResponse { pipelines };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineResponse {
    /// The metadata for the pipeline.
    pub pipeline: Metadata,
}

/// Get pipeline by id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}",
    tags = ["Pipelines"],
)]
pub async fn get_pipeline(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelinePathArgs>,
) -> Result<HttpResponseOk<GetPipelineResponse>, HttpError> {
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
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_pipeline_metadata =
        match storage::pipeline_metadata::get(&mut conn, &path.namespace_id, &path.pipeline_id)
            .await
        {
            Ok(pipeline) => pipeline,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(HttpError::for_not_found(None, String::new()));
                }
                _ => {
                    return Err(http_error!(
                        "Could not get objects from database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            },
        };

    let metadata = Metadata::try_from(storage_pipeline_metadata).map_err(|err| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    let resp = GetPipelineResponse { pipeline: metadata };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdatePipelineRequest {
    pub state: Option<PipelineState>,
}

/// Update a pipeline's state.
#[endpoint(
    method = PATCH,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}",
    tags = ["Pipelines"],
)]
pub async fn update_pipeline(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelinePathArgs>,
    body: TypedBody<UpdatePipelineRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
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
                ],
                action: Action::Write,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn().await {
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

    let updatable_fields = storage::pipeline_metadata::UpdatableFields {
        state: body.state.map(|state| state.to_string()),
        ..Default::default()
    };

    if let Err(e) = storage::pipeline_metadata::update(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        updatable_fields,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Pipeline entry for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not update object in database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    Ok(HttpResponseUpdatedNoContent())
}

/// Delete pipeline by id.
///
/// IMPORTANT: Deleting a pipeline is set to cascade. All downstream objects to the pipeline (configs, secrets, runs, tasks)
/// will be removed as well.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}",
    tags = ["Pipelines"],
)]
pub async fn delete_pipeline(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelinePathArgs>,
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
                ],
                action: Action::Delete,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn().await {
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

    if let Err(e) =
        storage::pipeline_metadata::delete(&mut conn, &path.namespace_id, &path.pipeline_id).await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "pipeline for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    Ok(HttpResponseDeleted())
}
