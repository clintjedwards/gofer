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
use sqlx::Acquire;
use std::{
    collections::{HashMap, HashSet},
    ops::Add,
    str::FromStr,
    sync::Arc,
};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TokenPathArgs {
    /// The unique identifier for the target namespace.
    pub id: String,
}

#[derive(Debug, Clone, Display, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema)]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum TokenType {
    /// Admin token; has access to just about everything.
    Management,

    /// Only has read/write access to namespaces granted.
    User,

    /// Special token given to extensions. Has access to any namespace, but does not have management access.
    Extension,

    /// Gofer has a special function that allows users to autogenerate a token and inject it into their run
    /// such that they can use it easily during the run. Has same access properties the user token with a more focused
    /// namespace.
    Run,
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

    /// The type of token. Management tokens are essentially root.
    pub token_type: TokenType,

    /// List of namespaces this token has access to, strings in this list can be a regex
    pub namespaces: HashSet<String>,

    /// Extra information about this token in label form
    pub metadata: HashMap<String, String>,

    /// Time in epoch milliseconds when token would expire.
    pub expires: u64,

    /// If the token is inactive or not; disabled tokens cannot be used for requests.
    pub disabled: bool,

    /// The user of the token in plaintext.
    pub user: String,
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
        token_type: TokenType,
        namespaces: HashSet<String>,
        metadata: HashMap<String, String>,
        expiry: u64, // Seconds from creation that token should expire.
        user: String,
    ) -> Self {
        let now = epoch_milli();
        let expires = now.add(expiry * 1000);

        Token {
            id: uuid::Uuid::now_v7().to_string(),
            hash: hash.into(),
            created: now,
            token_type,
            namespaces,
            metadata,
            expires,
            disabled: false,
            user,
        }
    }
}

impl TryFrom<storage::tokens::Token> for Token {
    type Error = anyhow::Error;

    fn try_from(value: storage::tokens::Token) -> Result<Self> {
        let token_type = TokenType::from_str(&value.token_type).with_context(|| {
            format!(
                "Could not parse field 'token type' from storage value '{}'",
                value.token_type
            )
        })?;

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

        let namespaces: HashSet<String> =
            serde_json::from_str(&value.namespaces).with_context(|| {
                format!(
                    "Could not parse field 'namespaces' from storage value '{}'",
                    value.namespaces
                )
            })?;

        let metadata: HashMap<String, String> = serde_json::from_str(&value.metadata)
            .with_context(|| {
                format!(
                    "Could not parse field 'metadata' from storage value '{}'",
                    value.metadata
                )
            })?;

        Ok(Token {
            id: value.id,
            hash: value.hash,
            created,
            token_type,
            namespaces,
            metadata,
            expires,
            disabled: value.disabled,
            user: value.user,
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

        let namespaces = serde_json::to_string(&value.namespaces).with_context(|| {
            format!(
                "Could not parse field 'namespaces' from storage value; '{:#?}'",
                value.namespaces
            )
        })?;

        Ok(Self {
            id: value.id,
            hash: value.hash,
            created: value.created.to_string(),
            token_type: value.token_type.to_string(),
            namespaces,
            metadata,
            expires: value.expires.to_string(),
            disabled: value.disabled,
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
/// This endpoint is restricted to management tokens only.
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
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
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
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
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

    let storage_token = match storage::tokens::get_by_id(&mut conn, &req_metadata.auth.key_id).await
    {
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
    /// The type of token to be created. Can be management or client.
    pub token_type: TokenType,

    /// The namespaces this token applies to. Token will be unauthorized for any namespace not listed. Accepts regexes.
    pub namespaces: HashSet<String>,

    /// Various other bits of data you can attach to tokens. This is used by Gofer to track some details about tokens,
    /// but can also be used by users to attach bits of information that would make the token easier to programmatically
    /// manage.
    pub metadata: HashMap<String, String>,

    /// The amount of time the token is valid for in seconds.
    pub expires: u64,

    /// The plaintext username of the token user.
    pub user: String,
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
/// This endpoint is restricted to management tokens only.
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
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
                management_only: true,
            },
        )
        .await?;

    let body = body.into_inner();

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

    let (token, hash) = create_new_api_token();

    let new_token = Token::new(
        &hash,
        body.token_type,
        body.namespaces,
        body.metadata,
        body.expires,
        body.user,
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
/// This endpoint is restricted to management tokens only.
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

/// Create root management token.
///
/// This endpoint can only be hit once and will create the root management token,
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
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: true,
                check_namespace: None,
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

    let mut tx = match conn.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Err(http_error!(
                "Could not open database transaction",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    let exists = match storage::tokens::management_token_exists(&mut tx).await {
        Ok(exists) => exists,
        Err(e) => {
            return Err(http_error!(
                "Could not query database database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                rqctx.request_id.clone(),
                Some(e.into())
            ));
        }
    };

    if exists {
        return Err(HttpError::for_client_error(
            None,
            StatusCode::CONFLICT,
            "Bootstrap token already exists".into(),
        ));
    }

    let (token, hash) = create_new_api_token();

    let new_token = Token::new(
        &hash,
        TokenType::Management,
        HashSet::from([".*".into()]),
        HashMap::from([("bootstrap_token".into(), "true".into())]),
        1103760000, // 35 years in seconds
        "root".into(),
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
        .run_preflight(
            &rqctx.request,
            PreflightOptions {
                bypass_auth: false,
                check_namespace: None,
                management_only: true,
            },
        )
        .await?;

    // Check that user isn't trying to make changes to their own token
    if path.id == _req_metadata.auth.key_id {
        return Err(HttpError::for_client_error(
            None,
            StatusCode::CONFLICT,
            "Cannot make state changes to your own token".into(),
        ));
    }

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
