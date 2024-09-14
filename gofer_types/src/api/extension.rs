use crate::{storage, RegistryAuth, Variable};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionPathArgs {
    /// The unique identifier for the target extension.
    pub extension_id: String,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "extension_state")]
pub enum State {
    /// Should never be in this state.
    #[default]
    Unknown,

    /// Pre-scheduling validation and prep.
    Processing,

    /// Currently running as reported by scheduler.
    Running,

    /// Extension has exited; usually because of an error.
    Exited,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[schemars(rename = "extension_status")]
pub enum Status {
    /// Cannot determine status of Extension; should never be in this status.
    #[default]
    Unknown,

    /// Installed and able to be used by pipelines.
    Enabled,

    /// Not available to be used by pipelines, either through lack of installation or being disabled by an admin.
    Disabled,
}

/// When installing a new extension, we allow the extension installer to pass a bunch of settings that allow us to
/// go get that extension on future startups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Registration {
    /// Unique identifier for the extension.
    pub extension_id: String,

    /// Which container image this extension should run.
    pub image: String,

    /// Auth credentials for the image's registry.
    pub registry_auth: Option<RegistryAuth>,

    /// Extensions allow configuration through env vars passed to them through this field. Refer to the extension's
    /// documentation for setting values.
    pub settings: Vec<Variable>,

    /// Time of registration creation in epoch milliseconds.
    pub created: u64,

    /// Time of last modification in epoch milliseconds.
    pub modified: u64,

    /// Whether the extension is enabled or not; extensions can be disabled to prevent use by admins.
    pub status: Status,

    /// Gofer creates an API key that it passes to extensions on start up in order to facilitate extensions talking
    /// back to the Gofer API. This is the identifier for that key.
    #[serde(skip)]
    key_id: String,
}

impl TryFrom<storage::extension_registration::ExtensionRegistration> for Registration {
    type Error = anyhow::Error;

    fn try_from(value: storage::extension_registration::ExtensionRegistration) -> Result<Self> {
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

        let status = Status::from_str(&value.status).with_context(|| {
            format!(
                "Could not parse field 'status' from storage value '{}'",
                value.status
            )
        })?;

        let registry_auth = serde_json::from_str(&value.registry_auth).with_context(|| {
            format!(
                "Could not parse field 'registry_auth' from storage value; '{:#?}'",
                value.registry_auth
            )
        })?;

        let settings = serde_json::from_str(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' from storage value; '{:#?}'",
                value.settings
            )
        })?;

        Ok(Registration {
            extension_id: value.extension_id,
            image: value.image,
            registry_auth,
            settings,
            created,
            modified,
            status,
            key_id: value.key_id,
        })
    }
}

impl TryFrom<Registration> for storage::extension_registration::ExtensionRegistration {
    type Error = anyhow::Error;

    fn try_from(value: Registration) -> Result<Self> {
        let registry_auth = serde_json::to_string(&value.registry_auth).with_context(|| {
            format!(
                "Could not parse field 'registry_auth' to storage value; '{:#?}'",
                value.registry_auth
            )
        })?;

        let settings = serde_json::to_string(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' to storage value; '{:#?}'",
                value.settings
            )
        })?;

