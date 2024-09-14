use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NamespacePathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,
}

/// A namespace represents a grouping of pipelines. Normally it is used to divide teams or logically different
/// sections of workloads. It is the highest level unit as it sits above pipelines in the hierarchy of Gofer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Namespace {
    /// Unique identifier for the namespace.
    pub id: String,

    /// Humanized name for the namespace.
    pub name: String,

    /// Short description about what the namespace is used for.
    pub description: String,

    /// Time in epoch milliseconds when namespace was created.
    pub created: u64,

    /// Time in epoch milliseconds when namespace would expire.
    pub modified: u64,
}

impl Namespace {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Namespace {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            created: epoch_milli(),
            modified: 0,
        }
    }
}

impl TryFrom<storage::namespace::Namespace> for Namespace {
    type Error = anyhow::Error;

    fn try_from(value: storage::namespace::Namespace) -> Result<Self> {
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

        Ok(Namespace {
            id: value.id,
            name: value.name,
            description: value.description,
            created,
            modified,
        })
    }
}

impl From<Namespace> for storage::namespace::Namespace {
    fn from(value: Namespace) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            created: value.created.to_string(),
            modified: value.modified.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListNamespacesResponse {
    /// A list of all namespaces.
    pub namespaces: Vec<Namespace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetNamespaceResponse {
    /// The target namespace.
    pub namespace: Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateNamespaceRequest {
    /// The unique identifier for the namespace. Only accepts alphanumeric chars with hyphens. No spaces.
    pub id: String,

    /// Humanized name for the namespace.
    pub name: String,

    /// Short description about what the namespace is used for.
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateNamespaceResponse {
    /// Information about the namespace created.
    pub namespace: Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateNamespaceRequest {
    /// Humanized name for the namespace.
    pub name: Option<String>,

    /// Short description about what the namespace is used for.
    pub description: Option<String>,
}

impl From<UpdateNamespaceRequest> for storage::namespace::UpdatableFields {
    fn from(value: UpdateNamespaceRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            modified: epoch_milli().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateNamespaceResponse {
    /// Information about the namespace updated.
    pub namespace: Namespace,
}
