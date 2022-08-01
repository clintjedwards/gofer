use super::{epoch, Variable};
use gofer_proto::run::{RunState, RunStatus};
use gofer_proto::run_status_reason::RunStatusReason;
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

impl From<RunState> for State {
    fn from(r: RunState) -> Self {
        match r {
            RunState::Unknown => State::Unknown,
            RunState::Pending => State::Pending,
            RunState::Running => State::Running,
            RunState::Complete => State::Complete,
        }
    }
}

impl From<State> for RunState {
    fn from(r: State) -> Self {
        match r {
            State::Unknown => RunState::Unknown,
            State::Pending => RunState::Pending,
            State::Running => RunState::Running,
            State::Complete => RunState::Complete,
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

impl From<RunStatus> for Status {
    fn from(r: RunStatus) -> Self {
        match r {
            RunStatus::Unknown => Status::Unknown,
            RunStatus::Successful => Status::Successful,
            RunStatus::Failed => Status::Failed,
            RunStatus::Cancelled => Status::Cancelled,
        }
    }
}

impl From<Status> for RunStatus {
    fn from(r: Status) -> Self {
        match r {
            Status::Unknown => RunStatus::Unknown,
            Status::Successful => RunStatus::Successful,
            Status::Failed => RunStatus::Failed,
            Status::Cancelled => RunStatus::Cancelled,
        }
    }
}

/// Explains in more detail why a particular run might have the status that it does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Reason {
    /// Gofer has no idea who the run got into this state.
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

impl From<RunStatusReason> for Reason {
    fn from(r: RunStatusReason) -> Self {
        match r {
            RunStatusReason::Unknown => Reason::Unknown,
            RunStatusReason::AbnormalExit => Reason::AbnormalExit,
            RunStatusReason::SchedulerError => Reason::SchedulerError,
            RunStatusReason::FailedPrecondition => Reason::FailedPrecondition,
            RunStatusReason::UserCancelled => Reason::UserCancelled,
            RunStatusReason::AdminCancelled => Reason::AdminCancelled,
        }
    }
}

impl From<Reason> for RunStatusReason {
    fn from(r: Reason) -> Self {
        match r {
            Reason::Unknown => RunStatusReason::Unknown,
            Reason::AbnormalExit => RunStatusReason::AbnormalExit,
            Reason::SchedulerError => RunStatusReason::SchedulerError,
            Reason::FailedPrecondition => RunStatusReason::FailedPrecondition,
            Reason::UserCancelled => RunStatusReason::UserCancelled,
            Reason::AdminCancelled => RunStatusReason::AdminCancelled,
        }
    }
}

/// More information about a run's status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusReason {
    /// Why the run might have failed.
    pub reason: Reason,
    /// A more exact description on what happened.
    pub description: String,
}

impl From<StatusReason> for gofer_proto::RunStatusReason {
    fn from(r: StatusReason) -> Self {
        Self {
            reason: RunStatusReason::from(r.reason) as i32,
            description: r.description,
        }
    }
}

impl From<gofer_proto::RunStatusReason> for StatusReason {
    fn from(r: gofer_proto::RunStatusReason) -> Self {
        Self {
            reason: gofer_proto::run_status_reason::RunStatusReason::from_i32(r.reason)
                .unwrap()
                .into(),
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

impl From<gofer_proto::RunTriggerInfo> for TriggerInfo {
    fn from(r: gofer_proto::RunTriggerInfo) -> Self {
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

impl From<gofer_proto::RunStoreInfo> for StoreInfo {
    fn from(r: gofer_proto::RunStoreInfo) -> Self {
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
    /// Contains more information about a run's current status.
    pub status_reason: Option<StatusReason>,
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
            status_reason: None,
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
            state: RunState::from(r.state) as i32,
            status: RunStatus::from(r.status) as i32,
            status_reason: r.status_reason.map(|fi| fi.into()),
            task_runs: r.task_runs,
            trigger: Some(r.trigger.into()),
            variables: r.variables.into_iter().map(|value| value.into()).collect(),
            store_info: r.store_info.map(|si| si.into()),
        }
    }
}

impl From<gofer_proto::Run> for Run {
    fn from(r: gofer_proto::Run) -> Self {
        Self {
            namespace: r.namespace,
            pipeline: r.pipeline,
            id: r.id,
            started: r.started,
            ended: r.ended,
            state: gofer_proto::run::RunState::from_i32(r.state)
                .unwrap()
                .into(),
            status: gofer_proto::run::RunStatus::from_i32(r.status)
                .unwrap()
                .into(),
            status_reason: r.status_reason.map(|fi| fi.into()),
            task_runs: r.task_runs,
            trigger: r.trigger.unwrap().into(),
            variables: r.variables.into_iter().map(|value| value.into()).collect(),
            store_info: r.store_info.map(|si| si.into()),
        }
    }
}
