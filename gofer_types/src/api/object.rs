use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineObjectPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineObjectPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target object.
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunObjectPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunObjectPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,

    /// The unique identifier for the target object.
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionObjectPathArgsRoot {
    /// The unique identifier for the target extension.
    pub extension_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionObjectPathArgs {
    /// The unique identifier for the target extension.
    pub extension_id: String,

    /// The unique identifier for the target object.
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Object {
    /// The identifier for the object value.
    pub key: String,

    /// Time in epoch milliseconds that this object was registered.
    pub created: u64,
}

impl Object {
    pub fn new(key: &str) -> Self {
        Object {
            key: key.into(),
            created: epoch_milli(),
        }
    }

    fn to_pipeline_object_storage(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
    ) -> Result<storage::object_store_pipeline_key::ObjectStorePipelineKey> {
        Ok(storage::object_store_pipeline_key::ObjectStorePipelineKey {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            key: self.key.clone(),
            created: self.created.to_string(),
        })
    }

    fn to_run_object_storage(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<storage::object_store_run_key::ObjectStoreRunKey> {
        let run_id_i64: i64 = run_id.try_into().with_context(|| {
            format!(
                "Could not parse field 'run_id' to storage value '{}'",
                run_id
            )
        })?;

        Ok(storage::object_store_run_key::ObjectStoreRunKey {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            run_id: run_id_i64,
            key: self.key.clone(),
            created: self.created.to_string(),
        })
    }

    fn to_extension_object_storage(
        &self,
        extension_id: &str,
    ) -> Result<storage::object_store_extension_key::ObjectStoreExtensionKey> {
        Ok(
            storage::object_store_extension_key::ObjectStoreExtensionKey {
                extension_id: extension_id.into(),
                key: self.key.clone(),
                created: self.created.to_string(),
            },
        )
    }
}

impl TryFrom<storage::object_store_run_key::ObjectStoreRunKey> for Object {
    type Error = anyhow::Error;

    fn try_from(value: storage::object_store_run_key::ObjectStoreRunKey) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Object {
            key: value.key,
            created,
        })
    }
}

impl TryFrom<storage::object_store_pipeline_key::ObjectStorePipelineKey> for Object {
    type Error = anyhow::Error;

    fn try_from(value: storage::object_store_pipeline_key::ObjectStorePipelineKey) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Object {
            key: value.key,
            created,
        })
    }
}

impl TryFrom<storage::object_store_extension_key::ObjectStoreExtensionKey> for Object {
    type Error = anyhow::Error;

    fn try_from(
        value: storage::object_store_extension_key::ObjectStoreExtensionKey,
    ) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        Ok(Object {
            key: value.key,
            created,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListRunObjectsResponse {
    /// A list of all run objects.
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetRunObjectResponse {
    /// The requested object data.
    pub object: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutRunObjectRequest {
    /// The name for the object you would like to store.
    pub key: String,

    /// The bytes for the object.
    pub content: Vec<u8>,

    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutRunObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPipelineObjectsResponse {
    /// A list of all pipeline objects.
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetPipelineObjectResponse {
    /// The requested object data.
    pub object: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineObjectRequest {
    /// The name for the object you would like to store.
    pub key: String,

    /// The bytes for the object.
    pub content: Vec<u8>,

    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutPipelineObjectResponse {
    /// Information about the object created.
    pub object: Object,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListExtensionObjectsResponse {
    /// A list of all extension objects.
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetExtensionObjectResponse {
    /// The requested object data.
    pub object: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionObjectRequest {
    /// The name for the object you would like to store.
    pub key: String,

    /// The bytes for the object.
    pub content: Vec<u8>,

    /// Overwrite a value of a object if it already exists.
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PutExtensionObjectResponse {
    /// Information about the object created.
    pub object: Object,
}
