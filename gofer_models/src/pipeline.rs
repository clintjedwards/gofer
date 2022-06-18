use super::{epoch, Task};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumString};

/// The current state of the pipeline. Pipelines can be disabled to stop execution.
#[derive(Debug, Display, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum PipelineState {
    /// The state of the pipeline is unknown. This should never happen.
    Unknown,
    /// Pipeline is enabled and able to start runs.
    Active,
    /// Pipeline is disabled and not able to start runs. Any triggers will be removed and any manually started runs
    /// will automatically fail.
    Disabled,
}

impl From<gofer_proto::pipeline::PipelineState> for PipelineState {
    fn from(p: gofer_proto::pipeline::PipelineState) -> Self {
        match p {
            gofer_proto::pipeline::PipelineState::Unknown => PipelineState::Unknown,
            gofer_proto::pipeline::PipelineState::Active => PipelineState::Active,
            gofer_proto::pipeline::PipelineState::Disabled => PipelineState::Disabled,
        }
    }
}

impl From<PipelineState> for gofer_proto::pipeline::PipelineState {
    fn from(p: PipelineState) -> Self {
        match p {
            PipelineState::Unknown => gofer_proto::pipeline::PipelineState::Unknown,
            PipelineState::Active => gofer_proto::pipeline::PipelineState::Active,
            PipelineState::Disabled => gofer_proto::pipeline::PipelineState::Disabled,
        }
    }
}

/// A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
/// Pipeline is a secondary level unit being contained within namespaces and containing runs.
#[derive(Debug, PartialEq, Eq)]
pub struct Pipeline {
    /// Unique identifier for the namespace that this pipeline belongs to.
    pub namespace: String,
    /// Unique user defined identifier.
    pub id: String,
    /// Humanized name, meant for display.
    pub name: String,
    /// Short description of what the pipeline is used for.
    pub description: String,
    /// The identifier for the last run that the pipeline executed.
    pub last_run_id: u64,
    /// The time in epoch milli that the last run started. 0 indicates that this was never run.
    pub last_run_time: u64,
    /// Controls how many runs can be active at any single time. 0 indicates unbounded with respect to bounds
    /// enforced by Gofer.
    pub parallelism: u64,
    /// The creation time in epoch milli.
    pub created: u64,
    /// The last modified time in epoch milli. Only updates on changes to the pipeline attributes, not tangential
    /// things like last run time.
    pub modified: u64,
    /// The current state of the pipeline. Pipelines can be disabled to stop execution of runs/tasks.
    pub state: PipelineState,
    /// A mapping of pipeline owned tasks.
    pub tasks: HashMap<String, Task>,
    /// A mapping of pipeline owned triggers to their settings.
    pub triggers: HashMap<String, PipelineTriggerSettings>,
    /// A mapping of pipeline owned notifiers to their settings.
    pub notifiers: HashMap<String, PipelineNotifierSettings>,
    /// A listing pipeline owned keys that are stored in Gofer's object store.
    pub store_keys: Vec<String>,
}

impl From<Pipeline> for gofer_proto::Pipeline {
    fn from(p: Pipeline) -> Self {
        gofer_proto::Pipeline {
            namespace: p.namespace,
            id: p.id,
            name: p.name,
            description: p.description,
            last_run_id: p.last_run_id,
            last_run_time: p.last_run_time,
            parallelism: p.parallelism,
            created: p.created,
            modified: p.modified,
            state: gofer_proto::pipeline::PipelineState::from(p.state) as i32,
            tasks: p
                .tasks
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
            triggers: p
                .triggers
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
            notifiers: p
                .notifiers
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
            store_keys: p.store_keys,
        }
    }
}

