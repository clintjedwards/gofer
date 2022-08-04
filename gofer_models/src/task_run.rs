use super::{epoch, task::Task, Variable};
use gofer_proto::task_run::{TaskRunState, TaskRunStatus};
use gofer_proto::task_run_status_reason;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Since task runs are basically an abstraction over containers, this tells us
/// which state of progress the container is currently in.
#[derive(Debug, Display, EnumString, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum State {
    /// Cannot determine state of task run, should never be in this state.
    Unknown,
    /// Task run is going through pre-scheduling verification and prep.
    Processing,
    /// Task run is waiting on parents task runs to finish.
    Waiting,
    /// Task run is currently running has reported by the scheduler.
    Running,
    /// Task run has completed.
    Complete,
}

impl From<State> for TaskRunState {
    fn from(r: State) -> Self {
        match r {
            State::Unknown => TaskRunState::UnknownState,
            State::Processing => TaskRunState::Processing,
            State::Waiting => TaskRunState::Waiting,
            State::Running => TaskRunState::Running,
            State::Complete => TaskRunState::Complete,
        }
    }
}

impl From<TaskRunState> for State {
    fn from(r: TaskRunState) -> Self {
        match r {
            TaskRunState::UnknownState => State::Unknown,
            TaskRunState::Processing => State::Processing,
            TaskRunState::Waiting => State::Waiting,
            TaskRunState::Running => State::Running,
            TaskRunState::Complete => State::Complete,
        }
    }
}

/// Since task runs are basically an abstraction over containers, this tells us
/// which status the container is in upon completion.
#[derive(Debug, Display, EnumString, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Status {
    /// Status is unknown; if task run is complete and this is that status, something is wrong.
    Unknown,
    /// The task run has completed successfully.
    Successful,
    /// The task run has failed either with an abnormal error code or during processing.
    Failed,
    /// The task run was cancelled during it's execution. Cancelled is explicitly a user
    /// invoked action. The only way a task gets cancelled is from an external request for
    /// it to be.
    Cancelled,
    /// The task run was skipped; This can happen be due to a task failing to meet it's dependencies
    /// (for instance it's parent was in an incorrect state).
    Skipped,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<Status> for TaskRunStatus {
    fn from(r: Status) -> Self {
        match r {
            Status::Unknown => TaskRunStatus::UnknownStatus,
            Status::Successful => TaskRunStatus::Successful,
            Status::Failed => TaskRunStatus::Failed,
            Status::Cancelled => TaskRunStatus::Cancelled,
            Status::Skipped => TaskRunStatus::Skipped,
        }
    }
}

impl From<TaskRunStatus> for Status {
    fn from(r: TaskRunStatus) -> Self {
        match r {
            TaskRunStatus::UnknownStatus => Status::Unknown,
            TaskRunStatus::Successful => Status::Successful,
            TaskRunStatus::Failed => Status::Failed,
            TaskRunStatus::Cancelled => Status::Cancelled,
            TaskRunStatus::Skipped => Status::Skipped,
        }
    }
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, PartialEq, Eq)]
pub enum Reason {
    /// Gofer has no idea how the task run got into this state.
    Unknown,
    /// A non-zero exit code has been received.
    AbnormalExit,
    /// Encountered an error with the backend scheduler.
    SchedulerError,
    /// User error in task run parameters.
    FailedPrecondition,
    /// User invoked cancellation.
    Cancelled,
    /// Task run was lost due to internal error in tracking.
    Orphaned,
}

impl From<Reason> for task_run_status_reason::Reason {
    fn from(r: Reason) -> Self {
        match r {
            Reason::Unknown => task_run_status_reason::Reason::Unknown,
            Reason::AbnormalExit => task_run_status_reason::Reason::AbnormalExit,
            Reason::SchedulerError => task_run_status_reason::Reason::SchedulerError,
            Reason::FailedPrecondition => task_run_status_reason::Reason::FailedPrecondition,
            Reason::Cancelled => task_run_status_reason::Reason::Cancelled,
            Reason::Orphaned => task_run_status_reason::Reason::Orphaned,
        }
    }
}

impl From<task_run_status_reason::Reason> for Reason {
    fn from(r: task_run_status_reason::Reason) -> Self {
        match r {
            task_run_status_reason::Reason::Unknown => Reason::Unknown,
            task_run_status_reason::Reason::AbnormalExit => Reason::AbnormalExit,
            task_run_status_reason::Reason::SchedulerError => Reason::SchedulerError,
            task_run_status_reason::Reason::FailedPrecondition => Reason::FailedPrecondition,
            task_run_status_reason::Reason::Cancelled => Reason::Cancelled,
            task_run_status_reason::Reason::Orphaned => Reason::Orphaned,
        }
    }
}