        Ok(Self {
            extension_id: value.extension_id,
            image: value.image,
            registry_auth,
            settings,
            created: value.created.to_string(),
            modified: value.modified.to_string(),
            status: value.status.to_string(),
            key_id: value.key_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Parameter {
    pub key: String,
    pub required: bool,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Documentation {
    /// Each extension has configuration parameters that can be passed in at extension startup. These parameters
    /// should control extension behavior for it's entire lifetime.
    pub config_params: Vec<Parameter>,

    /// Each extension has pipeline subscription parameters that are passed in by a pipeline when it attempts to
    /// subscribe to an extension. This controls how the extension treats that specific pipeline subscription.
    pub pipeline_subscription_params: Vec<Parameter>,

    /// Anything the extension wants to explain to the user. This text is inserted into the documentation a user
    /// can look up about the extension. Supports AsciiDoc.
    pub body: String,
}

// impl From<gofer_sdk::extension::api::types::Documentation> for Documentation {
//     fn from(value: gofer_sdk::extension::api::types::Documentation) -> Self {
//         Documentation {
//             config_params: value
//                 .config_params
//                 .into_iter()
//                 .map(|param| Parameter {
//                     key: param.key,
//                     required: param.required,
//                     documentation: param.documentation,
//                 })
//                 .collect(),
//             pipeline_subscription_params: value
//                 .pipeline_subscription_params
//                 .into_iter()
//                 .map(|param| Parameter {
//                     key: param.key,
//                     required: param.required,
//                     documentation: param.documentation,
//                 })
//                 .collect(),
//             body: value.body,
//         }
//     }
// }

/// An Extension is the way that pipelines add extra functionality to themselves. Pipelines can "subscribe" to
/// extensions and extensions then act on behalf of that pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Extension {
    /// Metadata about the extension as it is registered within Gofer.
    pub registration: Registration,

    /// The network address used to communicate with the extension by the main process.
    pub url: String,

    /// The start time of the extension in epoch milliseconds.
    pub started: u64,

    /// The current state of the extension as it exists within Gofer's operating model.
    pub state: State,

    /// Extension given documentation usually in markdown.
    pub documentation: Documentation,

    /// Key is an extension's authentication key used to validate requests from the Gofer main service. On every
    /// request the Gofer main service passes this key so that it is impossible for others to contact and manipulate
    /// extensions directly.
    #[serde(skip)]
    pub secret: String,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum SubscriptionStatus {
    #[default]
    Unknown,

    /// Successfully connected and active.
    Active,

    /// Not connected and inactive due to error.
    Error,

    /// Inactive due to user or operator request.
    Disabled,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum SubscriptionStatusReasonType {
    /// Gofer has no fucking clue how the run got into this state.
    #[default]
    Unknown,

    /// Subscription is not registered within Gofer.
    NotFound,

    SubscriptionFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct SubscriptionStatusReason {
    /// The specific type of subscription failure.
    pub reason: SubscriptionStatusReasonType,

    /// A description of why the subscription might have failed and what was going on at the time.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Subscription {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Unique identifier for the target extension.
    pub extension_id: String,

    /// A per pipeline unique identifier to differentiate multiple subscriptions to a single pipeline.
    pub label: String,

    /// Each extension defines per pipeline settings that the user can subscribe with to perform different functionalities;
    /// These are generally listed in the extension documentation and passed through here.
    pub settings: HashMap<String, String>,

    /// The state of the subscription for the pipeline; defines whether this subscription is still active.
    pub status: SubscriptionStatus,

    /// More details about why a subscription has a particular status.
    pub status_reason: SubscriptionStatusReason,
}

impl TryFrom<storage::extension_subscription::ExtensionSubscription> for Subscription {
    type Error = anyhow::Error;

    fn try_from(value: storage::extension_subscription::ExtensionSubscription) -> Result<Self> {
        let settings = serde_json::from_str(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' from storage value; '{:#?}'",
                value.settings
            )
        })?;

        let status = SubscriptionStatus::from_str(&value.status).with_context(|| {
            format!(
                "Could not parse field 'status' from storage value; '{:#?}'",
                value.status
            )
        })?;

        let status_reason = serde_json::from_str(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        Ok(Subscription {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            extension_id: value.extension_id,
            label: value.extension_subscription_id,
            settings,
            status,
            status_reason,
        })
    }
}

impl TryFrom<Subscription> for storage::extension_subscription::ExtensionSubscription {
    type Error = anyhow::Error;

    fn try_from(value: Subscription) -> Result<Self> {
        let settings = serde_json::to_string(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' from storage value; '{:#?}'",
                value.settings
            )
        })?;

        let status_reason = serde_json::to_string(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        Ok(Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            extension_id: value.extension_id,
            extension_subscription_id: value.label,
            settings,
            status: value.status.to_string(),
            status_reason,
        })
    }
}
