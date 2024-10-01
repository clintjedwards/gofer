use super::permissioning::{Action, Resource, SystemRoles};
use crate::{
    api::{epoch_milli, ApiState, PreflightOptions},
    http_error, storage,
};
use anyhow::{Context, Result};
use dropshot::{
    endpoint, HttpError, HttpResponseCreated, HttpResponseDeleted, HttpResponseOk,
    HttpResponseUpdatedNoContent, Path, RequestContext, TypedBody,
};
use http::StatusCode;
use rand::{distributions::Alphanumeric, Rng};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, ops::Add, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TokenPathArgs {
    /// The unique identifier for the target namespace.
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
/// Gofer API Token.
///
/// The hash field is skipped during serialization to prevent it from being exposed to the user.
/// This isn't a foolproof practice, but it'll work for now.
pub struct Token {
    /// Unique identifier for token.
    pub id: String,

    #[serde(skip)]
    /// The SHA256 hash for the token.
    pub hash: String,

    /// Time in epoch milliseconds when token was created.
    pub created: u64,

    /// Extra information about this token in label form
    pub metadata: HashMap<String, String>,

    /// Time in epoch milliseconds when token would expire.
    /// An expiry of 0 means that token does not expire.
    pub expires: u64,

    /// If the token is inactive or not; disabled tokens cannot be used for requests.
    pub disabled: bool,

    /// The user of the token in plaintext.
    pub user: String,

    /// The role ids for the current token.
    pub roles: Vec<String>,
}

fn generate_rand_str(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}

/// Creates a new secure token string and returns (token, hash)
pub fn create_new_api_token() -> (String, String) {
    let token = generate_rand_str(32);

    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    (token, hash)
}

impl Token {
    pub fn new(
        hash: &str,
        metadata: HashMap<String, String>,
        expiry: u64, // Seconds from creation that token should expire. 0 means that token does not expire.
        user: String,
        roles: Vec<String>,
    ) -> Self {
        let now = epoch_milli();
        let expires = if expiry > 0 {
            now.add(expiry * 1000)
        } else {
            0
        };

        Token {
            id: uuid::Uuid::now_v7().to_string(),
            hash: hash.into(),
            created: now,
            metadata,
            expires,
            disabled: false,
            user,
            roles,
        }
    }
}

impl TryFrom<storage::tokens::Token> for Token {
    type Error = anyhow::Error;

    fn try_from(value: storage::tokens::Token) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let expires = value.expires.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'expires' from storage value '{}'",
                value.expires
            )
        })?;

        let metadata: HashMap<String, String> = serde_json::from_str(&value.metadata)
            .with_context(|| {
                format!(
                    "Could not parse field 'metadata' from storage value '{}'",
                    value.metadata
                )
            })?;

        let roles: Vec<String> = serde_json::from_str(&value.roles).with_context(|| {
            format!(
                "Could not parse field 'roles' from storage value '{}'",
                value.roles
            )
        })?;

        Ok(Token {
            id: value.id,
            hash: value.hash,
            created,
            metadata,
            expires,
            disabled: value.disabled,
            user: value.user,
            roles,
        })
    }
}

impl TryFrom<Token> for storage::tokens::Token {
    type Error = anyhow::Error;

