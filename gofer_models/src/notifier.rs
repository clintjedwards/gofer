use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The in-memory representation of a notifier.
pub struct Notifier {
    /// User given custom name for a notifier, allowing multiple notifiers of the same image to be used with different
    /// configurations. Must be unique among other notifiers.
    pub name: String,
    pub image: String, // The docker image string.
    pub registry_auth: Option<super::RegistryAuth>,
    pub variables: HashMap<String, String>,
    pub documentation: Option<String>,
}

/// When installing a new notifier, we allow the notifier installer to pass a bunch of settings that
/// allow us to customize notifier containers on startup.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct NotifierRegistration {
    pub name: String,
    pub image: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub variables: HashMap<String, String>,
    pub created: u64,
}

impl From<gofer_proto::InstallNotifierRequest> for NotifierRegistration {
    fn from(v: gofer_proto::InstallNotifierRequest) -> Self {
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
        }
    }
}
