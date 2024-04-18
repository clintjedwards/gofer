use anyhow::{Context, Result};
use progenitor::generate_api;
use std::fmt;
use std::str::FromStr;

/// A constant for the header that tracks which version of the API a client has requested.
pub const API_VERSION_HEADER: &str = "gofer-api-version";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiVersion {
    V0,
}

impl ApiVersion {
    pub fn to_list() -> [String; 1] {
        ["v0".into()]
    }

    pub fn to_header_value(&self) -> Result<http::header::HeaderValue> {
        http::header::HeaderValue::from_str(&self.to_string())
            .context("could not construct header value")
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::V0 => write!(f, "v0"),
        }
    }
}

impl FromStr for ApiVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "v0" => Ok(ApiVersion::V0),
            _ => Err(anyhow::anyhow!("Invalid API version")),
        }
    }
}

generate_api!("../../gofer/docs/src/assets/openapi.json");
