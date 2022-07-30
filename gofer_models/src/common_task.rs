use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumString};

/// CommonTasks can be enabled and disabled.
#[derive(Debug, Display, EnumString, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Status {
    /// Cannot determine status, should never be in this status.
    Unknown,
    /// Installed and able to be used by pipelines.
    Enabled,
    /// Not available to be used by pipelines, either through lack of installation or
    /// being disabled by an admin.
    Disabled,
}

/// The in-memory representation of a common task.
#[derive(Debug, Clone)]
pub struct CommonTask {
    /// User given custom name for a common task, allowing multiple common tasks of the same image to be used with different
    /// configurations. Must be unique among other common_tasks.
    pub name: String,
    pub image: String, // The docker image string.
    pub registry_auth: Option<super::RegistryAuth>,
    pub variables: HashMap<String, String>,
    pub documentation: Option<String>,
}

/// When installing a new common task, we allow the common task installer to pass a bunch of settings that
/// allow us to customize common task containers on startup.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Registration {
    pub name: String,
    pub image: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub variables: HashMap<String, String>,
    pub created: u64,
    pub status: Status,
}

impl From<gofer_proto::InstallCommonTaskRequest> for Registration {
    fn from(v: gofer_proto::InstallCommonTaskRequest) -> Self {
        Self {
            name: v.name,
            image: v.image,
            user: {
                if v.user.is_empty() {
                    None
                } else {
                    Some(v.user)
                }
            },
            pass: {
                if v.pass.is_empty() {
                    None
                } else {
                    Some(v.pass)
                }
            },
            variables: v.variables,
            created: super::epoch(),
            status: Status::Enabled,
        }
    }
}
