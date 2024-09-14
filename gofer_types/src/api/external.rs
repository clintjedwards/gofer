use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExternalEventPathArgs {
    /// The unique identifier for the target extension.
    pub extension_id: String,
}
