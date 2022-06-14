use crate::{RunStatus, TaskRunStatus};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

#[derive(Debug, PartialEq, Eq, EnumIter, EnumString, Display, Serialize, Deserialize, Clone)]
pub enum EventKind {
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
        status: RunStatus,
    },

    // Task run events
    StartedTaskRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_run_id: String,
    },
    CompletedTaskRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_run_id: String,
        status: TaskRunStatus,
    },

    // Trigger events
    InstalledTrigger {
        name: String,
        image: String,
    },
    UninstalledTrigger {
        name: String,
        image: String,
    },
    EnabledTrigger {
        name: String,
        image: String,
    },
    DisabledTrigger {
        name: String,
        image: String,
    },

    // Trigger event events
    FiredTriggerEvent {
        namespace_id: String,
        pipeline_id: String,
        label: String,
    },
    ProcessedTriggerEvent {
        namespace_id: String,
        pipeline_id: String,
        label: String,
    },
    ResolvedTriggerEvent {
        namespace_id: String,
        pipeline_id: String,
        label: String,
    },
}

/// A single event type
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Event {
    /// Unique identifier for event.
    pub id: u64,
    /// The type of event it is.
    pub kind: EventKind,
    /// Time event was performed in epoch milliseconds.
    pub emitted: u64,
}

impl Event {
    pub fn new(kind: EventKind) -> Self {
        Self {
            id: 0,
            kind,
            emitted: super::epoch(),
        }
    }
}
