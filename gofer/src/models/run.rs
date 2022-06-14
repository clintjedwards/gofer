use super::Variable;
use crate::models::epoch;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Represents the current state of the run as it progresses through the steps
/// involved to completion.
#[derive(Debug, Display, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunState {
    /// Could not determine current state of the run. Should never happen.
    Unknown,
    /// Before a run is sent to a scheduler it must complete various steps like
    /// validation checking, this state represents that step.
    Pending,
    /// The run is currently being executed on the scheduler.
    Running,
    /// All tasks have been resolved and the run is no longer being executed.
    Complete,
}

impl From<gofer_proto::run::RunState> for RunState {
    fn from(r: gofer_proto::run::RunState) -> Self {
        match r {
            gofer_proto::run::RunState::Unknown => RunState::Unknown,
            gofer_proto::run::RunState::Pending => RunState::Pending,
            gofer_proto::run::RunState::Running => RunState::Running,
            gofer_proto::run::RunState::Complete => RunState::Complete,
        }
    }
}

impl From<RunState> for gofer_proto::run::RunState {
    fn from(r: RunState) -> Self {
        match r {
            RunState::Unknown => gofer_proto::run::RunState::Unknown,
            RunState::Pending => gofer_proto::run::RunState::Pending,
            RunState::Running => gofer_proto::run::RunState::Running,
            RunState::Complete => gofer_proto::run::RunState::Complete,
        }
    }
}

/// Represents the current status of a completed run.
#[derive(Debug, Display, EnumString, PartialEq, Eq)]
pub enum RunStatus {
    /// Could not determine current state of the status. Should only be in this state if
    /// the run has not yet completed.
    Unknown,
    /// All tasks in run have completed with a non-failure state.
    Successful,
    /// One or more tasks in run have failed.
    Failed,
    /// One or more tasks in a run have been cancelled.
    Cancelled,
}

impl From<gofer_proto::run::RunStatus> for RunStatus {
    fn from(r: gofer_proto::run::RunStatus) -> Self {
        match r {
            gofer_proto::run::RunStatus::Unknown => RunStatus::Unknown,
            gofer_proto::run::RunStatus::Successful => RunStatus::Successful,
            gofer_proto::run::RunStatus::Failed => RunStatus::Failed,
            gofer_proto::run::RunStatus::Cancelled => RunStatus::Cancelled,
        }
    }
}

impl From<RunStatus> for gofer_proto::run::RunStatus {
    fn from(r: RunStatus) -> Self {
        match r {
            RunStatus::Unknown => gofer_proto::run::RunStatus::Unknown,
            RunStatus::Successful => gofer_proto::run::RunStatus::Successful,
            RunStatus::Failed => gofer_proto::run::RunStatus::Failed,
            RunStatus::Cancelled => gofer_proto::run::RunStatus::Cancelled,
        }
    }
}

/// Explains in more detail why a particular run might have failed.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunFailureReason {
    /// Could not determine failure reason for current run. Should never happen.
    Unknown,
    /// While executing the run one or more tasks exited with an abnormal exit code.
    AbnormalExit,
    /// While executing the run one or more tasks could not be scheduled.
    SchedulerError,
    /// The run could not be executed as requested due to user defined attributes given.
    FailedPrecondition,
    /// One or more tasks could not be completed due to a user cancelling the run.
    UserCancelled,
    /// One or more tasks could not be completed due to the system or admin cancelling the run.
    AdminCancelled,
}

impl From<gofer_proto::run_failure_info::RunFailureReason> for RunFailureReason {
    fn from(r: gofer_proto::run_failure_info::RunFailureReason) -> Self {
        match r {
            gofer_proto::run_failure_info::RunFailureReason::Unknown => RunFailureReason::Unknown,
            gofer_proto::run_failure_info::RunFailureReason::AbnormalExit => {
                RunFailureReason::AbnormalExit
            }
            gofer_proto::run_failure_info::RunFailureReason::SchedulerError => {
                RunFailureReason::SchedulerError
            }
            gofer_proto::run_failure_info::RunFailureReason::FailedPrecondition => {
                RunFailureReason::FailedPrecondition
            }
            gofer_proto::run_failure_info::RunFailureReason::UserCancelled => {
                RunFailureReason::UserCancelled
            }
            gofer_proto::run_failure_info::RunFailureReason::AdminCancelled => {
                RunFailureReason::AdminCancelled
            }
        }
    }
}

