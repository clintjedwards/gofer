use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::debug;

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
    pub created: u64,
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
    ) -> Result<storage::secret_store_pipeline_key::SecretStorePipelineKey> {
        Ok(storage::secret_store_pipeline_key::SecretStorePipelineKey {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            key: self.key.clone(),
            created: self.created.to_string(),
        })
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

impl TryFrom<storage::secret_store_global_key::SecretStoreGlobalKey> for Secret {
    type Error = anyhow::Error;

    fn try_from(value: storage::secret_store_global_key::SecretStoreGlobalKey) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
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

impl TryFrom<Secret> for storage::secret_store_global_key::SecretStoreGlobalKey {
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

impl TryFrom<storage::secret_store_pipeline_key::SecretStorePipelineKey> for Secret {
    type Error = anyhow::Error;

    fn try_from(value: storage::secret_store_pipeline_key::SecretStorePipelineKey) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
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
