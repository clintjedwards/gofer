use crate::storage::{map_rusqlite_error, StorageError};
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
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

impl From<&Row<'_>> for TaskExecution {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            run_id: row.get_unwrap("run_id"),
            task_id: row.get_unwrap("task_id"),
            task: row.get_unwrap("task"),
            created: row.get_unwrap("created"),
            started: row.get_unwrap("started"),
            ended: row.get_unwrap("ended"),
            exit_code: row.get_unwrap("exit_code"),
            logs_expired: row.get_unwrap("logs_expired"),
            logs_removed: row.get_unwrap("logs_removed"),
            state: row.get_unwrap("state"),
            status: row.get_unwrap("status"),
            status_reason: row.get_unwrap("status_reason"),
            variables: row.get_unwrap("variables"),
        }
    }
}

#[derive(Iden)]
enum TaskExecutionTable {
    Table,
    NamespaceId,
    PipelineId,
    RunId,
    TaskId,
    Task,
    Created,
    Started,
    Ended,
    ExitCode,
    LogsExpired,
    LogsRemoved,
    State,
    Status,
    StatusReason,
    Variables,
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

pub fn insert(conn: &Connection, task_execution: &TaskExecution) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(TaskExecutionTable::Table)
        .columns([
            TaskExecutionTable::NamespaceId,
            TaskExecutionTable::PipelineId,
            TaskExecutionTable::RunId,
            TaskExecutionTable::TaskId,
            TaskExecutionTable::Task,
            TaskExecutionTable::Created,
            TaskExecutionTable::Started,
            TaskExecutionTable::Ended,
            TaskExecutionTable::ExitCode,
            TaskExecutionTable::LogsExpired,
            TaskExecutionTable::LogsRemoved,
            TaskExecutionTable::State,
            TaskExecutionTable::Status,
            TaskExecutionTable::StatusReason,
            TaskExecutionTable::Variables,
        ])
        .values_panic([
            task_execution.namespace_id.clone().into(),
            task_execution.pipeline_id.clone().into(),
            task_execution.run_id.into(),
            task_execution.task_id.clone().into(),
            task_execution.task.clone().into(),
            task_execution.created.clone().into(),
            task_execution.started.clone().into(),
            task_execution.ended.clone().into(),
            task_execution.exit_code.into(),
            task_execution.logs_expired.into(),
            task_execution.logs_removed.into(),
            task_execution.state.clone().into(),
            task_execution.status.clone().into(),
            task_execution.status_reason.clone().into(),
            task_execution.variables.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
) -> Result<Vec<TaskExecution>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TaskExecutionTable::NamespaceId,
            TaskExecutionTable::PipelineId,
            TaskExecutionTable::RunId,
            TaskExecutionTable::TaskId,
            TaskExecutionTable::Task,
            TaskExecutionTable::Created,
            TaskExecutionTable::Started,
            TaskExecutionTable::Ended,
            TaskExecutionTable::ExitCode,
            TaskExecutionTable::LogsExpired,
            TaskExecutionTable::LogsRemoved,
            TaskExecutionTable::State,
            TaskExecutionTable::Status,
            TaskExecutionTable::StatusReason,
            TaskExecutionTable::Variables,
        ])
        .from(TaskExecutionTable::Table)
        .and_where(Expr::col(TaskExecutionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(TaskExecutionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(TaskExecutionTable::RunId).eq(run_id))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<TaskExecution> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(TaskExecution::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    task_id: &str,
) -> Result<TaskExecution, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TaskExecutionTable::NamespaceId,
            TaskExecutionTable::PipelineId,
            TaskExecutionTable::RunId,
            TaskExecutionTable::TaskId,
            TaskExecutionTable::Task,
            TaskExecutionTable::Created,
            TaskExecutionTable::Started,
            TaskExecutionTable::Ended,
            TaskExecutionTable::ExitCode,
            TaskExecutionTable::LogsExpired,
            TaskExecutionTable::LogsRemoved,
            TaskExecutionTable::State,
            TaskExecutionTable::Status,
            TaskExecutionTable::StatusReason,
            TaskExecutionTable::Variables,
        ])
        .from(TaskExecutionTable::Table)
        .and_where(Expr::col(TaskExecutionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(TaskExecutionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(TaskExecutionTable::RunId).eq(run_id))
        .and_where(Expr::col(TaskExecutionTable::TaskId).eq(task_id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(TaskExecution::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    task_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(TaskExecutionTable::Table);

    if let Some(value) = fields.started {
        query.value(TaskExecutionTable::Started, value);
    }

    if let Some(value) = fields.ended {
        query.value(TaskExecutionTable::Ended, value);
    }

    if let Some(value) = fields.exit_code {
        query.value(TaskExecutionTable::ExitCode, value);
    }

    if let Some(value) = fields.state {
        query.value(TaskExecutionTable::State, value);
    }

    if let Some(value) = fields.status {
        query.value(TaskExecutionTable::Status, value);
    }

    if let Some(value) = fields.status_reason {
        query.value(TaskExecutionTable::StatusReason, value);
    }

    if let Some(value) = fields.logs_expired {
        query.value(TaskExecutionTable::LogsExpired, value);
    }

    if let Some(value) = fields.logs_removed {
        query.value(TaskExecutionTable::LogsRemoved, value);
    }

    if let Some(value) = fields.variables {
        query.value(TaskExecutionTable::Variables, value);
    }

    if query.get_values().is_empty() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query
        .and_where(Expr::col(TaskExecutionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(TaskExecutionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(TaskExecutionTable::RunId).eq(run_id))
        .and_where(Expr::col(TaskExecutionTable::TaskId).eq(task_id));

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

// For now we don't allow deletion of task_executions and there really shouldn't be a need for it, but in the future
// we might allow it through an admin route.
#[allow(dead_code)]
pub fn delete(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    task_id: &str,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(TaskExecutionTable::Table)
        .and_where(Expr::col(TaskExecutionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(TaskExecutionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(TaskExecutionTable::RunId).eq(run_id))
        .and_where(Expr::col(TaskExecutionTable::TaskId).eq(task_id))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;

    fn setup() -> Result<(TestHarness, Connection), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_pipeline_id".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace)?;

        let pipeline_metadata = crate::storage::pipeline_metadata::PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::pipeline_metadata::insert(&mut conn, &pipeline_metadata)?;

        let new_pipeline_config = crate::storage::pipeline_configs::PipelineConfig {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            version: 1,
            parallelism: 4,
            name: "New Test Pipeline".to_string(),
            description: "A newly inserted test pipeline".to_string(),
            registered: "2023-01-01".to_string(),
            deprecated: "none".to_string(),
            state: "active".to_string(),
        };

        crate::storage::pipeline_configs::insert(&mut conn, &new_pipeline_config)
            .expect("Failed to insert pipeline_config");

        let run = crate::storage::runs::Run {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            pipeline_config_version: 1,
            run_id: 1,
            started: "2021-01-01T00:00:00Z".to_string(),
            ended: "2021-01-01T01:00:00Z".to_string(),
            state: "Running".to_string(),
            status: "Active".to_string(),
            status_reason: "No issues".to_string(),
            initiator: "UserA".to_string(),
            variables: "key=value".to_string(),
            token_id: Some("some_id".into()),
            store_objects_expired: false,
        };

        crate::storage::runs::insert(&mut conn, &run)?;

        let task_execution = TaskExecution {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            run_id: 1, // assuming a valid run_id for setup
            task_id: "task001".to_string(),
            task: "Task Description".to_string(),
            created: "2021-01-01T00:00:00Z".to_string(),
            started: "2021-01-01T01:00:00Z".to_string(),
            ended: "2021-01-01T02:00:00Z".to_string(),
            exit_code: None,
            logs_expired: false,
            logs_removed: false,
            state: "some_state".to_string(),
            status: "Completed".to_string(),
            status_reason: "Finished successfully".to_string(),
            variables: "key=value".to_string(),
        };

        insert(&mut conn, &task_execution)?;

        Ok((harness, conn))
    }

    fn test_list_task_executions() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let task_executions = list(&mut conn, "some_id", "some_pipeline_id", 1)
            .expect("Failed to list task_executions");

        assert!(!task_executions.is_empty(), "No task_executions returned");

        let some_task_execution = task_executions
            .iter()
            .find(|n| n.task_id == "task001")
            .expect("TaskExecution not found");
        assert_eq!(some_task_execution.pipeline_id, "some_pipeline_id");
        assert_eq!(some_task_execution.state, "some_state");
    }

    fn test_get_task_execution() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let task_execution = get(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
            .expect("Failed to get task_execution");

        assert_eq!(task_execution.pipeline_id, "some_pipeline_id");
    }

    fn test_update_task_execution() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            started: Some("2021-01-01T03:00:00Z".to_string()),
            ended: Some("2021-01-01T04:00:00Z".to_string()),
            exit_code: Some(1),
            state: Some("updated_state".to_string()),
            status: Some("Failed".to_string()),
            status_reason: Some("Error encountered".to_string()),
            logs_expired: Some(true),
            logs_removed: Some(false),
            variables: Some("key2=value2".to_string()),
        };

        update(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            1,
            "task001",
            fields_to_update,
        )
        .expect("Failed to update task_execution");

        let updated_task_execution = get(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
            .expect("Failed to retrieve updated task_execution");

        assert_eq!(updated_task_execution.state, "updated_state");
    }

    fn test_delete_task_execution() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
            .expect("Failed to delete task_execution");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", 1, "task001").is_err(),
            "TaskExecution was not deleted"
        );
    }
}
