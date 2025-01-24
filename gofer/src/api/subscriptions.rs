use super::permissioning::{Action, Resource};
use crate::{
    api::{
        event_utils, extensions, interpolate_vars, is_valid_identifier, ApiState, PreflightOptions,
        Variable, VariableSource,
    },
    http_error, storage,
};
use anyhow::{anyhow, Context, Result};
use dropshot::{
    endpoint, ClientErrorStatusCode, HttpError, HttpResponseCreated, HttpResponseDeleted,
    HttpResponseOk, HttpResponseUpdatedNoContent, Path, RequestContext, TypedBody,
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
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Subscriptions,
                ],
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

        subscriptions.push(subscription);
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
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Subscriptions,
                ],
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

    let resp = GetSubscriptionResponse { subscription };
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
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Subscriptions,
                ],
                action: Action::Write,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn().await {
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

    if body.status.is_none() {
        return Err(HttpError::for_bad_request(
            None,
            "Update request is missing any changes".into(),
        ));
    };

    let status = match body.status.unwrap() {
        UpdateSubscriptionStatus::Active => Status::Active,
        UpdateSubscriptionStatus::Disabled => Status::Disabled,
    };

    let updatable_fields = storage::extension_subscriptions::UpdatableFields {
        status: Some(status.to_string()),
        ..Default::default()
    };

    if let Err(e) = storage::extension_subscriptions::update(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        &path.extension_id,
        &path.subscription_id,
        updatable_fields,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Subscription entry for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could insert object into database",
                    hyper::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(e.into())
                ));
            }
        }
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
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Subscriptions,
                ],
                action: Action::Write,
            },
        )
        .await?;

    if let Err(e) = is_valid_identifier(&body.subscription_id) {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "'{}' is not a valid identifier; {}",
                &body.subscription_id,
                &e.to_string()
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

    if let Err(e) = subscribe_extension(api_state, &new_subscription).await {
        return Err(http_error!(
            "Could not subscribe to extension",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

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
                resources: vec![
                    Resource::Namespaces(path.namespace_id.clone()),
                    Resource::Pipelines(path.pipeline_id.clone()),
                    Resource::Subscriptions,
                ],
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
    api_state: &ApiState,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    subscription_id: &str,
) -> Result<()> {
    let extension = api_state
        .extensions
        .get(extension_id)
        .ok_or(anyhow!("could not find extension"))?;
    let extension = extension.value();

    let client = extensions::new_extension_client(
        &extension.url,
        &extension.secret,
        api_state.config.extensions.verify_certs,
    )
    .context("Could not establish client while attempting to unsubscribe")?;

    client
        .unsubscribe(&gofer_sdk::extension::api::types::UnsubscriptionRequest {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            pipeline_subscription_id: subscription_id.into(),
        })
        .await
        .context("could not unsubscribe from extension")?;

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
async fn subscribe_extension(api_state: &ApiState, subscription: &Subscription) -> Result<()> {
    let extension = api_state
        .extensions
        .get(&subscription.extension_id)
        .ok_or(anyhow!("could not find extension"))?;
    let extension = extension.value();

    let client = extensions::new_extension_client(
        &extension.url,
        &extension.secret,
        api_state.config.extensions.verify_certs,
    )
    .context("Could not establish client while attempting to unsubscribe")?;

    let settings = interpolate_vars(
        api_state,
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
    .await.context("Could not interpolate variables within pipeline config while attempting to subscribe to extension")?;

    let settings = settings
        .into_iter()
        .map(|variable| (variable.key, variable.value))
        .collect();

    client
        .subscribe(&gofer_sdk::extension::api::types::SubscriptionRequest {
            namespace_id: subscription.namespace_id.clone(),
            pipeline_id: subscription.pipeline_id.clone(),
            pipeline_subscription_id: subscription.subscription_id.clone(),
            pipeline_subscription_params: settings,
        })
        .await?;

    debug!(
        namespace_id = subscription.namespace_id,
        pipeline_id = subscription.pipeline_id,
        extension_id = subscription.extension_id,
        subscription_id = subscription.subscription_id,
        "subscribed pipeline to extension"
    );

    Ok(())
}
