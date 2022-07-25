use super::{epoch, Variable};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Represents the current state of the run as it progresses through the steps
/// involved to completion.
#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    /// Could not determine current state of the run. Should never happen.
    Unknown,
    /// Before the tasks in a run is sent to a scheduler it must complete various steps like
    /// validation checking. This state represents that step where the run and task_runs are
    /// pre-checked.
    Pending,
    /// The run is currently being executed on the scheduler.
    Running,
    /// All tasks have been resolved and the run is no longer being executed.
    Complete,
}

impl From<gofer_proto::run::RunState> for State {
    fn from(r: gofer_proto::run::RunState) -> Self {
        match r {
            gofer_proto::run::RunState::Unknown => State::Unknown,
            gofer_proto::run::RunState::Pending => State::Pending,
            gofer_proto::run::RunState::Running => State::Running,
            gofer_proto::run::RunState::Complete => State::Complete,
        }
    }
}

impl From<State> for gofer_proto::run::RunState {
    fn from(r: State) -> Self {
        match r {
            State::Unknown => gofer_proto::run::RunState::Unknown,
            State::Pending => gofer_proto::run::RunState::Pending,
            State::Running => gofer_proto::run::RunState::Running,
            State::Complete => gofer_proto::run::RunState::Complete,
        }
    }
}

/// Represents the current status of a completed run.
#[derive(Debug, Display, EnumString, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Status {
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

impl Default for Status {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<gofer_proto::run::RunStatus> for Status {
    fn from(r: gofer_proto::run::RunStatus) -> Self {
        match r {
            gofer_proto::run::RunStatus::Unknown => Status::Unknown,
            gofer_proto::run::RunStatus::Successful => Status::Successful,
            gofer_proto::run::RunStatus::Failed => Status::Failed,
            gofer_proto::run::RunStatus::Cancelled => Status::Cancelled,
        }
    }
}

impl From<Status> for gofer_proto::run::RunStatus {
    fn from(r: Status) -> Self {
        match r {
            Status::Unknown => gofer_proto::run::RunStatus::Unknown,
            Status::Successful => gofer_proto::run::RunStatus::Successful,
            Status::Failed => gofer_proto::run::RunStatus::Failed,
            Status::Cancelled => gofer_proto::run::RunStatus::Cancelled,
        }
    }
}

/// Explains in more detail why a particular run might have failed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureReason {
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

impl From<gofer_proto::run_failure_info::RunFailureReason> for FailureReason {
    fn from(r: gofer_proto::run_failure_info::RunFailureReason) -> Self {
        match r {
            gofer_proto::run_failure_info::RunFailureReason::Unknown => FailureReason::Unknown,
            gofer_proto::run_failure_info::RunFailureReason::AbnormalExit => {
                FailureReason::AbnormalExit
            }
            gofer_proto::run_failure_info::RunFailureReason::SchedulerError => {
                FailureReason::SchedulerError
            }
            gofer_proto::run_failure_info::RunFailureReason::FailedPrecondition => {
                FailureReason::FailedPrecondition
            }
            gofer_proto::run_failure_info::RunFailureReason::UserCancelled => {
                FailureReason::UserCancelled
            }
            gofer_proto::run_failure_info::RunFailureReason::AdminCancelled => {
                FailureReason::AdminCancelled
            }
        }
    }
}

impl From<FailureReason> for gofer_proto::run_failure_info::RunFailureReason {
    fn from(r: FailureReason) -> Self {
        match r {
            FailureReason::Unknown => gofer_proto::run_failure_info::RunFailureReason::Unknown,
            FailureReason::AbnormalExit => {
                gofer_proto::run_failure_info::RunFailureReason::AbnormalExit
            }
            FailureReason::SchedulerError => {
                gofer_proto::run_failure_info::RunFailureReason::SchedulerError
            }
            FailureReason::FailedPrecondition => {
                gofer_proto::run_failure_info::RunFailureReason::FailedPrecondition
            }
            FailureReason::UserCancelled => {
                gofer_proto::run_failure_info::RunFailureReason::UserCancelled
            }
            FailureReason::AdminCancelled => {
                gofer_proto::run_failure_info::RunFailureReason::AdminCancelled
            }
        }
    }
}

/// Information about a run's failure. Does not get populated before a run is finished.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureInfo {
    /// Why the run might have failed.
    pub reason: FailureReason,
    /// A more exact description on what happened.
    pub description: String,
}

impl From<FailureInfo> for gofer_proto::RunFailureInfo {
    fn from(r: FailureInfo) -> Self {
        Self {
            reason: gofer_proto::run_failure_info::RunFailureReason::from(r.reason) as i32,
            description: r.description,
        }
    }
}

/// Information about which trigger was responsible for the run's execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerInfo {
    /// The trigger kind responsible for starting the run.
    pub name: String,
    /// The trigger label responsible for starting the run. The label is a user chosen name
    /// for the trigger to differentiate it from other pipeline triggers of the same kind.
    pub label: String,
}

impl From<TriggerInfo> for gofer_proto::RunTriggerInfo {
    fn from(r: TriggerInfo) -> Self {
        Self {
            name: r.name,
            label: r.label,
        }
    }
}

/// Information about the run's store keys as they pertain to Gofer's object store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreInfo {
    /// After a certain number of runs Gofer's run objects are removed.
    pub is_expired: bool,
    /// They keys specific to this run.
    pub keys: Vec<String>,
}

impl From<StoreInfo> for gofer_proto::RunStoreInfo {
    fn from(r: StoreInfo) -> Self {
        Self {
            is_expired: r.is_expired,
            keys: r.keys,
        }
    }
}

/// A run is one or more tasks being executed on behalf of some trigger.
/// Run is a third level unit containing tasks and being contained in a pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub state: State,
    /// Used to describe the final outcome of the run (success/fail).
    pub status: Status,
    /// On a failed run, contains more information about the run's status.
    pub failure_info: Option<FailureInfo>,
    /// The unique identifier for each task run.
    pub task_runs: Vec<String>,
    /// Information about which trigger was responsible for the run's execution.
    pub trigger: TriggerInfo,
    /// Environment variables to be injected into each child task run. These are usually injected by the trigger.
    pub variables: Vec<Variable>,
    /// Information about the object keys that were stored in Gofer's run object store for this run.
    pub store_info: Option<StoreInfo>,
}

impl Run {
    pub fn new(
        namespace: &str,
        pipeline: &str,
        trigger: TriggerInfo,
        variables: Vec<Variable>,
    ) -> Self {
        Self {
            namespace: namespace.to_string(),
            pipeline: pipeline.to_string(),
            id: 0,
            started: epoch(),
            ended: 0,
            state: State::Pending,
            status: Status::Unknown,
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
