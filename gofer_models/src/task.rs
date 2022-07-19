use super::{Variable, VariableOwner, VariableSensitivity};
use gofer_sdk::config;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

/// Defines what a tasks parent status must be for the task to continue.
/// This indirectly maps back to a task run's [status](super::TaskRunStatus) and
/// is only evaluated for task runs that have come to a [Complete](super::TaskRunState::Complete) state.
///
/// Somewhat uniquely an Any status will accept both a success or failure or a task run, but also will
/// run even if the status was [skipped](super::TaskRunState::Skipped).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum RequiredParentStatus {
    Unknown,
    Any,
    Success,
    Failure,
}

impl From<gofer_proto::task::RequiredParentStatus> for RequiredParentStatus {
    fn from(r: gofer_proto::task::RequiredParentStatus) -> Self {
        match r {
            gofer_proto::task::RequiredParentStatus::Unknown => RequiredParentStatus::Unknown,
            gofer_proto::task::RequiredParentStatus::Any => RequiredParentStatus::Any,
            gofer_proto::task::RequiredParentStatus::Success => RequiredParentStatus::Success,
            gofer_proto::task::RequiredParentStatus::Failure => RequiredParentStatus::Failure,
        }
    }
}

impl From<RequiredParentStatus> for gofer_proto::task::RequiredParentStatus {
    fn from(r: RequiredParentStatus) -> Self {
        match r {
            RequiredParentStatus::Unknown => gofer_proto::task::RequiredParentStatus::Unknown,
            RequiredParentStatus::Any => gofer_proto::task::RequiredParentStatus::Any,
            RequiredParentStatus::Success => gofer_proto::task::RequiredParentStatus::Success,
            RequiredParentStatus::Failure => gofer_proto::task::RequiredParentStatus::Failure,
        }
    }
}

impl From<config::RequiredParentStatus> for RequiredParentStatus {
    fn from(r: config::RequiredParentStatus) -> Self {
        match r {
            config::RequiredParentStatus::Unknown => RequiredParentStatus::Unknown,
            config::RequiredParentStatus::Any => RequiredParentStatus::Any,
            config::RequiredParentStatus::Success => RequiredParentStatus::Success,
            config::RequiredParentStatus::Failure => RequiredParentStatus::Failure,
        }
    }
}

impl FromStr for RequiredParentStatus {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "unknown" => Ok(RequiredParentStatus::Unknown),
            "any" => Ok(RequiredParentStatus::Any),
            "success" => Ok(RequiredParentStatus::Success),
            "failure" => Ok(RequiredParentStatus::Failure),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

impl From<gofer_proto::RegistryAuth> for RegistryAuth {
    fn from(p: gofer_proto::RegistryAuth) -> Self {
        RegistryAuth {
            user: p.user,
            pass: p.pass,
        }
    }
}

impl From<RegistryAuth> for gofer_proto::RegistryAuth {
    fn from(p: RegistryAuth) -> Self {
        gofer_proto::RegistryAuth {
            user: p.user,
            pass: p.pass,
        }
    }
}

impl From<config::RegistryAuth> for RegistryAuth {
    fn from(p: config::RegistryAuth) -> Self {
        RegistryAuth {
            user: p.user,
            pass: p.pass,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Task {
    pub id: String,
    pub description: Option<String>,
    pub image: String,
    pub registry_auth: Option<RegistryAuth>,
    pub depends_on: HashMap<String, RequiredParentStatus>,
    pub variables: Vec<Variable>,
    pub entrypoint: Vec<String>,
    pub command: Vec<String>,
}

impl Task {
    pub fn new(id: &str, image: &str) -> Self {
        Self {
            id: id.to_string(),
            description: None,
            image: image.to_string(),
            registry_auth: None,
            depends_on: HashMap::new(),
            variables: Vec::new(),
            entrypoint: Vec::new(),
            command: Vec::new(),
        }
    }
}

impl From<gofer_proto::Task> for Task {
    fn from(p: gofer_proto::Task) -> Self {
        Task {
            id: p.id,
            description: {
                if p.description.is_empty() {
                    None
                } else {
                    Some(p.description)
                }
            },
            image: p.image,
            registry_auth: p.registry_auth.map(RegistryAuth::from),
            depends_on: p
                .depends_on
                .into_iter()
                .map(|(key, value)| {
                    let value = gofer_proto::task::RequiredParentStatus::from_i32(value).unwrap();
                    (key, value.into())
                })
                .collect(),
            variables: { p.variables.into_iter().map(Variable::from).collect() },
            entrypoint: p.entrypoint,
            command: p.command,
        }
    }
}

impl From<Task> for gofer_proto::Task {
    fn from(p: Task) -> Self {
        gofer_proto::Task {
            id: p.id,
            description: p.description.unwrap_or_default(),
            image: p.image,
            registry_auth: p.registry_auth.map(gofer_proto::RegistryAuth::from),
            depends_on: p
                .depends_on
                .into_iter()
                .map(|(key, value)| {
                    (
                        key,
                        gofer_proto::task::RequiredParentStatus::from(value) as i32,
                    )
                })
                .collect(),
            variables: { p.variables.into_iter().map(|var| var.into()).collect() },
            entrypoint: p.entrypoint,
            command: p.command,
        }
    }
}

impl From<config::Task> for Task {
    fn from(p: config::Task) -> Self {
        Task {
            id: p.id,
            description: p.description,
            image: p.image,
            registry_auth: p.registry_auth.map(|ra| ra.into()),
            depends_on: p
                .depends_on
                .into_iter()
                .map(|(key, value)| (key, RequiredParentStatus::from(value)))
                .collect(),
            variables: {
                p.variables
                    .into_iter()
                    .map(|(key, value)| Variable {
                        key,
                        value,
                        owner: VariableOwner::User,
                        sensitivity: VariableSensitivity::Unknown,
                    })
                    .collect()
            },
            entrypoint: p.entrypoint,
            command: p.command,
        }
    }
}
