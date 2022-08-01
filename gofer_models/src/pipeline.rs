use super::{epoch, task::Task};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{Display, EnumString};

/// The current state of the pipeline. Pipelines can be disabled to stop execution.
#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    /// The state of the pipeline is unknown. This should never happen.
    Unknown,
    /// Pipeline is enabled and able to start runs.
    Active,
    /// Pipeline is disabled and not able to start runs. Any triggers will be removed and any manually started runs
    /// will automatically fail.
    Disabled,
}

impl From<gofer_proto::pipeline::PipelineState> for State {
    fn from(p: gofer_proto::pipeline::PipelineState) -> Self {
        match p {
            gofer_proto::pipeline::PipelineState::Unknown => State::Unknown,
            gofer_proto::pipeline::PipelineState::Active => State::Active,
            gofer_proto::pipeline::PipelineState::Disabled => State::Disabled,
        }
    }
}

impl From<State> for gofer_proto::pipeline::PipelineState {
    fn from(p: State) -> Self {
        match p {
            State::Unknown => gofer_proto::pipeline::PipelineState::Unknown,
            State::Active => gofer_proto::pipeline::PipelineState::Active,
            State::Disabled => gofer_proto::pipeline::PipelineState::Disabled,
        }
    }
}

/// A collection of logically grouped tasks. A task is a unit of work wrapped in a docker container.
/// Pipeline is a secondary level unit being contained within namespaces and containing runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    /// Unique identifier for the namespace that this pipeline belongs to.
    pub namespace: String,
    /// Unique user defined identifier.
    pub id: String,
    /// Humanized name, meant for display.
    pub name: String,
    /// Short description of what the pipeline is used for.
    pub description: String,
    /// Controls how many runs can be active at any single time. 0 indicates unbounded with respect to bounds
    /// enforced by Gofer.
    pub parallelism: u64,
    /// The creation time in epoch milli.
    pub created: u64,
    /// The last modified time in epoch milli. Only updates on changes to the pipeline attributes, not tangential
    /// things like last run time.
    pub modified: u64,
    /// The current state of the pipeline. Pipelines can be disabled to stop execution of runs/tasks.
    pub state: State,
    /// A mapping of pipeline owned tasks.
    pub tasks: HashMap<String, Task>,
    /// A mapping of pipeline owned triggers to their settings.
    pub triggers: HashMap<String, TriggerSettings>,
    /// A mapping of pipeline owned common_tasks to their settings.
    pub common_tasks: HashMap<String, CommonTaskSettings>,
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
            common_tasks: p
                .common_tasks
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
            store_keys: p.store_keys,
        }
    }
}

impl From<gofer_proto::Pipeline> for Pipeline {
    fn from(p: gofer_proto::Pipeline) -> Self {
        Self {
            namespace: p.namespace,
            id: p.id,
            name: p.name,
            description: p.description,
            parallelism: p.parallelism,
            created: p.created,
            modified: p.modified,
            state: gofer_proto::pipeline::PipelineState::from_i32(p.state)
                .unwrap()
                .into(),
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
            common_tasks: p
                .common_tasks
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
            parallelism: config.parallelism,
            created: epoch(),
            modified: epoch(),
            state: State::Active,
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
            common_tasks: config
                .common_tasks
                .into_iter()
                .map(|common_task| (common_task.label.clone(), common_task.into()))
                .collect(),
            store_keys: vec![],
        }
    }
}

/// Every time a pipeline attempts to subscribe to a trigger, it passes certain
/// values back to that trigger for certain functionality. Since triggers keep no
/// permanent state, these settings are kept here so that when triggers are restarted
/// they can be restored with proper settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerSettings {
    /// A global unique identifier for the trigger type.
    pub name: String,
    /// A user defined identifier for the trigger so that a pipeline with
    /// multiple common tasks can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific trigger.
    pub settings: HashMap<String, String>,
    /// If the trigger could not be set up for the pipeline we return an error on why that might be.
    pub error: Option<String>,
}

impl TriggerSettings {
    pub fn new(kind: &str, label: &str) -> Self {
        TriggerSettings {
            name: kind.to_string(),
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

impl From<gofer_proto::PipelineTriggerSettings> for TriggerSettings {
    fn from(p: gofer_proto::PipelineTriggerSettings) -> Self {
        TriggerSettings {
            name: p.name,
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

impl From<TriggerSettings> for gofer_proto::PipelineTriggerSettings {
    fn from(p: TriggerSettings) -> Self {
        gofer_proto::PipelineTriggerSettings {
            name: p.name,
            label: p.label,
            settings: p.settings,
            error: match p.error {
                Some(error) => error,
                None => "".to_string(),
            },
        }
    }
}

impl From<gofer_sdk::config::PipelineTriggerConfig> for TriggerSettings {
    fn from(p: gofer_sdk::config::PipelineTriggerConfig) -> Self {
        Self {
            name: p.name,
            label: p.label,
            settings: p.settings,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommonTaskSettings {
    /// A global unique identifier for the common_task type.
    pub name: String,
    /// A user defined identifier for the common_task so that a pipeline with
    /// multiple common_tasks can be differentiated.
    pub label: String,
    /// The settings for pertaining to that specific common_task.
    pub settings: HashMap<String, String>,
    /// If the common_task could not be set up for the pipeline we return an error on why that might be.
    pub error: Option<String>,
}

impl CommonTaskSettings {
    pub fn new(name: &str, label: &str) -> Self {
        CommonTaskSettings {
            name: name.to_string(),
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

impl From<gofer_proto::PipelineCommonTaskSettings> for CommonTaskSettings {
    fn from(p: gofer_proto::PipelineCommonTaskSettings) -> Self {
        CommonTaskSettings {
            name: p.name,
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

impl From<CommonTaskSettings> for gofer_proto::PipelineCommonTaskSettings {
    fn from(p: CommonTaskSettings) -> Self {
        gofer_proto::PipelineCommonTaskSettings {
            name: p.name,
            label: p.label,
            settings: p.settings,
            error: match p.error {
                Some(error) => error,
                None => "".to_string(),
            },
        }
    }
}

impl From<gofer_sdk::config::PipelineCommonTaskConfig> for CommonTaskSettings {
    fn from(p: gofer_sdk::config::PipelineCommonTaskConfig) -> Self {
        Self {
            name: p.name,
            label: p.label,
            settings: p.settings,
            error: None,
        }
    }
}
