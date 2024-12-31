use super::permissioning::{Action, Resource};
use crate::api::{storage, ApiState, PreflightOptions, BUILD_COMMIT, BUILD_SEMVER};
use crate::http_error;
use anyhow::Result;
use dropshot::{endpoint, HttpError, HttpResponseOk, RequestContext, TypedBody};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSystemMetadataResponse {
    /// The commit of the current build.
    pub commit: String,

    /// The semver version of the current build.
    pub semver: String,
}

/// Describe current system meta-information.
///
/// Return a number of internal metadata about the Gofer service itself.
#[endpoint(
    method = GET,
    path = "/api/system/metadata",
    tags = ["System"],
)]
pub async fn get_system_metadata(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<GetSystemMetadataResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: true, // Anyone can query for the version/commit of the system.
                admin_only: false,
                allow_anonymous: false,
                resources: vec![Resource::System],
                action: Action::Read,
            },
        )
        .await?;

    let resp = GetSystemMetadataResponse {
        commit: BUILD_COMMIT.to_string(),
        semver: BUILD_SEMVER.to_string(),
    };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
/// System preferences
pub struct System {
    pub bootstrap_token_created: bool,
    pub ignore_pipeline_run_events: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSystemPreferencesResponse {
    pub bootstrap_token_created: bool,
    pub ignore_pipeline_run_events: bool,
}

/// Get system parameters.
#[endpoint(
    method = GET,
    path = "/api/system",
    tags = ["System"],
)]
pub async fn get_system_preferences(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<GetSystemPreferencesResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::System],
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

    let system_preferences = match storage::system::get_system_parameters(&mut conn).await {
        Ok(perf) => perf,
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

    let resp = GetSystemPreferencesResponse {
        bootstrap_token_created: system_preferences.bootstrap_token_created,
        ignore_pipeline_run_events: system_preferences.ignore_pipeline_run_events,
    };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSystemPreferencesRequest {
    pub ignore_pipeline_run_events: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSystemPreferencesResponse {}

/// Update system parameters.
#[endpoint(
    method = PATCH,
    path = "/api/system",
    tags = ["System"],
)]
pub async fn update_system_preferences(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<UpdateSystemPreferencesRequest>,
) -> Result<HttpResponseOk<UpdateSystemPreferencesResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                allow_anonymous: false,
                resources: vec![Resource::System],
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

    if let Err(e) =
        storage::system::update_system_parameters(&mut conn, None, body.ignore_pipeline_run_events)
            .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Extension entry for id given does not exist".into(),
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

    if let Some(value) = body.ignore_pipeline_run_events {
        api_state
            .ignore_pipeline_run_events
            .swap(value, std::sync::atomic::Ordering::Relaxed);
    }

    Ok(HttpResponseOk(UpdateSystemPreferencesResponse {}))
}