impl Pipeline {
    pub fn new(namespace: &str, config: gofer_sdk::config::Pipeline) -> Self {
        Pipeline {
            namespace: namespace.to_string(),
            id: config.id,
            name: config.name,
            description: config.description.unwrap_or_default(),
            last_run_id: 0,
            last_run_time: 0,
            parallelism: config.parallelism,
            created: epoch(),
            modified: epoch(),
            state: PipelineState::Active,
            tasks: config
                .tasks
                .into_iter()
                .map(|task| (task.id.clone(), task.into()))
                .collect(),
            triggers: config
                .triggers
                .into_iter()
                .map(|trigger| (trigger.label.clone(), trigger.into()))
                .collect(),
            notifiers: config
                .notifiers
                .into_iter()
                .map(|notifier| (notifier.label.clone(), notifier.into()))
                .collect(),
            store_keys: vec![],
        }
    }
}

/// Every time a pipeline attempts to subscribe to a trigger, it passes certain
/// values back to that trigger for certain functionality. Since triggers keep no
/// permanent state, these settings are kept here so that when triggers are restarted
/// they can be restored with proper settings.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineTriggerSettings {
    /// A global unique identifier for the trigger type.
    pub kind: String,
    /// A user defined identifier for the trigger so that a pipeline with
    /// multiple notifiers can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific trigger.
    pub settings: HashMap<String, String>,
    /// If the trigger could not be set up for the pipeline we return an error on why that might be.
    pub error: Option<String>,
}

impl PipelineTriggerSettings {
    pub fn new(kind: &str, label: &str) -> Self {
        PipelineTriggerSettings {
            kind: kind.to_string(),
            label: label.to_string(),
            settings: HashMap::new(),
            error: None,
        }
    }

    pub fn settings(mut self, settings: HashMap<String, String>) -> Self {
        self.settings = settings;
        self
    }
}

impl From<gofer_proto::PipelineTriggerSettings> for PipelineTriggerSettings {
    fn from(p: gofer_proto::PipelineTriggerSettings) -> Self {
        PipelineTriggerSettings {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
            error: {
                if p.error.is_empty() {
                    None
                } else {
                    Some(p.error)
                }
            },
        }
    }
}

impl From<PipelineTriggerSettings> for gofer_proto::PipelineTriggerSettings {
    fn from(p: PipelineTriggerSettings) -> Self {
        gofer_proto::PipelineTriggerSettings {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
            error: match p.error {
                Some(error) => error,
                None => "".to_string(),
            },
        }
    }
}

impl From<gofer_sdk::config::PipelineTriggerConfig> for PipelineTriggerSettings {
    fn from(p: gofer_sdk::config::PipelineTriggerConfig) -> Self {
        Self {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
            error: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineNotifierSettings {
    /// A global unique identifier for the notifier type.
    pub kind: String,
    /// A user defined identifier for the notifier so that a pipeline with
    /// multiple notifiers can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific notifier.
    pub settings: HashMap<String, String>,
    /// If the notifier could not be set up for the pipeline we return an error on why that might be.
    pub error: Option<String>,
}

impl PipelineNotifierSettings {
    pub fn new(kind: &str, label: &str) -> Self {
        PipelineNotifierSettings {
            kind: kind.to_string(),
            label: label.to_string(),
            settings: HashMap::new(),
            error: None,
        }
    }

    pub fn settings(mut self, settings: HashMap<String, String>) -> Self {
        self.settings = settings;
        self
    }
}

impl From<gofer_proto::PipelineNotifierSettings> for PipelineNotifierSettings {
    fn from(p: gofer_proto::PipelineNotifierSettings) -> Self {
        PipelineNotifierSettings {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
            error: {
                if p.error.is_empty() {
                    None
                } else {
                    Some(p.error)
                }
            },
        }
    }
}

impl From<PipelineNotifierSettings> for gofer_proto::PipelineNotifierSettings {
    fn from(p: PipelineNotifierSettings) -> Self {
        gofer_proto::PipelineNotifierSettings {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
            error: match p.error {
                Some(error) => error,
                None => "".to_string(),
            },
        }
    }
}

impl From<gofer_sdk::config::PipelineNotifierConfig> for PipelineNotifierSettings {
    fn from(p: gofer_sdk::config::PipelineNotifierConfig) -> Self {
        Self {
            kind: p.kind,
            label: p.label,
            settings: p.settings,
            error: None,
        }
    }
}
