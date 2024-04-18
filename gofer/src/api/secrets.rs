use crate::{
    api::{epoch_milli, ApiState, PreflightOptions},
    http_error, secret_store, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk, Path, Query,
    RequestContext, TypedBody,
};
use futures::TryFutureExt;
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

pub fn pipeline_secret_store_key(namespace_id: &str, pipeline_id: &str, key: &str) -> String {
    format!("{namespace_id}_{pipeline_id}_{key}")
}

pub fn global_secret_store_key(key: &str) -> String {
    format!("global_secret_{key}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineSecretPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineSecretPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target secret.
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GlobalSecretPathArgs {
    /// The unique identifier for the target secret.
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Secret {
    /// The identifier for the secret value.
    pub key: String,

    /// The namespaces this secret is allowed to be accessed from. Accepts regexes.
    pub namespaces: Vec<String>,

    /// Time in epoch milliseconds that this secret was registered.
    pub created: u128,
}

impl Secret {
    pub fn new(key: &str, namespaces: Vec<String>) -> Self {
        Secret {
            key: key.into(),
            namespaces,
            created: epoch_milli(),
        }
    }

    fn to_pipeline_secret_storage(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
    ) -> Result<storage::secret_store_pipeline_keys::SecretStorePipelineKey> {
        Ok(
            storage::secret_store_pipeline_keys::SecretStorePipelineKey {
                namespace_id: namespace_id.into(),
                pipeline_id: pipeline_id.into(),
                key: self.key.clone(),
                created: self.created.to_string(),
            },
        )
    }

    /// Checks the secret key's namespace list to confirm it actually does match a given namespace.
    /// It loops through the namespaces list and tries to evaluate regexp when it can.
    pub fn is_allowed_namespace(&self, namespace_id: &str) -> bool {
        for namespace_filter_str in &self.namespaces {
            if namespace_filter_str.is_empty() {
                continue;
            }

            // Check if the string is a valid regex
            let namespace_regex = match regex::Regex::new(namespace_filter_str) {
                Ok(val) => val,
                Err(e) => {
                    debug!(error = %e, "Could not parse namespace filter during is_allowed_namespace check");
                    continue;
                }
            };

            if namespace_regex.is_match(namespace_id) {
                return true;
            }

            continue;
        }

        false
    }
}

impl TryFrom<storage::secret_store_global_keys::SecretStoreGlobalKey> for Secret {
    type Error = anyhow::Error;

    fn try_from(value: storage::secret_store_global_keys::SecretStoreGlobalKey) -> Result<Self> {
        let created = value.created.parse::<u128>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let namespaces = serde_json::from_str(&value.namespaces).with_context(|| {
            format!(
                "Could not parse field 'namespaces' from storage value '{}'",
                value.namespaces
            )
        })?;

        Ok(Secret {
            key: value.key,
            namespaces,
            created,
        })
    }
}

impl TryFrom<Secret> for storage::secret_store_global_keys::SecretStoreGlobalKey {
    type Error = anyhow::Error;

    fn try_from(value: Secret) -> Result<Self> {
        let namespaces = serde_json::to_string(&value.namespaces).with_context(|| {
            format!(
                "Could not serialize field 'namespaces' into value '{:#?}'",
                value.namespaces
            )
        })?;

        Ok(Self {
            key: value.key,
            namespaces,
            created: value.created.to_string(),
        })
    }
}

impl TryFrom<storage::secret_store_pipeline_keys::SecretStorePipelineKey> for Secret {
    type Error = anyhow::Error;

    fn try_from(
        value: storage::secret_store_pipeline_keys::SecretStorePipelineKey,
    ) -> Result<Self> {
        let created = value.created.parse::<u128>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Secret {
            key: value.key,
            namespaces: vec![value.namespace_id],
            created,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListGlobalSecretsResponse {
    /// A list of all global secrets.
    pub secrets: Vec<Secret>,
}

/// List all global secrets.
///
/// Management tokens required.
#[endpoint(
    method = GET,
    path = "/api/secrets/global",
    tags = ["Secrets"],
)]
pub async fn list_global_secrets(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<ListGlobalSecretsResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
                management_only: true,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_secrets = match storage::secret_store_global_keys::list(&mut conn).await {
        Ok(secrets) => secrets,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut secrets: Vec<Secret> = vec![];

    for storage_secret in storage_secrets {
        let secret = Secret::try_from(storage_secret).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        secrets.push(secret);
    }

    let resp = ListGlobalSecretsResponse { secrets };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetGlobalSecretQueryArgs {
    /// Includes the actual plaintext secret in the response.
    pub include_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetGlobalSecretResponse {
    /// The target secret metadata.
    pub metadata: Secret,

    /// The actual secret, only included if "include_secret" param is true.
    pub secret: Option<String>,
}

/// Get global secret by key.
///
/// Management token required.
#[endpoint(
    method = GET,
    path = "/api/secrets/global/{key}",
    tags = ["Secrets"],
)]
pub async fn get_global_secret(
    rqctx: RequestContext<Arc<ApiState>>,
    query_params: Query<GetGlobalSecretQueryArgs>,
    path_params: Path<GlobalSecretPathArgs>,
) -> Result<HttpResponseOk<GetGlobalSecretResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
                management_only: true,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_secret = match storage::secret_store_global_keys::get(&mut conn, &path.key).await {
        Ok(secret) => secret,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, "".into()));
            }
            _ => {
                error!(message = "Could not get secret from database", error = %e);
                return Err(HttpError::for_internal_error(format!(
                    "Could not get secret from database; {:#?}",
                    e
                )));
            }
        },
    };

    let metadata = Secret::try_from(storage_secret).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let mut resp = GetGlobalSecretResponse {
        metadata,
        secret: None,
    };

    if query.include_secret {
        let secret_value = api_state
            .secret_store
            .get(&global_secret_store_key(&path.key))
            .map_err(|err| {
                if err == secret_store::SecretStoreError::NotFound {
                    return HttpError::for_bad_request(None, "Secret not found".into());
                };

                http_error!(
                    "Could not get objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(err.into())
                )
            })
            .await?;

        resp.secret = Some(String::from_utf8_lossy(&secret_value.0).to_string())
    }

    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutGlobalSecretRequest {
    /// The name for the secret you would like to store.
    pub key: String,

    /// The actual plaintext secret.
    pub content: String,

    /// The namespaces you want this secret to be accessible by. Accepts Regexes.
    pub namespaces: Vec<String>,

    /// Overwrite a value of a secret if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutGlobalSecretResponse {
    /// Information about the secret created.
    pub secret: Secret,
}

/// Insert a new secret into the global secret store.
///
/// This route is only accessible for management tokens.
#[endpoint(
    method = POST,
    path = "/api/secrets/global",
    tags = ["Secrets"],
)]
pub async fn put_global_secret(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<PutGlobalSecretRequest>,
) -> Result<HttpResponseCreated<PutGlobalSecretResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
                management_only: true,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let new_secret = Secret::new(&body.key, body.namespaces);

    let new_secret_storage = match new_secret.clone().try_into() {
        Ok(secret) => secret,
        Err(e) => {
            return Err(http_error!(
                "Could not serialize secret into database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(anyhow::anyhow!("{}", e).into())
            ));
        }
    };

    if let Err(e) = storage::secret_store_global_keys::insert(&mut conn, &new_secret_storage).await
    {
        match e {
            storage::StorageError::Exists => {
                if !body.force {
                    return Err(HttpError::for_client_error(
                        None,
                        StatusCode::CONFLICT,
                        "secret entry already exists".into(),
                    ));
                } else {
                    unimplemented!()
                    //TODO(implement force)
                }
            }
            _ => {
                return Err(http_error!(
                    "Could not insert secret into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = api_state
        .secret_store
        .put(
            &global_secret_store_key(&body.key),
            body.content.as_bytes().to_vec(),
            body.force,
        )
        .await
    {
        return Err(http_error!(
            "Could not insert secret into store",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    let resp = PutGlobalSecretResponse { secret: new_secret };

    Ok(HttpResponseCreated(resp))
}

/// Delete global secret by key.
///
/// This route is only accessible for management tokens.
#[endpoint(
    method = DELETE,
    path = "/api/secrets/global/{key}",
    tags = ["Secrets"],
)]
pub async fn delete_global_secret(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<GlobalSecretPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
                management_only: true,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::secret_store_global_keys::delete(&mut conn, &path.key).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "secret for key given does not exist".into(),
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

    if let Err(e) = api_state
        .secret_store
        .delete(&global_secret_store_key(&path.key))
        .await
    {
        return Err(http_error!(
            "Could not delete object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelineSecretsResponse {
    /// A list of all pipeline secrets.
    pub secrets: Vec<Secret>,
}

/// List all pipeline secrets.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets",
    tags = ["Secrets"],
)]
pub async fn list_pipeline_secrets(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineSecretPathArgsRoot>,
) -> Result<HttpResponseOk<ListPipelineSecretsResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_secrets = match storage::secret_store_pipeline_keys::list(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
    )
    .await
    {
        Ok(secrets) => secrets,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut secrets: Vec<Secret> = vec![];

    for storage_secret in storage_secrets {
        let secret = Secret::try_from(storage_secret).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        secrets.push(secret);
    }

    let resp = ListPipelineSecretsResponse { secrets };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineSecretQueryArgs {
    /// Includes the actual plaintext secret in the response.
    pub include_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineSecretResponse {
    /// The target secret metadata.
    pub metadata: Secret,

    /// The actual secret, only included if "include_secret" param is true.
    pub secret: Option<String>,
}

/// Get pipeline secret by key.
#[endpoint(
    method = GET,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}",
    tags = ["Secrets"],
)]
pub async fn get_pipeline_secret(
    rqctx: RequestContext<Arc<ApiState>>,
    query_params: Query<GetPipelineSecretQueryArgs>,
    path_params: Path<PipelineSecretPathArgs>,
) -> Result<HttpResponseOk<GetPipelineSecretResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let query = query_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let storage_secret = match storage::secret_store_pipeline_keys::get(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        &path.key,
    )
    .await
    {
        Ok(secret) => secret,
        Err(e) => match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(None, "".into()));
            }
            _ => {
                return Err(http_error!(
                    "Could not get objects from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        },
    };

    let metadata = Secret::try_from(storage_secret).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let mut resp = GetPipelineSecretResponse {
        metadata,
        secret: None,
    };

    if query.include_secret {
        let secret_value = api_state
            .secret_store
            .get(&pipeline_secret_store_key(
                &path.namespace_id,
                &path.pipeline_id,
                &path.key,
            ))
            .map_err(|err| {
                if err == secret_store::SecretStoreError::NotFound {
                    return HttpError::for_bad_request(None, "Secret not found".into());
                };

                http_error!(
                    "Could not get object from store",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(err.into())
                )
            })
            .await?;

        resp.secret = Some(String::from_utf8_lossy(&secret_value.0).to_string())
    }

    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineSecretRequest {
    /// The name for the secret you would like to store.
    pub key: String,

    /// The actual plaintext secret.
    pub content: String,

    /// Overwrite a value of a secret if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineSecretResponse {
    /// Information about the secret created.
    pub secret: Secret,
}

/// Insert a new secret into the pipeline secret store.
#[endpoint(
    method = POST,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets",
    tags = ["Secrets"],
)]
pub async fn put_pipeline_secret(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineSecretPathArgsRoot>,
    body: TypedBody<PutPipelineSecretRequest>,
) -> Result<HttpResponseCreated<PutPipelineSecretResponse>, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    let new_secret = Secret::new(&body.key, vec![]);

    let new_secret_storage =
        match new_secret.to_pipeline_secret_storage(&path.namespace_id, &path.pipeline_id) {
            Ok(secret) => secret,
            Err(e) => {
                return Err(http_error!(
                    "Could not serialize secret object to database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        };

    if let Err(e) =
        storage::secret_store_pipeline_keys::insert(&mut conn, &new_secret_storage).await
    {
        match e {
            storage::StorageError::Exists => {
                if !body.force {
                    return Err(HttpError::for_client_error(
                        None,
                        StatusCode::CONFLICT,
                        "secret entry already exists".into(),
                    ));
                } else {
                    unimplemented!()
                    //TODO(implement force)
                }
            }
            _ => {
                return Err(http_error!(
                    "Could not insert object into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = api_state
        .secret_store
        .put(
            &pipeline_secret_store_key(&path.namespace_id, &path.pipeline_id, &body.key),
            body.content.as_bytes().to_vec(),
            body.force,
        )
        .await
    {
        return Err(http_error!(
            "Could not insert objects from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    let resp = PutPipelineSecretResponse { secret: new_secret };

    Ok(HttpResponseCreated(resp))
}

/// Delete pipeline secret by key.
#[endpoint(
    method = DELETE,
    path = "/api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key}",
    tags = ["Secrets"],
)]
pub async fn delete_pipeline_secret(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<PipelineSecretPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: Some(path.namespace_id.clone()),
                management_only: false,
            },
        )
        .await?;

    let mut conn = match api_state.storage.conn().await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(http_error!(
                "Could not open connection to database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id,
                Some(e.into())
            ));
        }
    };

    if let Err(e) = storage::secret_store_pipeline_keys::delete(
        &mut conn,
        &path.namespace_id,
        &path.pipeline_id,
        &path.key,
    )
    .await
    {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "secret for key given does not exist".into(),
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

    if let Err(e) = api_state
        .secret_store
        .delete(&pipeline_secret_store_key(
            &path.namespace_id,
            &path.pipeline_id,
            &path.key,
        ))
        .await
    {
        return Err(http_error!(
            "Could not delete object from store",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    Ok(HttpResponseDeleted())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::exact_match("my_namespace", vec!["my_namespace".into()], true)]
    #[case::exact_mismatch("my_namespace", vec!["another_namespace".into()], false)]
    // A paranoid test to make sure we don't get regex matches that simply match on the first part of the namespace.
    #[case::mismatch_substring("my_namespace", vec!["my_namespacee".into()], false)]
    #[case::match_all_namespaces_with_variable_num("test123", vec!["^test\\d+$".into()], true)]
    #[case::regex_mismatch("test123", vec!["^test\\d{4}$".into()], false)]
    #[case::regex_invalid_pattern("namespace", vec!["[".into()], false)]
    #[case::match_everything("my_namespace", vec![".*".into()], true)]
    #[case::match_nothing("my_namespace", vec!["".into()], false)]
    fn test_is_allowed_namespace(
        #[case] namespace: &str,
        #[case] allowed_namespaces: Vec<String>,
        #[case] expected_result: bool,
    ) {
        let secret_store_key = Secret {
            key: "test".into(),
            created: 0,
            namespaces: allowed_namespaces,
        };
        assert_eq!(
            secret_store_key.is_allowed_namespace(namespace),
            expected_result
        );
    }
}
