pub mod common_task;
pub mod event;
pub mod namespace;
pub mod pipeline;
pub mod run;
pub mod task;
pub mod task_run;
pub mod trigger;

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

fn epoch() -> u64 {
    let current_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    u64::try_from(current_epoch).unwrap()
}

/// Private repositories sometimes require authentication.
#[derive(Debug, Clone)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

/// The owner for the variable controls where the value of the variable
/// might show up. It also may control ordering of overwriting when the variables are injected
/// into a container.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum VariableOwner {
    Unknown,
    User,
    System,
}

impl From<gofer_proto::variable::VariableOwner> for VariableOwner {
    fn from(p: gofer_proto::variable::VariableOwner) -> Self {
        match p {
            gofer_proto::variable::VariableOwner::Unknown => VariableOwner::Unknown,
            gofer_proto::variable::VariableOwner::User => VariableOwner::User,
            gofer_proto::variable::VariableOwner::System => VariableOwner::System,
        }
    }
}

impl From<VariableOwner> for gofer_proto::variable::VariableOwner {
    fn from(p: VariableOwner) -> Self {
        match p {
            VariableOwner::Unknown => gofer_proto::variable::VariableOwner::Unknown,
            VariableOwner::User => gofer_proto::variable::VariableOwner::User,
            VariableOwner::System => gofer_proto::variable::VariableOwner::System,
        }
    }
}

impl FromStr for VariableOwner {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "unknown" => Ok(VariableOwner::Unknown),
            "user" => Ok(VariableOwner::User),
            "system" => Ok(VariableOwner::System),
            _ => Err(()),
        }
    }
}

/// The sensitivity for a variable defines where the variable will show up.
/// In combination with the owner of the variable it allows Gofer to control
/// which interfaces show which variable types and which are hidden to only
/// administrators.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum VariableSensitivity {
    Unknown,
    Public,
    Private,
}

impl From<gofer_proto::variable::VariableSensitivity> for VariableSensitivity {
    fn from(p: gofer_proto::variable::VariableSensitivity) -> Self {
        match p {
            gofer_proto::variable::VariableSensitivity::Unknown => VariableSensitivity::Unknown,
            gofer_proto::variable::VariableSensitivity::Public => VariableSensitivity::Public,
            gofer_proto::variable::VariableSensitivity::Private => VariableSensitivity::Private,
        }
    }
}

impl From<VariableSensitivity> for gofer_proto::variable::VariableSensitivity {
    fn from(p: VariableSensitivity) -> Self {
        match p {
            VariableSensitivity::Unknown => gofer_proto::variable::VariableSensitivity::Unknown,
            VariableSensitivity::Public => gofer_proto::variable::VariableSensitivity::Public,
            VariableSensitivity::Private => gofer_proto::variable::VariableSensitivity::Private,
        }
    }
}

impl FromStr for VariableSensitivity {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "unknown" => Ok(VariableSensitivity::Unknown),
            "public" => Ok(VariableSensitivity::Public),
            "private" => Ok(VariableSensitivity::Private),
            _ => Err(()),
        }
    }
}

/// A variable is a key value pair that is used either in a run or task level.
/// The variable is inserted as an environment variable to an eventual task run.
/// It can be owned by different parts of the system which control where
/// the potentially sensitive variables might show up.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Variable {
    pub key: String,
    pub value: String,
    pub owner: VariableOwner,
    pub sensitivity: VariableSensitivity,
}

impl From<gofer_proto::Variable> for Variable {
    fn from(p: gofer_proto::Variable) -> Self {
        Variable {
            key: p.key,
            value: p.value,
            owner: gofer_proto::variable::VariableOwner::from_i32(p.owner)
                .unwrap()
                .into(),
            sensitivity: gofer_proto::variable::VariableSensitivity::from_i32(p.sensitivity)
                .unwrap()
                .into(),
        }
    }
}

impl From<Variable> for gofer_proto::Variable {
    fn from(p: Variable) -> Self {
        gofer_proto::Variable {
            key: p.key,
            value: p.value,
            owner: Into::<gofer_proto::variable::VariableOwner>::into(p.owner) as i32,
            sensitivity: Into::<gofer_proto::variable::VariableSensitivity>::into(p.sensitivity)
                as i32,
        }
    }
}
