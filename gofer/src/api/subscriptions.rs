use super::permissioning::{Action, NamespaceResource, Requirement};
use crate::{
    api::{
        ApiState, BadReference, PreflightOptions, Variable, VariableSource, ensure_pipeline_exists,
        event_utils, extensions, interpolate_vars, is_valid_identifier,
    },
    http_error, storage,
};
use anyhow::{Context, Result, anyhow};
use dropshot::{
    ClientErrorStatusCode, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk,
    HttpResponseUpdatedNoContent, Path, RequestContext, TypedBody, endpoint,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};
use strum::{Display, EnumString};
use tracing::debug;

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

    /// Whether the subscription is working. `error` means the extension behind it isn't installed or isn't running,
    /// so the subscription does nothing until it's back.
    pub status: Status,

    /// Why the subscription is in `error`, including what's wrong with the extension.
    pub status_reason: Option<StatusReason>,
}

/// Lowercases the first letter so an extension's reason reads as the second half of a sentence, but leaves acronyms
/// like "HTTP" alone.
fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if first.is_uppercase() && !second.is_uppercase() => first
            .to_lowercase()
            .chain(text[first.len_utf8()..].chars())
            .collect(),
        _ => text.to_string(),
    }
}

impl Subscription {
    /// Fills in the status from the extension's current state. What's stored only records whether the subscription
    /// was turned off; the extension can stop or fail without the subscription ever being touched, so whether it
    /// actually works is worked out each time it's read.
    pub fn with_extension_health(mut self, api_state: &ApiState) -> Self {
        if self.status == Status::Disabled {
            return self;
        }

        let Some(extension) = api_state
            .extensions
            .get(&self.extension_id)
            .map(|extension| extension.value().clone())
        else {
            self.status = Status::Error;
            self.status_reason = Some(StatusReason {
                reason: StatusReasonType::NotFound,
                description: format!("Extension '{}' isn't installed", self.extension_id),
            });
            return self;
        };

        if extension.state == extensions::State::Running {
            self.status = Status::Active;
            self.status_reason = None;
            return self;
        }

        let mut description = match extension.state {
            extensions::State::Failed => format!("Extension '{}' has failed", self.extension_id),
            state => format!("Extension '{}' is {state}", self.extension_id),
        };
        if !extension.state_reason.is_empty() {
            description = format!(
                "{description}: {}",
                lowercase_first(&extension.state_reason)
            );
        }

        self.status = Status::Error;
        self.status_reason = Some(StatusReason {
            reason: StatusReasonType::Failed,
            description,
        });
        self
    }

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

impl TryFrom<storage::extension_subscriptions::ExtensionSubscription> for Subscription {
    type Error = anyhow::Error;

    fn try_from(value: storage::extension_subscriptions::ExtensionSubscription) -> Result<Self> {
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

impl TryFrom<Subscription> for storage::extension_subscriptions::ExtensionSubscription {
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

/// List all subscriptions.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions",
    tags = ["Subscriptions"],
)]
pub async fn list_subscriptions(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<SubscriptionPathArgsRoot>,
) -> Result<HttpResponseOk<ListSubscriptionsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Subscriptions,
                ),
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    ensure_pipeline_exists(
        &mut conn,
        &rqctx.request_id,
        &path.namespace_id,
        &path.pipeline_id,
    )
    .await?;

