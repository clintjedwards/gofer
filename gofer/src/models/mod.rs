mod namespace;
mod pipeline;
mod run;
mod task;
mod task_run;

pub use self::namespace::*;
pub use self::pipeline::*;
pub use self::run::*;
pub use self::task::*;
pub use self::task_run::*;

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

/// The owner for the variable controls where the value of the variable
/// might show up.
/// It also may control ordering of overwriting when the variables are injected
/// into a container.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum VariableOwner {
    Unknown,
    User,
    System,
}

impl From<gofer_proto::VariableOwner> for VariableOwner {
    fn from(p: gofer_proto::VariableOwner) -> Self {
        match p {
            gofer_proto::VariableOwner::Unknown => VariableOwner::Unknown,
            gofer_proto::VariableOwner::User => VariableOwner::User,
            gofer_proto::VariableOwner::System => VariableOwner::System,
        }
    }
}

impl From<VariableOwner> for gofer_proto::VariableOwner {
    fn from(p: VariableOwner) -> Self {
        match p {
            VariableOwner::Unknown => gofer_proto::VariableOwner::Unknown,
            VariableOwner::User => gofer_proto::VariableOwner::User,
            VariableOwner::System => gofer_proto::VariableOwner::System,
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

/// A variable is a key value pair that is used either in a run or task level.
/// The variable is inserted as an environment variable to an eventual task run.
/// It can be owned by different parts of the system and which controls where
/// the potentially sensitive variables might show up.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Variable {
    pub key: String,
    pub value: String,
    pub owner: VariableOwner,
}

impl From<gofer_proto::Variable> for Variable {
    fn from(p: gofer_proto::Variable) -> Self {
        Variable {
            key: p.key,
            value: p.value,
            owner: gofer_proto::VariableOwner::from_i32(p.owner)
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
            owner: Into::<gofer_proto::VariableOwner>::into(p.owner) as i32,
        }
    }
}