    fn try_from(value: Token) -> Result<Self> {
        let metadata = serde_json::to_string(&value.metadata).with_context(|| {
            format!(
                "Could not parse field 'metadata' from storage value; '{:#?}'",
                value.metadata
            )
        })?;

        let roles = serde_json::to_string(&value.roles).with_context(|| {
            format!(
                "Could not parse field 'roles' from storage value; '{:#?}'",
                value.roles
            )
        })?;

        Ok(Self {
            id: value.id,
            hash: value.hash,
            created: value.created.to_string(),
            metadata,
            expires: value.expires.to_string(),
            disabled: value.disabled,
            roles,
            user: value.user,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListTokensResponse {
    /// A list of all tokens.
    pub tokens: Vec<Token>,
}

/// List all Gofer API tokens.
///
/// This endpoint is restricted to admin tokens only.
#[endpoint(
    method = GET,
    path = "/api/tokens",
    tags = ["Tokens"],
)]
pub async fn list_tokens(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<ListTokensResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Tokens],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
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

    let storage_tokens = match storage::tokens::list(&mut conn).await {
        Ok(tokens) => tokens,
        Err(e) => {
            return Err(http_error!(
                "Could not get objects from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let mut tokens: Vec<Token> = vec![];

    for storage_token in storage_tokens {
        let token = Token::try_from(storage_token).map_err(|e| {
            http_error!(
                "Could not parse object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            )
        })?;

        tokens.push(token);
    }

    let resp = ListTokensResponse { tokens };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetTokenByIDResponse {
    /// The target token.
    pub token: Token,
}

/// Get api token by id.
#[endpoint(
    method = GET,
    path = "/api/tokens/{id}",
    tags = ["Tokens"]
)]
pub async fn get_token_by_id(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TokenPathArgs>,
) -> Result<HttpResponseOk<GetTokenByIDResponse>, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![Resource::Tokens],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
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

    let storage_token = match storage::tokens::get_by_id(&mut conn, &path.id).await {
        Ok(token) => token,
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

    let token = Token::try_from(storage_token).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = GetTokenByIDResponse { token };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhoAmIResponse {
    /// The target token.
    pub token: Token,
}

/// Get api token who made the request.
#[endpoint(
    method = GET,
    path = "/api/tokens/whoami",
    tags = ["Tokens"]
)]
pub async fn whoami(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseOk<WhoAmIResponse>, HttpError> {
    let api_state = rqctx.context();
    let req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: false,
                resources: vec![Resource::Tokens],
                action: Action::Read,
            },
        )
        .await?;

    let mut conn = match api_state.storage.read_conn().await {
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

    let storage_token =
        match storage::tokens::get_by_id(&mut conn, &req_metadata.auth.token_id).await {
            Ok(token) => token,
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

    let token = Token::try_from(storage_token).map_err(|e| {
        http_error!(
            "Could not parse object from database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        )
    })?;

    let resp = WhoAmIResponse { token };
    Ok(HttpResponseOk(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateTokenRequest {
    /// Various other bits of data you can attach to tokens. This is used by Gofer to track some details about tokens,
    /// but can also be used by users to attach bits of information that would make the token easier to programmatically
    /// manage.
    pub metadata: Option<HashMap<String, String>>,

    /// The amount of time the token is valid for in seconds. An expiry of 0 means that token does not expire.
    pub expires: u64,

    /// The plaintext username of the token user.
    pub user: String,

    /// The list of roles to apply to the token.
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateTokenResponse {
    /// Information about the token created.
    pub token_details: Token,

    /// The actual token created. API Tokens should be protected in the same fashion as passwords.
    pub secret: String,
}

/// Create a new token.
///
/// This endpoint is restricted to admin tokens only.
#[endpoint(
    method = POST,
    path = "/api/tokens",
    tags = ["Tokens"]
)]
pub async fn create_token(
    rqctx: RequestContext<Arc<ApiState>>,
    body: TypedBody<CreateTokenRequest>,
) -> Result<HttpResponseCreated<CreateTokenResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Tokens],
                action: Action::Write,
            },
        )
        .await?;

    let body = body.into_inner();

    let mut conn = match api_state.storage.write_conn().await {
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

    // Deny any attempt to assign the bootstrap role to other tokens.
    for role in &body.roles {
        if role.to_lowercase() == "bootstrap" {
            return Err(HttpError::for_client_error(
                None,
                StatusCode::UNAUTHORIZED,
                "cannot assign the bootstrap role to any other token".into(),
            ));
        };
    }

    let (token, hash) = create_new_api_token();

    let new_token = Token::new(
        &hash,
        body.metadata.unwrap_or_default(),
        body.expires,
        body.user,
        body.roles,
    );

    let new_token_storage = match new_token.clone().try_into() {
        Ok(token) => token,
        Err(e) => {
            return Err(http_error!(
                "Could not parse token into storage type while creating token",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(anyhow::anyhow!("{}", e).into())
            ));
        }
    };

    if let Err(e) = storage::tokens::insert(&mut conn, &new_token_storage).await {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "token entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert token into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    let resp = CreateTokenResponse {
        token_details: new_token,
        secret: token,
    };
    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeleteTokenResponse {
    /// Information about the token deleted.
    pub token_details: Token,
}

/// Delete api token by id.
///
/// This endpoint is restricted to admin tokens only.
#[endpoint(
    method = DELETE,
    path = "/api/tokens/{id}",
    tags = ["Tokens"],
)]
pub async fn delete_token(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TokenPathArgs>,
) -> Result<HttpResponseDeleted, HttpError> {
    let api_state = rqctx.context();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Tokens],
                action: Action::Delete,
            },
        )
        .await?;

    let mut conn = match api_state.storage.write_conn().await {
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

    if let Err(e) = storage::tokens::delete(&mut conn, &path.id).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "token for id given does not exist".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not delete object from database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id,
                    Some(e.into())
                ));
            }
        }
    };

    Ok(HttpResponseDeleted())
}