    let storage_subscriptions = match storage::extension_subscriptions::list_by_pipeline(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
    )
    .await
    {
        Ok(subscriptions) => subscriptions,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut subscriptions: Vec<Subscription> = vec![];

    for storage_subscription in storage_subscriptions {
        let subscription = Subscription::try_from(storage_subscription).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        subscriptions.push(subscription.with_extension_health(api_state));
    }

    let resp = ListSubscriptionsResponse { subscriptions };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSubscriptionResponse {
    /// The metadata for the subscription.
    pub subscription: Subscription,
}

/// Get subscription by id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id}",
    tags = ["Subscriptions"],
)]
pub async fn get_subscription(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<SubscriptionPathArgs>,
) -> Result<HttpResponseOk<GetSubscriptionResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Subscriptions,
                ),
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_subscription_metadata = match storage::extension_subscriptions::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        &path.extension_id,
        &path.subscription_id,
    )
    .await
    {
        Ok(subscription) => subscription,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get metadata object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let subscription = Subscription::try_from(storage_subscription_metadata).map_err(|err| {
        http_error!(
            "Could not serialize object from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(err.into())
        )
    })?;

    let resp = GetSubscriptionResponse {
        subscription: subscription.with_extension_health(api_state),
    };
    Ok(HttpResponseOk(resp))
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

/// Update a subscription's state.
#[endpoint(
    method = PATCH,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id}",
    tags = ["Subscriptions"],
)]
pub async fn update_subscription(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<SubscriptionPathArgs>,
    body: TypedBody<UpdateSubscriptionRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Subscriptions,
                ),
                action: Action::Write,
            },
        )
        .await?;

    let Some(requested) = body.status else {
        return Err(HttpError::for_bad_request(
            None,
            "Update request is missing any changes".into(),
        ));
    };

    let status = match requested {
        UpdateSubscriptionStatus::Active => Status::Active,
        UpdateSubscriptionStatus::Disabled => Status::Disabled,
    };

    // The status change is only committed once the extension has been told, so a failure leaves both sides as they
    // were.
    let mut tx = match api_state.storage.open_tx().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let current = match storage::extension_subscriptions::get(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        &path.extension_id,
        &path.subscription_id,
    )
    .await
    {
        Ok(subscription) => subscription,
        Err(storage::StorageError::NotFound) => {
            return Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::NOT_FOUND,
                format!(
                    "pipeline '{}' has no subscription '{}' for extension '{}'",
                    path.pipeline_id, path.subscription_id, path.extension_id
                ),
            ));
        }
        Err(e) => {
            return Err(http_error!(
                "Could not get subscription from database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let current = Subscription::try_from(current).map_err(|e| {
        http_error!(
            "Could not parse subscription from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    if current.status == status {
        return Ok(HttpResponseUpdatedNoContent());
    }

    let updatable_fields = storage::extension_subscriptions::UpdatableFields {
        status: Some(status.to_string()),
        ..Default::default()
    };

    if let Err(e) = storage::extension_subscriptions::update(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        &path.extension_id,
        &path.subscription_id,
        updatable_fields,
    )
    .await
    {
        return Err(http_error!(
            "Could not update subscription in database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    // Extensions keep their own list of what they're subscribed to and fire from that, so changing the status here
    // only matters once the extension hears about it. An extension that isn't running doesn't need telling; it skips
    // disabled subscriptions when it loads them on startup.
    if api_state.running_extension(&path.extension_id).is_some() {
        match status {
            Status::Disabled => {
                if let Err(e) = unsubscribe_extension(
                    api_state,
                    &path.namespace_id,
                    &path.pipeline_id,
                    &path.extension_id,
                    &path.subscription_id,
                )
                .await
                {
                    return Err(http_error!(
                        "Could not unsubscribe from extension",
                        hyper::StatusCode::INTERNAL_SERVER_ERROR,
                        rqctx.request_id.clone(),
                        Some(e.into())
                    ));
                }
            }
            _ => subscribe_extension(api_state, &current)
                .await
                .map_err(|e| e.into_http_error(&path.extension_id, &rqctx.request_id))?,
        }
    }

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database connection",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseUpdatedNoContent())
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

/// Create a new subscription.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions",
    tags = ["Subscriptions"],
)]
pub async fn create_subscription(
    rqctx: RequestContext<Arc<ApiState>>,
    path: Path<SubscriptionPathArgsRoot>,
    body: TypedBody<CreateSubscriptionRequest>,
) -> Result<HttpResponseCreated<CreateSubscriptionResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Subscriptions,
                ),
                action: Action::Write,
            },
        )
        .await?;

    if let Err(e) = is_valid_identifier(&body.subscription_id) {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "'{}' is not a valid identifier; {}",
                body.subscription_id, e
            ),
        ));
    };

    let mut tx = match api_state.storage.open_tx().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let new_subscription =
        Subscription::from_create_request(&path.namespace_id, &path.pipeline_id, body.clone());

    let new_subscription_storage =
        new_subscription
            .clone()
            .try_into()
            .map_err(|err: anyhow::Error| {
                http_error!(
                    "Could not serialize new object",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(err.into())
                )
            })?;

    if let Err(e) =
        storage::pipeline_metadata::get(&mut tx, &path.namespace_id, &path.pipeline_id).await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_client_error(
                    None,
                    ClientErrorStatusCode::NOT_FOUND,
                    "could not find pipeline".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = storage::extension_registrations::get(&mut tx, &body.extension_id).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_client_error(
                    None,
                    ClientErrorStatusCode::NOT_FOUND,
                    "could not find extension".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not get registration object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    subscribe_extension(api_state, &new_subscription)
        .await
        .map_err(|e| e.into_http_error(&body.extension_id, &rqctx.request_id))?;

    if let Err(e) =
        storage::extension_subscriptions::insert(&mut tx, &new_subscription_storage).await
    {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    ClientErrorStatusCode::CONFLICT,
                    "subscription entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert subscription to database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database connection",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    api_state.event_bus.clone().publish(
        event_utils::Kind::PipelineExtensionSubscriptionRegistered {
            namespace_id: path.namespace_id.clone(),
            pipeline_id: path.pipeline_id.clone(),
            extension_id: body.extension_id.clone(),
            subscription_id: new_subscription.subscription_id.clone(),
        },
    );

    let resp = CreateSubscriptionResponse {
        subscription: new_subscription,
    };

    Ok(HttpResponseCreated(resp))
}

/// Delete subscription by id.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id}",
    tags = ["Subscriptions"],
)]
pub async fn delete_subscription(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<SubscriptionPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                allow_anonymous: false,
                requires: Requirement::pipeline(
                    &path.namespace_id,
                    &path.pipeline_id,
                    NamespaceResource::Subscriptions,
                ),
                action: Action::Delete,
            },
        )
        .await?;

    let mut tx = match api_state.storage.open_tx().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::extension_subscriptions::delete(
        &mut tx,
        &path.namespace_id,
        &path.pipeline_id,
        &path.extension_id,
        &path.subscription_id,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "subscription for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = unsubscribe_extension(
        api_state,
        &path.namespace_id,
        &path.pipeline_id,
        &path.extension_id,
        &path.subscription_id,
    )
    .await
    {
        return Err(http_error!(
            "Could not unsubscribe from pipeline",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}

/// Contacts extension to remove pipeline subscription
async fn unsubscribe_extension(
    api_state: &Arc<ApiState>,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    subscription_id: &str,
) -> Result<()> {
    let extension = api_state.running_extension(extension_id).ok_or(anyhow!(
        "could not find a running extension '{extension_id}'"
    ))?;

    let client = extensions::new_extension_client(
        &extension.url,
        &extension.secret,
        api_state.config.extensions.verify_certs,
    )
    .context("Could not establish client while attempting to unsubscribe")?;

    if let Err(e) = client
        .unsubscribe(&gofer_sdk::extension::api::types::UnsubscriptionRequest {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            pipeline_subscription_id: subscription_id.into(),
        })
        .await
    {
        // Extensions built with older Go SDKs answer 201 instead of the 204 the spec says, which the client treats
        // as an error even though the unsubscribe worked.
        let succeeded = matches!(
            &e,
            gofer_sdk::extension::api::Error::UnexpectedResponse(response)
                if response.status().is_success()
        );

        if !succeeded {
            extensions::health::record_failed_call(api_state, &extension, &e);
            return Err(anyhow::Error::from(e).context("could not unsubscribe from extension"));
        }
    }

    debug!(
        namespace_id = namespace_id,
        pipeline_id = pipeline_id,
        extension_id = extension_id,
        subscription_id = subscription_id,
        "unsubscribed pipeline from extension"
    );

    Ok(())
}

/// Communicates with the extension container in order to appropriately make sure the extension is aware of the
/// pipeline.
#[derive(thiserror::Error, Debug)]
enum SubscribeError {
    /// The extension refused the subscription, usually because of its settings. The message is the extension's own
    /// explanation, written for the user, so it gets passed back to them.
    #[error("{0}")]
    Rejected(String),

    /// A setting references a secret or object that doesn't exist or isn't allowed in this namespace.
    #[error("{0}")]
    BadSettings(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl SubscribeError {
    /// The user's mistakes become 400s with the reason; anything else is ours and stays a 500.
    fn into_http_error(self, extension_id: &str, request_id: &str) -> HttpError {
        match self {
            SubscribeError::Rejected(message) => HttpError::for_bad_request(
                None,
                format!("extension '{extension_id}' rejected the subscription; {message}"),
            ),
            SubscribeError::BadSettings(message) => HttpError::for_bad_request(
                None,
                format!("invalid subscription settings; {message}"),
            ),
            SubscribeError::Internal(e) => http_error!(
                "Could not subscribe to extension",
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                request_id.to_string(),
                Some(e.into())
            ),
        }
    }
}

async fn subscribe_extension(
    api_state: &Arc<ApiState>,
    subscription: &Subscription,
) -> Result<(), SubscribeError> {
    let extension = api_state
        .running_extension(&subscription.extension_id)
        .ok_or(anyhow!(
            "could not find a running extension '{}'",
            subscription.extension_id
        ))?;

    let client = extensions::new_extension_client(
        &extension.url,
        &extension.secret,
        api_state.config.extensions.verify_certs,
    )
    .context("Could not establish client while attempting to unsubscribe")?;

    let settings = match interpolate_vars(
        &api_state.storage,
        api_state.secret_store.as_ref(),
        api_state.object_store.as_ref(),
        &subscription.namespace_id,
        &subscription.pipeline_id,
        None,
        &subscription
            .settings
            .clone()
            .into_iter()
            .map(|(key, value)| Variable {
                key,
                value,
                source: VariableSource::PipelineConfig,
            })
            .collect(),
    )
    .await
    {
        Ok(settings) => settings,
        Err(e) => {
            if let Some(bad_reference) = e.downcast_ref::<BadReference>() {
                return Err(SubscribeError::BadSettings(bad_reference.to_string()));
            }

            return Err(e
                .context("Could not interpolate subscription settings while attempting to subscribe to extension")
                .into());
        }
    };

    let settings = settings
        .into_iter()
        .map(|variable| (variable.key, variable.value))
        .collect();

    if let Err(e) = client
        .subscribe(&gofer_sdk::extension::api::types::SubscriptionRequest {
            namespace_id: subscription.namespace_id.clone(),
            pipeline_id: subscription.pipeline_id.clone(),
            pipeline_subscription_id: subscription.subscription_id.clone(),
            pipeline_subscription_params: settings,
        })
        .await
    {
        extensions::health::record_failed_call(api_state, &extension, &e);

        // A 4xx means the extension looked at the request and said no, which is the user's to fix.
        if let gofer_sdk::extension::api::Error::ErrorResponse(response) = &e
            && response.status().is_client_error()
        {
            return Err(SubscribeError::Rejected(response.message.clone()));
        }

        return Err(anyhow::Error::from(e).into());
    }

    debug!(
        namespace_id = subscription.namespace_id,
        pipeline_id = subscription.pipeline_id,
        extension_id = subscription.extension_id,
        subscription_id = subscription.subscription_id,
        "subscribed pipeline to extension"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::lowercase_first;

    #[test]
    fn lowercase_first_leaves_acronyms_alone() {
        assert_eq!(lowercase_first("Not installed"), "not installed");
        assert_eq!(
            lowercase_first("HTTP 500 from extension"),
            "HTTP 500 from extension"
        );
        assert_eq!(lowercase_first("already lower"), "already lower");
        assert_eq!(lowercase_first("X"), "X");
        assert_eq!(lowercase_first(""), "");
    }
}
