use crate::storage;
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum::{Display, EnumString};

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "subscription_status")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum Status {
    #[default]
    Unknown,

    Active,

    Error,

    Disabled,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "subscription_status_reason_type")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum StatusReasonType {
    #[default]
    Unknown,

    NotFound,

    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "subscription_status_reason")]
pub struct StatusReason {
    /// The specific type of subscription failure.
    pub reason: StatusReasonType,

    /// A description of why the subscription might have failed and what was going on at the time.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Subscription {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Unique identifier of the target extension.
    pub extension_id: String,

    /// A unique label differentiating this subscription from other subscriptions.
    pub subscription_id: String,

    /// The extension's pipeline configuration settings.
    pub settings: HashMap<String, String>,

    /// The state of the subscription.
    pub status: Status,

    /// A further description about the status.
    pub status_reason: Option<StatusReason>,
}

impl Subscription {
    fn from_create_request(
        namespace_id: &str,
        pipeline_id: &str,
        req: CreateSubscriptionRequest,
    ) -> Subscription {
        Self {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            extension_id: req.extension_id,
            subscription_id: req.subscription_id,
            settings: req.settings,
            status: Status::Active,
            status_reason: None,
        }
    }
}

impl TryFrom<storage::extension_subscription::ExtensionSubscription> for Subscription {
    type Error = anyhow::Error;

    fn try_from(value: storage::extension_subscription::ExtensionSubscription) -> Result<Self> {
        let settings = serde_json::from_str(&value.settings).with_context(|| {
            format!(
                "Could not parse field 'settings' from storage value '{}'",
                value.settings
            )
        })?;

        let status_reason = serde_json::from_str(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value '{}'",
                value.status_reason
            )
        })?;

        let status = Status::from_str(&value.status).with_context(|| {
            format!(
                "Could not parse field 'status' from storage value '{}'",
                value.status
            )
        })?;

        Ok(Subscription {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            extension_id: value.extension_id,
            subscription_id: value.extension_subscription_id,
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
                "Could not parse field 'settings' to storage value '{:#?}'",
                value.settings
            )
        })?;

        let status_reason = serde_json::to_string(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' to storage value '{:#?}'",
                value.status_reason
            )
        })?;

        Ok(Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            extension_id: value.extension_id,
            extension_subscription_id: value.subscription_id,
            settings,
            status: value.status.to_string(),
            status_reason,
        })
    }
}

/* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id} */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SubscriptionPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SubscriptionPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target extension.
    pub extension_id: String,

    /// The unique identifier for the target subscription.
    pub subscription_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListSubscriptionsResponse {
    /// A list of all pipeline subscriptions.
    pub subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSubscriptionResponse {
    /// The metadata for the subscription.
    pub subscription: Subscription,
}

#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum UpdateSubscriptionStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSubscriptionRequest {
    pub status: Option<UpdateSubscriptionStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateSubscriptionRequest {
    pub extension_id: String,
    pub subscription_id: String,
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateSubscriptionResponse {
    /// Information about the subscription created.
    pub subscription: Subscription,
}
