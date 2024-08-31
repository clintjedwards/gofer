use super::permissioning::{Action, Resource};
use crate::{
    api::{epoch_milli, event_utils, is_valid_identifier, ApiState, PreflightOptions},
    http_error, storage,
};
use anyhow::{bail, Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk, Path,
    RequestContext, TypedBody,
};
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Acquire;
use std::sync::Arc;
use tracing::error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NamespacePathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,
}

/// A namespace represents a grouping of pipelines. Normally it is used to divide teams or logically different
/// sections of workloads. It is the highest level unit as it sits above pipelines in the hierarchy of Gofer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Namespace {
    /// Unique identifier for the namespace.
    pub id: String,

    /// Humanized name for the namespace.
    pub name: String,

    /// Short description about what the namespace is used for.
    pub description: String,

    /// Time in epoch milliseconds when namespace was created.
    pub created: u64,

    /// Time in epoch milliseconds when namespace would expire.
    pub modified: u64,
}

impl Namespace {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Namespace {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            created: epoch_milli(),
            modified: 0,
        }
    }
}

impl TryFrom<storage::namespaces::Namespace> for Namespace {
    type Error = anyhow::Error;

    fn try_from(value: storage::namespaces::Namespace) -> Result<Self> {
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

        Ok(Namespace {
            id: value.id,
            name: value.name,
            description: value.description,
            created,
            modified,
        })
    }
}

impl From<Namespace> for storage::namespaces::Namespace {
    fn from(value: Namespace) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            created: value.created.to_string(),
            modified: value.modified.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListNamespacesResponse {
    /// A list of all namespaces.
    pub namespaces: Vec<Namespace>,
}

/// List all namespaces.
#[endpoint(
    method = GET,
    path = "/api/namespaces",
    tags = ["Namespaces"],
)]
pub async fn list_namespaces(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<ListNamespacesResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![Resource::Namespaces("".into())],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let storage_namespaces = match storage::namespaces::list(&mut conn).await {
        Ok(namespaces) => namespaces,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut namespaces: Vec<Namespace> = vec![];

    for storage_namespace in storage_namespaces {
        let namespace = Namespace::try_from(storage_namespace).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        namespaces.push(namespace);
    }

    let resp = ListNamespacesResponse { namespaces };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetNamespaceResponse {
    /// The target namespace.
    pub namespace: Namespace,
}

/// Get api namespace by id.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}",
    tags = ["Namespaces"],
)]
pub async fn get_namespace(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<NamespacePathArgs>,
) -> Result<HttpResponseOk<GetNamespaceResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![Resource::Namespaces(path.namespace_id.clone())],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let storage_namespace = match storage::namespaces::get(&mut conn, &path.namespace_id).await {
        Ok(namespace) => namespace,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, String::new()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let namespace = Namespace::try_from(storage_namespace).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = GetNamespaceResponse { namespace };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateNamespaceRequest {
    /// The unique identifier for the namespace. Only accepts alphanumeric chars with hyphens. No spaces.
    pub id: String,

    /// Humanized name for the namespace.
    pub name: String,

    /// Short description about what the namespace is used for.
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateNamespaceResponse {
    /// Information about the namespace created.
    pub namespace: Namespace,
}

/// Create a new namespace.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = POST,
    path = "/api/namespaces",
    tags = ["Namespaces"],
)]
pub async fn create_namespace(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<CreateNamespaceRequest>,
) -> Result<HttpResponseCreated<CreateNamespaceResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Namespaces("".into())],
                action: Action::Write,
            },
        )
        .await?;

    if let Err(e) = is_valid_identifier(&body.id) {
        return Err(HttpError::for_bad_request(
            None,
            format!(
                "'{}' is not a valid identifier; {}",
                &body.id,
                &e.to_string()
            ),
        ));
    };

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let new_namespace = Namespace::new(&body.id, &body.name, &body.description);

    let new_namespace_storage = new_namespace.clone().into();

    if let Err(e) = storage::namespaces::insert(&mut conn, &new_namespace_storage).await {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "namespace entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert objects into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::CreatedNamespace {
            namespace_id: new_namespace.id.clone(),
        });

    let resp = CreateNamespaceResponse {
        namespace: new_namespace,
    };

    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateNamespaceRequest {
    /// Humanized name for the namespace.
    pub name: Option<String>,

    /// Short description about what the namespace is used for.
    pub description: Option<String>,
}

impl From<UpdateNamespaceRequest> for storage::namespaces::UpdatableFields {
    fn from(value: UpdateNamespaceRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            modified: epoch_milli().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateNamespaceResponse {
    /// Information about the namespace updated.
    pub namespace: Namespace,
}

/// Update a namespace's details.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = PATCH,
    path = "/api/namespaces/{namespace_id}",
    tags = ["Namespaces"],
)]
pub async fn update_namespace(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<NamespacePathArgs>,
    body: TypedBody<UpdateNamespaceRequest>,
) -> Result<HttpResponseOk<UpdateNamespaceResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Namespaces(path.namespace_id.clone())],
                action: Action::Write,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let updatable_fields = storage::namespaces::UpdatableFields::from(body.clone());

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open transaction to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::namespaces::update(&mut tx, &path.namespace_id, updatable_fields).await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Namespace entry for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not update object in database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    let storage_namespace = match storage::namespaces::get(&mut tx, &path.namespace_id).await {
        Ok(namespace) => namespace,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Namespace for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not get object in database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    if let Err(e) = tx.commit().await {
        error!(message = "Could not close transaction from database", error = %e);
        return Err(HttpError::for_internal_error(format!(
            "Encountered error when attempting to write namespace to database; {:#?}",
            e
        )));
    };

    let namespace = Namespace::try_from(storage_namespace).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = UpdateNamespaceResponse { namespace };

    Ok(HttpResponseOk(resp))
}

/// Delete api namespace by id.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}",
    tags = ["Namespaces"],
)]
pub async fn delete_namespace(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<NamespacePathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Namespaces(path.namespace_id.clone())],
                action: Action::Delete,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::namespaces::delete(&mut conn, &path.namespace_id).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "namespace for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::DeletedNamespace {
            namespace_id: path.namespace_id.clone(),
        });

    Ok(HttpResponseDeleted())
}

/// Creates the default namespace for Gofer. It is safe to call this even if the namespace has been already created.
pub async fn create_default_namespace(api_state: Arc<ApiState>) -> Result<()> {
    let default_namespace = Namespace::new(
        "default",
        "Default",
        "The original namespace created automatically by the Gofer system.",
    );

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(message = "Could not open connection to database", error = %e);
            bail!("Could not open connection to database")
        }
    };

    if let Err(e) = storage::namespaces::insert(&mut conn, &default_namespace.clone().into()).await
    {
        match e {
            storage::StorageError::Exists => {
                return Ok(());
            }
            _ => {
                bail!("{e}")
            }
        }
    }

    api_state
        .event_bus
        .clone()
        .publish(event_utils::Kind::CreatedNamespace {
            namespace_id: default_namespace.id,
        });

    Ok(())
}
