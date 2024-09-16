use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use rand::{distributions::Alphanumeric, Rng};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, ops::Add};

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
        expiry: u64, // Seconds from creation that token should expire.
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

impl TryFrom<storage::token::Token> for Token {
    type Error = anyhow::Error;

    fn try_from(value: storage::token::Token) -> Result<Self> {
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

impl TryFrom<Token> for storage::token::Token {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetTokenByIDResponse {
    /// The target token.
    pub token: Token,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhoAmIResponse {
    /// The target token.
    pub token: Token,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeleteTokenResponse {
    /// Information about the token deleted.
    pub token_details: Token,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTokenRequest {
    pub disabled: Option<bool>,
}
