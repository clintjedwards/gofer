use crate::{epoch_milli, storage};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter, EnumString};
use uuid::Uuid;

#[derive(
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    EnumString,
    EnumDiscriminants,
    Display,
    Serialize,
    Deserialize,
    Clone,
    JsonSchema,
)]
#[strum_discriminants(derive(EnumString, Display, Hash))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The Any kind is a special event kind that denotes the caller wants to listen for any event.
    /// It should not be used as a normal event type(for example do not publish anything with it).
    /// It is internal only and not passed back on event streaming.
    Any,

    // Namespace events
    CreatedNamespace {
        namespace_id: String,
    },
    DeletedNamespace {
        namespace_id: String,
    },

    // Pipeline events
    DisabledPipeline {
        namespace_id: String,
        pipeline_id: String,
    },
    EnabledPipeline {
        namespace_id: String,
        pipeline_id: String,
    },
    CreatedPipeline {
        namespace_id: String,
        pipeline_id: String,
    },
    DeletedPipeline {
        namespace_id: String,
        pipeline_id: String,
    },

    // Deployment events
    StartedDeployment {
        namespace_id: String,
        pipeline_id: String,
        start_version: u64,
        end_version: u64,
    },
    CompletedDeployment {
        namespace_id: String,
        pipeline_id: String,
        start_version: u64,
        end_version: u64,
    },

    // Run events
    StartedRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
    },
    CompletedRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        status: runs::Status,
    },
    StartedRunCancellation {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
    },

    // Task execution events
    CreatedTaskExecution {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
    },
    StartedTaskExecution {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
    },
    CompletedTaskExecution {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
        status: task_executions::Status,
    },
    StartedTaskExecutionCancellation {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_execution_id: String,
        timeout: u64,
    },

    // Extension events
    InstalledExtension {
        id: String,
        image: String,
    },
    UninstalledExtension {
        id: String,
        image: String,
    },
    EnabledExtension {
        id: String,
        image: String,
    },
    DisabledExtension {
        id: String,
        image: String,
    },

    // Subscriptions
    PipelineExtensionSubscriptionRegistered {
        namespace_id: String,
        pipeline_id: String,
        extension_id: String,
        subscription_id: String,
    },
    PipelineExtensionSubscriptionUnregistered {
        namespace_id: String,
        pipeline_id: String,
        extension_id: String,
        subscription_id: String,
    },

    // Permissioning eventss
    CreatedRole {
        role_id: String,
    },
    DeletedRole {
        role_id: String,
    },
}

/// A single event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Event {
    /// Unique identifier for event.
    pub id: String,

    /// The type of event it is.
    pub kind: Kind,

    /// Time event was performed in epoch milliseconds.
    pub emitted: u64,
}

impl TryFrom<storage::event::Event> for Event {
    type Error = anyhow::Error;

    fn try_from(value: storage::event::Event) -> Result<Self> {
        let emitted = value.emitted.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'emitted' from storage value '{}'",
                value.emitted
            )
        })?;

        let kind: Kind = serde_json::from_str(&value.kind).with_context(|| {
            format!(
                "Could not parse field 'kind' from storage value '{}'",
                value.kind
            )
        })?;

        Ok(Event {
            id: value.id,
            kind,
            emitted,
        })
    }
}

impl TryFrom<Event> for storage::event::Event {
    type Error = anyhow::Error;

    fn try_from(value: Event) -> Result<Self> {
        let kind = serde_json::to_string(&value.kind).with_context(|| {
            format!(
                "Could not parse field 'kind' to storage value '{:#?}'",
                value.kind
            )
        })?;

        Ok(Self {
            id: value.id,
            kind,
            details: "test".into(),
            emitted: value.emitted.to_string(),
        })
    }
}

impl Event {
    pub fn new(kind: Kind) -> Self {
        Self {
            id: Uuid::now_v7().to_string(),
            kind,
            emitted: epoch_milli(),
        }
    }
}
