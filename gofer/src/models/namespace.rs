use crate::models::epoch;

/// Represents a division of pipelines. Normally it is used to divide teams or logically different
/// sections of workloads. This is the highest level unit.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    /// Unique user defined identifier.
    pub id: String,
    /// Humanized name, meant for display.
    pub name: String,
    /// Short description of what namespace is used for.
    pub description: String,
    /// The creation time in epoch milli.
    pub created: u64,
    /// The last modified time in epoch milli.
    pub modified: u64,
}

impl Namespace {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Namespace {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            created: epoch(),
            modified: epoch(),
        }
    }
}

impl From<Namespace> for gofer_proto::Namespace {
    fn from(ns: Namespace) -> Self {
        gofer_proto::Namespace {
            id: ns.id,
            name: ns.name,
            description: ns.description,
            created: ns.created,
            modified: ns.modified,
        }
    }
}

impl From<gofer_proto::Namespace> for Namespace {
    fn from(ns: gofer_proto::Namespace) -> Self {
        Namespace {
            id: ns.id,
            name: ns.name,
            description: ns.description,
            created: ns.created,
            modified: ns.modified,
        }
    }
}
