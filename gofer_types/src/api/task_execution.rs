use crate::{api::task, epoch_milli, storage, Variable};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumString};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TaskExecutionPathArgsRoot {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TaskExecutionPathArgs {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// The unique identifier for the target run.
    pub run_id: u64,

    /// The unique identifier for the target task execution.
    pub task_id: String,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "task_execution_state")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum State {
    /// Should never be in this state.
    #[default]
    Unknown,

    /// Pre-scheduler validation and prep.
    Processing,

    /// Waiting to be scheduled.
    Waiting,

    /// Currently running as reported by scheduler.
    Running,

    Complete,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "task_execution_status")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum Status {
    #[default]
    Unknown,

    /// Has encountered an issue, either container issue or scheduling issue.
    Failed,

    /// Finished with a proper exit code.
    Successful,

    /// Cancelled mid run due to user requested cancellation.
    Cancelled,

    /// Not run due to dependencies not being met.
    Skipped,
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "task_execution_status_reason_type")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum StatusReasonType {
    /// Gofer has no fucking clue how the run got into this state.
    #[default]
    Unknown,

    /// A non-zero exit code has been received.
    AbnormalExit,

    /// Encountered an error with the container scheduler.
    SchedulerError,

    /// User error in task execution parameters.
    FailedPrecondition,

    /// User invoked cancellation.k
    Cancelled,

    /// Task execution was lost due to extreme internal error.
    Orphaned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "task_execution_status_reason")]
pub struct StatusReason {
    /// The specific type of task execution failure.
    pub reason: StatusReasonType,

    /// A description of why the task execution might have failed and what was going on at the time.
    pub description: String,
}

/// a task execution is a specific execution of a task/container. It represents a 4th level unit in the hierarchy.
/// namespace -> pipeline -> run -> task execution. It is the last and most specific object in Gofer's execution model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct TaskExecution {
    /// Unique identifier of the target namespace.
    pub namespace_id: String,

    /// Unique identifier of the target pipeline.
    pub pipeline_id: String,

    /// Unique identifier of the target run.
    pub run_id: u64,

    /// Unique identifier of the current task being executed.
    pub task_id: String,

    /// Time of task execution creation in epoch milliseconds.
    pub created: u64,

    /// Time of task execution start in epoch milliseconds.
    pub started: u64,

    /// Time of task execution end in epoch milliseconds.
    pub ended: u64,

    /// The exit code of the task execution completion if it is finished.
    pub exit_code: Option<u8>,

    /// Whether the logs have past their retention time.
    pub logs_expired: bool,

    /// If the logs for this execution have been removed.
    /// This can be due to user request or automatic action based on expiry time.
    pub logs_removed: bool,

    /// The current state of the task execution within the Gofer execution model.
    /// Describes if the execution is in progress or not.
    pub state: State,

    /// The final result of the task execution.
    pub status: Status,

    /// More information on the circumstances around a particular task execution's status.
    pub status_reason: Option<StatusReason>,

    /// The environment variables injected during this particular task execution.
    pub variables: Vec<Variable>,

    /// Information about the underlying task this task execution ran.
    pub task: task::Task,
}

impl TaskExecution {
    pub fn new(namespace_id: &str, pipeline_id: &str, run_id: u64, task: task::Task) -> Self {
        TaskExecution {
            namespace_id: namespace_id.into(),
            pipeline_id: pipeline_id.into(),
            run_id,
            task_id: task.id.clone(),
            created: epoch_milli(),
            started: 0,
            ended: 0,
            exit_code: None,
            logs_expired: false,
            logs_removed: false,
            state: State::Processing,
            status: Status::Unknown,
            status_reason: None,
            variables: vec![],
            task,
        }
    }
}

impl TryFrom<storage::task_execution::TaskExecution> for TaskExecution {
    type Error = anyhow::Error;

    fn try_from(value: storage::task_execution::TaskExecution) -> Result<Self> {
        let created = value.created.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'created' from storage value '{}'",
                value.created
            )
        })?;

        let started = value.started.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'started' from storage value '{}'",
                value.started
            )
        })?;

        let ended = value.ended.parse::<u64>().with_context(|| {
            format!(
                "Could not parse field 'ended' from storage value '{}'",
                value.ended
            )
        })?;

        let state = State::from_str(&value.state).with_context(|| {
            format!(
                "Could not parse field 'state' from storage value '{}'",
                value.state
            )
        })?;

        let status = Status::from_str(&value.status).with_context(|| {
            format!(
                "Could not parse field 'status' from storage value '{}'",
                value.state
            )
        })?;

        let status_reason = serde_json::from_str(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' from storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        let variables = serde_json::from_str(&value.variables).with_context(|| {
            format!(
                "Could not parse field 'variables' from storage value; '{:#?}'",
                value.variables
            )
        })?;

        let task = serde_json::from_str(&value.task).with_context(|| {
            format!(
                "Could not parse field 'task' from storage value; '{:#?}'",
                value.task
            )
        })?;

        let exit_code = value.exit_code.and_then(|value| match u8::try_from(value) {
            Ok(v) => Some(v),
            Err(e) => {
                debug!(
                    value = value,
                    error = %e,
                    "Could not parse field 'exit code' from storage value; Defaulting to None type"
                );
                None
            }
        });

        Ok(TaskExecution {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            run_id: value.run_id.try_into()?,
            task_id: value.task_id,
            created,
            started,
            ended,
            exit_code,
            logs_expired: value.logs_expired,
            logs_removed: value.logs_removed,
            state,
            status,
            status_reason,
            variables,
            task,
        })
    }
}

impl TryFrom<TaskExecution> for storage::task_execution::TaskExecution {
    type Error = anyhow::Error;

    fn try_from(value: TaskExecution) -> Result<Self> {
        let status_reason = serde_json::to_string(&value.status_reason).with_context(|| {
            format!(
                "Could not parse field 'status_reason' to storage value; '{:#?}'",
                value.status_reason
            )
        })?;

        let task = serde_json::to_string(&value.task).with_context(|| {
            format!(
                "Could not parse field 'task' to storage value; '{:#?}'",
                value.task
            )
        })?;

        let variables = serde_json::to_string(&value.variables).with_context(|| {
            format!(
                "Could not parse field 'variables' to storage value; '{:#?}'",
                value.variables
            )
        })?;

        let exit_code = value.exit_code.map(i64::from);

        Ok(Self {
            namespace_id: value.namespace_id,
            pipeline_id: value.pipeline_id,
            run_id: value.run_id.try_into()?,
            task_id: value.task_id,
            created: value.created.to_string(),
            started: value.started.to_string(),
            ended: value.ended.to_string(),
            exit_code,
            logs_expired: value.logs_expired,
            logs_removed: value.logs_removed,
            state: value.state.to_string(),
            status: value.status.to_string(),
            status_reason,
            variables,
            task,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListTaskExecutionsResponse {
    /// A list of all task executions.
    pub task_executions: Vec<TaskExecution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskExecutionResponse {
    /// The task execution requested.
    pub task_execution: TaskExecution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CancelTaskExecutionQueryArgs {
    /// Period of time to wait the task before forcing it to cancel. 0 means send SIGKILL instantly.
    pub wait_for: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AttachTaskExecutionQueryParams {
    pub command: String,
}
