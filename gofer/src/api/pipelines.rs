use super::permissioning::{Action, NamespaceResource, Requirement};
use crate::{
    api::{
        ApiState, PreflightOptions, ensure_namespace_exists, ensure_pipeline_exists, epoch_milli,
        pipeline_configs,
    },
    http_error, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    HttpError, HttpResponseDeleted, HttpResponseOk, HttpResponseUpdatedNoContent, Path,
    RequestContext, TypedBody, endpoint,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Works out a pipeline's state from what's stored. Nothing flips a pipeline back to active the moment its timed disable
/// runs out, so anything reading the stored state has to go through this. Returns the state along with when the disable
/// ends, which is 0 unless the pipeline is disabled with an end time.
pub fn state_in_effect(
    state: PipelineState,
    disabled_until: u64,
    now: u64,
) -> (PipelineState, u64) {
    match state {
        PipelineState::Disabled if disabled_until != 0 && disabled_until <= now => {
            (PipelineState::Active, 0)
        }
        PipelineState::Disabled => (state, disabled_until),
        _ => (state, 0),
    }
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

    /// When a disabled pipeline becomes active again, in epoch milliseconds. 0 if it isn't disabled or if it stays that
    /// way until someone enables it.
    pub disabled_until: u64,
}

impl Metadata {
    pub fn new(namespace_id: &str, pipeline_id: &str) -> Self {
        Metadata {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            created: epoch_milli(),
            modified: 0,
            state: PipelineState::Active,
            disabled_until: 0,
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

        let disabled_until = value.disabled_until.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'disabled_until' from storage value '{}'",
                value.disabled_until
            )
        })?;

        let (state, disabled_until) = state_in_effect(state, disabled_until, epoch_milli());

        Ok(Metadata {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            created,
            modified,
            state,
            disabled_until,
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
            disabled_until: value.disabled_until.to_string(),
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

/// A pipeline as it shows up in a listing: its metadata plus the name and description from its newest config, so
/// callers can show something friendlier than the id without fetching every config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineSummary {
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

    /// When a disabled pipeline becomes active again, in epoch milliseconds. 0 if it isn't disabled or if it stays that
    /// way until someone enables it.
    pub disabled_until: u64,

    /// Humanized name from the newest registered config.
    pub name: String,

    /// Description from the newest registered config.
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelinesResponse {
    /// A list of all pipelines.
    pub pipelines: Vec<PipelineSummary>,
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
    let req_metadata = api_state
        .preflight_check(
            &rqctx,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: true,
                requires: Requirement::Pipeline {
                    namespace: path.namespace_id.clone(),
                    pipeline: None,
                    resource: NamespaceResource::Pipelines,
                },
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

    ensure_namespace_exists(&mut conn, &rqctx.request_id, &path.namespace_id).await?;

    let storage_pipelines =
        match storage::pipeline_metadata::list(&mut conn, &path.namespace_id).await {
            Ok(pipelines) => pipelines,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let latest_configs =
        match storage::pipeline_configs::list_latest(&mut conn, &path.namespace_id).await {
            Ok(configs) => configs,
            Err(e) => {
                return Err(http_error!(
                    "Could not get objects from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    let mut names: HashMap<String, (String, String)> = latest_configs
        .into_iter()
        .map(|config| (config.pipeline_id, (config.name, config.description)))
        .collect();

    let mut pipelines: Vec<PipelineSummary> = vec![];

    for storage_pipeline in storage_pipelines {
        let pipeline = Metadata::try_from(storage_pipeline).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        let requirement = Requirement::pipeline(
            &pipeline.namespace_id,
            &pipeline.pipeline_id,
            NamespaceResource::Pipelines,
        );

        if req_metadata.allows(&requirement, &Action::Read) {
            // Every pipeline is created with a config, so this only falls back if the data is inconsistent.
            let (name, description) = names.remove(&pipeline.pipeline_id).unwrap_or_default();

            pipelines.push(PipelineSummary {
                namespace_id: pipeline.namespace_id,
                pipeline_id: pipeline.pipeline_id,
                created: pipeline.created,
                modified: pipeline.modified,
                state: pipeline.state,
                disabled_until: pipeline.disabled_until,
                name,
                description,
            });
        }
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
            &rqctx,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: true,
                admin_only: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Pipelines,
                ),
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
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            },
        };

    let metadata = Metadata::try_from(storage_pipeline_metadata).map_err(|err| {
        http_error!(
            "Could not parse object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
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

    /// When the pipeline becomes active again by itself, in epoch milliseconds. Can only be set along with the state
    /// `disabled`, and has to be in the future. Leave it out to keep the pipeline disabled until it's enabled again.
    pub disabled_until: Option<u64>,
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
            &rqctx,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Pipelines,
                ),
                action: Action::Write,
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

    ensure_pipeline_exists(
        &mut conn,
        &rqctx.request_id,
        &path.namespace_id,
        &path.pipeline_id,
    )
    .await?;

    if let Some(disabled_until) = body.disabled_until {
        if body.state != Some(PipelineState::Disabled) {
            return Err(HttpError::for_bad_request(
                None,
                "disabled_until can only be set when state is 'disabled'".into(),
            ));
        }

        if disabled_until <= epoch_milli() {
            return Err(HttpError::for_bad_request(
                None,
                "disabled_until must be in the future".into(),
            ));
        }
    }

    // Any change of state replaces the old end time, so enabling a pipeline or disabling it for good also clears it.
    let disabled_until = body
        .state
        .as_ref()
        .map(|_| body.disabled_until.unwrap_or_default().to_string());

    let updatable_fields = storage::pipeline_metadata::UpdatableFields {
        disabled_until,
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
            storage::StorageError::NoFieldsUpdated => {}
            _ => {
                return Err(http_error!(
                    "Could not update object in database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
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
            &rqctx,
            PreflightOptions {
                bypass_auth: false,
                allow_anonymous: false,
                admin_only: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Pipelines,
                ),
                action: Action::Delete,
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
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    Ok(HttpResponseDeleted())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::active(PipelineState::Active, 0, (PipelineState::Active, 0))]
    #[case::disabled_for_good(PipelineState::Disabled, 0, (PipelineState::Disabled, 0))]
    #[case::before_end(PipelineState::Disabled, 1001, (PipelineState::Disabled, 1001))]
    #[case::at_end(PipelineState::Disabled, 1000, (PipelineState::Active, 0))]
    #[case::after_end(PipelineState::Disabled, 999, (PipelineState::Active, 0))]
    #[case::unknown(PipelineState::Unknown, 999, (PipelineState::Unknown, 0))]
    fn state_in_effect_ends_timed_disables(
        #[case] state: PipelineState,
        #[case] disabled_until: u64,
        #[case] expected: (PipelineState, u64),
    ) {
        assert_eq!(state_in_effect(state, disabled_until, 1000), expected);
    }
}
