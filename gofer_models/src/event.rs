use crate::{run, task_run};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter, EnumString};

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
)]
// KindDiscriminant is a strum derive that allows us to mention which enum variant we want
// without having to define the entire enum.
// For example: We might have a function which just wants to filter on events :: sub(event)
// Instead of determining which enum we want by giving the whole enum including the data
// like so: `sub(CreatedNamespace{...})` instead we can use the KindDiscriminant to give
// just the variant like so: `sub(CreatedNamespace)`.
#[strum_discriminants(derive(EnumString, Display, Hash))]
#[strum_discriminants(name(KindDiscriminant))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[strum(serialize_all = "snake_case")]
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
        status: run::Status,
    },

    // Task run events
    CreatedTaskRun {
        namespace_id: String,
        pipeline_id: String,
        run_id: u64,
        task_run_id: String,
    },
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
        status: task_run::Status,
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
    pub kind: Kind,
    /// Time event was performed in epoch milliseconds.
    pub emitted: u64,
}

impl Event {
    pub fn new(kind: Kind) -> Self {
        Self {
            id: 0,
            kind,
            emitted: super::epoch(),
        }
    }
}

impl From<Event> for gofer_proto::Event {
    fn from(r: Event) -> Self {
        Self {
            id: r.id,
            kind: KindDiscriminant::from(&r.kind).to_string(),
            details: serde_json::to_string(&r.kind).unwrap(),
            emitted: r.emitted,
        }
    }
}
