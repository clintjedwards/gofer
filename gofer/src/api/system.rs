use crate::api::{ApiState, PreflightOptions, BUILD_COMMIT, BUILD_SEMVER};
use anyhow::Result;
use dropshot::{endpoint, HttpError, HttpResponseOk, RequestContext};
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
pub async fn get_metadata(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<GetSystemMetadataResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: true, // Anyone can query for the version/commit of the system.
                check_namespace: None,
                management_only: false,
            },
        )
        .await?;

    let resp = GetSystemMetadataResponse {
        commit: BUILD_COMMIT.to_string(),
        semver: BUILD_SEMVER.to_string(),
    };
    Ok(HttpResponseOk(resp))
}