/// Create root admin token.
///
/// This endpoint can only be hit once and will create the root admin token,
/// from which all other tokens can be created.
#[endpoint(
    method = POST,
    path = "/api/tokens/bootstrap",
    tags = ["Tokens"]
)]
pub async fn create_bootstrap_token(
    rqctx: RequestContext<Arc<ApiState>>,
) -> Result<HttpResponseCreated<CreateTokenResponse>, HttpError> {
    let api_state = rqctx.context();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: true,
                admin_only: false,
                resources: vec![Resource::Tokens],
                action: Action::Write,
            },
        )
        .await?;

    let mut tx = match api_state.storage.open_tx().await {
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

    let system_parameters = match storage::system::get_system_parameters(&mut tx).await {
        Ok(system_parameters) => system_parameters,
        Err(e) => {
            return Err(http_error!(
                "Could not query database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if system_parameters.bootstrap_token_created {
        return Err(HttpError::for_client_error(
            None,
            StatusCode::CONFLICT,
            "Bootstrap token already exists".into(),
        ));
    }

    let (token, hash) = create_new_api_token();

    let new_token = Token::new(
        &hash,
        HashMap::from([("bootstrap_token".into(), "true".into())]),
        0, // Make token last forever
        "bootstrap".into(),
        vec![SystemRoles::Bootstrap.to_string()],
    );

    let new_token_storage = match new_token.clone().try_into() {
        Ok(token) => token,
        Err(e) => {
            return Err(http_error!(
                "Could not parse token into storage type while creating token",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(anyhow::anyhow!("{}", e).into())
            ));
        }
    };

    if let Err(e) = storage::tokens::insert(&mut tx, &new_token_storage).await {
        match e {
            storage::StorageError::Exists => {
                return Err(HttpError::for_client_error(
                    None,
                    StatusCode::CONFLICT,
                    "token entry already exists".into(),
                ));
            }
            _ => {
                return Err(http_error!(
                    "Could not insert token into database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    rqctx.request_id.clone(),
                    Some(e.into())
                ));
            }
        }
    };

    if let Err(e) = storage::system::update_system_parameters(&mut tx, Some(true), None).await {
        return Err(http_error!(
            "Could not update system_parameters into database",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    if let Err(e) = tx.commit().await {
        return Err(http_error!(
            "Could not close database transaction",
            http::StatusCode::INTERNAL_SERVER_ERROR,
            rqctx.request_id.clone(),
            Some(e.into())
        ));
    };

    let resp = CreateTokenResponse {
        token_details: new_token,
        secret: token,
    };

    Ok(HttpResponseCreated(resp))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTokenRequest {
    pub disabled: Option<bool>,
}

/// Update a token's state.
#[endpoint(
    method = PATCH,
    path = "/api/tokens/{id}",
    tags = ["Tokens"],
)]
pub async fn update_token(
    rqctx: RequestContext<Arc<ApiState>>,
    path_params: Path<TokenPathArgs>,
    body: TypedBody<UpdateTokenRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let body = body.into_inner();
    let path = path_params.into_inner();
    let _req_metadata = api_state
        .preflight_check(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                admin_only: true,
                resources: vec![Resource::Tokens],
                action: Action::Write,
            },
        )
        .await?;

    // Check that user isn't trying to make changes to their own token
    if path.id == _req_metadata.auth.token_id {
        return Err(HttpError::for_client_error(
            None,
            StatusCode::CONFLICT,
            "Cannot make state changes to your own token".into(),
        ));
    }

    let mut conn = match api_state.storage.write_conn().await {
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

    let updatable_fields = storage::tokens::UpdatableFields {
        disabled: body.disabled,
    };

    if let Err(e) = storage::tokens::update(&mut conn, &path.id, updatable_fields).await {
        match e {
            storage::StorageError::NotFound => {
                return Err(HttpError::for_not_found(
                    None,
                    "Token entry for id given does not exist".into(),
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

    Ok(HttpResponseUpdatedNoContent())
}
