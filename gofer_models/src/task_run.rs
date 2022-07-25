use super::{epoch, task::Task, Variable};
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

/// A description of the current status of a task run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusReason {
    /// The kind of reason for the current status.
    pub kind: Reason,
    /// A short description of the reason.
    pub description: String,
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
