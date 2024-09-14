use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct TaskExecution {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub run_id: i64,
    pub task_id: String,
    pub task: String,
    pub created: String,
    pub started: String,
    pub ended: String,
    pub exit_code: Option<i64>,
    pub logs_expired: bool,
    pub logs_removed: bool,
    pub state: String,
    pub status: String,
    pub status_reason: String,
    pub variables: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub started: Option<String>,
    pub ended: Option<String>,
    pub exit_code: Option<i64>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub logs_expired: Option<bool>,
    pub logs_removed: Option<bool>,
    pub variables: Option<String>,
}