/// A description of the current status of a task run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusReason {
    /// The kind of reason for the current status.
    pub reason: Reason,
    /// A short description of the reason.
    pub description: String,
}

impl From<StatusReason> for gofer_proto::TaskRunStatusReason {
    fn from(r: StatusReason) -> Self {
        Self {
            reason: task_run_status_reason::Reason::from(r.reason) as i32,
            description: r.description,
        }
    }
}

impl From<gofer_proto::TaskRunStatusReason> for StatusReason {
    fn from(r: gofer_proto::TaskRunStatusReason) -> Self {
        Self {
            reason: gofer_proto::task_run_status_reason::Reason::from_i32(r.reason)
                .unwrap()
                .into(),
            description: r.description,
        }
    }
}

/// A task run is a specific execution of a task/container.
/// It represents a 4th level unit in the hierarchy:
/// namespace -> pipeline -> run -> taskrun
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRun {
    /// Unique identifier for namespace.
    pub namespace: String,
    /// Unique identifier for pipeline.
    pub pipeline: String,
    /// Unique identifier for run.
    pub run: u64,
    /// Unique identifier for task run. Taken from the task identifier.
    pub id: String,
    /// The task information of the task associated with this particular task run.
    pub task: Task,
    /// Time the task run was created. Essentially whenever the run has started.
    pub created: u64,
    /// Time the task run was started on the scheduler.
    pub started: u64,
    /// Time the task run completed.
    pub ended: u64,
    /// The exit code of the task run.
    pub exit_code: Option<u8>,
    /// If the logs have past their predefined retention time.
    pub logs_expired: bool,
    /// If the logs have been removed due to user request or automatic action based on expiry time.
    pub logs_removed: bool,
    /// The current place of progress that task run is at.
    pub state: State,
    /// Upon completion of the task run, the status it has completed with.
    pub status: Status,
    /// Extra information about the current status.
    pub status_reason: Option<StatusReason>,
    /// Identifier used by the scheduler to identify this specific task run container.
    /// This is provided by the scheduler at the time of scheduling.
    pub scheduler_id: Option<String>,
    /// The environment variables injected during this particular task run.
    pub variables: Vec<Variable>,
}

impl TaskRun {
    pub fn new(namespace: &str, pipeline: &str, run: u64, task: Task) -> Self {
        Self {
            namespace: namespace.to_string(),
            pipeline: pipeline.to_string(),
            run,
            id: task.id.clone(),
            task,
            created: epoch(),
            started: 0,
            ended: 0,
            exit_code: None,
            status_reason: None,
            logs_expired: false,
            logs_removed: false,
            state: State::Processing,
            status: Status::Unknown,
            scheduler_id: None,
            variables: vec![],
        }
    }
}

impl From<TaskRun> for gofer_proto::TaskRun {
    fn from(r: TaskRun) -> Self {
        Self {
            namespace_id: r.namespace,
            pipeline_id: r.pipeline,
            run_id: r.run,
            id: r.id,
            task: Some(r.task.into()),
            created: r.created,
            started: r.started,
            ended: r.ended,
            exit_code: r.exit_code.unwrap_or_default() as u64,
            status_reason: r.status_reason.map(|r| r.into()),
            logs_expired: r.logs_expired,
            logs_removed: r.logs_removed,
            state: TaskRunState::from(r.state) as i32,
            status: TaskRunStatus::from(r.status) as i32,
            scheduler_id: r.scheduler_id.unwrap_or_default(),
            variables: r.variables.into_iter().map(|value| value.into()).collect(),
        }
    }
}

impl From<gofer_proto::TaskRun> for TaskRun {
    fn from(r: gofer_proto::TaskRun) -> Self {
        Self {
            namespace: r.namespace_id,
            pipeline: r.pipeline_id,
            run: r.run_id,
            id: r.id,
            task: r.task.unwrap().into(),
            created: r.created,
            started: r.started,
            ended: r.ended,
            exit_code: Some(r.exit_code as u8),
            status_reason: r.status_reason.map(|s| s.into()),
            logs_expired: r.logs_expired,
            logs_removed: r.logs_removed,
            state: gofer_proto::task_run::TaskRunState::from_i32(r.state)
                .unwrap()
                .into(),
            status: gofer_proto::task_run::TaskRunStatus::from_i32(r.status)
                .unwrap()
                .into(),
            scheduler_id: Some(r.scheduler_id),
            variables: r.variables.into_iter().map(|v| v.into()).collect(),
        }
    }
}
