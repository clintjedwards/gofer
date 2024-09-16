use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSystemMetadataResponse {
    /// The commit of the current build.
    pub commit: String,

    /// The semver version of the current build.
    pub semver: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSystemPreferencesRequest {
    pub ignore_pipeline_run_events: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSystemPreferencesResponse {}