impl From<RunFailureReason> for gofer_proto::run_failure_info::RunFailureReason {
    fn from(r: RunFailureReason) -> Self {
        match r {
            RunFailureReason::Unknown => gofer_proto::run_failure_info::RunFailureReason::Unknown,
            RunFailureReason::AbnormalExit => {
                gofer_proto::run_failure_info::RunFailureReason::AbnormalExit
            }
            RunFailureReason::SchedulerError => {
                gofer_proto::run_failure_info::RunFailureReason::SchedulerError
            }
            RunFailureReason::FailedPrecondition => {
                gofer_proto::run_failure_info::RunFailureReason::FailedPrecondition
            }
            RunFailureReason::UserCancelled => {
                gofer_proto::run_failure_info::RunFailureReason::UserCancelled
            }
            RunFailureReason::AdminCancelled => {
                gofer_proto::run_failure_info::RunFailureReason::AdminCancelled
            }
        }
    }
}

/// Information about a run's failure. Does not get populated before a run is finished.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunFailureInfo {
    /// Why the run might have failed.
    pub reason: RunFailureReason,
    /// A more exact description on what happened.
    pub description: String,
}

impl From<RunFailureInfo> for gofer_proto::RunFailureInfo {
    fn from(r: RunFailureInfo) -> Self {
        Self {
            reason: gofer_proto::run_failure_info::RunFailureReason::from(r.reason) as i32,
            description: r.description,
        }
    }
}

/// Information about which trigger was responsible for the run's execution.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunTriggerInfo {
    /// The trigger kind responsible for starting the run.
    pub kind: String,
    /// The trigger label responsible for starting the run. The label is a user chosen name
    /// for the trigger to differentiate it from other pipeline triggers of the same kind.
    pub label: String,
}

impl From<RunTriggerInfo> for gofer_proto::RunTriggerInfo {
    fn from(r: RunTriggerInfo) -> Self {
        Self {
            kind: r.kind,
            label: r.label,
        }
    }
}

/// Information about the run's store keys as they pertain to Gofer's object store.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunStoreInfo {
    /// After a certain number of runs Gofer's run objects are removed.
    pub is_expired: bool,
    /// They keys specific to this run.
    pub keys: Vec<String>,
}

impl From<RunStoreInfo> for gofer_proto::RunStoreInfo {
    fn from(r: RunStoreInfo) -> Self {
        Self {
            is_expired: r.is_expired,
            keys: r.keys,
        }
    }
}

/// A run is one or more tasks being executed on behalf of some trigger.
/// Run is a third level unit containing tasks and being contained in a pipeline.
#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    /// Identifier for the namespace that this run belongs to.
    pub namespace: String,
    /// Identifier for the pipeline that this run belongs to.
    pub pipeline: String,
    /// Unique numeric auto-incrementing identifier.
    pub id: u64,
    /// Time run started in epoch milli.
    pub started: u64,
    /// Time run ended in epoch milli.
    pub ended: u64,
    /// Used to describe the current stage in the process of the run.
    pub state: RunState,
    /// Used to describe the final outcome of the run (success/fail).
    pub status: RunStatus,
    /// On a failed run, contains more information about the run's status.
    pub failure_info: Option<RunFailureInfo>,
    /// The unique identifier for each task run.
    pub task_runs: Vec<String>,
    /// Information about which trigger was responsible for the run's execution.
    pub trigger: RunTriggerInfo,
    /// Environment variables to be injected into each child task run. These are usually injected by the trigger.
    pub variables: Vec<Variable>,
    /// Information about the object keys that were stored in Gofer's run object store for this run.
    pub store_info: Option<RunStoreInfo>,
}

impl Run {
    pub fn new(
        namespace: &str,
        pipeline: &str,
        trigger: RunTriggerInfo,
        variables: Vec<Variable>,
    ) -> Self {
        Self {
            namespace: namespace.to_string(),
            pipeline: pipeline.to_string(),
            id: 0,
            started: epoch(),
            ended: 0,
            state: RunState::Pending,
            status: RunStatus::Unknown,
            failure_info: None,
            task_runs: vec![],
            trigger,
            variables,
            store_info: None,
        }
    }
}

impl From<Run> for gofer_proto::Run {
    fn from(r: Run) -> Self {
        Self {
            namespace: r.namespace,
            pipeline: r.pipeline,
            id: r.id,
            started: r.started,
            ended: r.ended,
            state: gofer_proto::run::RunState::from(r.state) as i32,
            status: gofer_proto::run::RunStatus::from(r.status) as i32,
            failure_info: r.failure_info.map(|fi| fi.into()),
            task_runs: r.task_runs,
            trigger: Some(r.trigger.into()),
            variables: r.variables.into_iter().map(|value| value.into()).collect(),
            store_info: r.store_info.map(|si| si.into()),
        }
    }
}
