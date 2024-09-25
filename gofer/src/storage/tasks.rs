use crate::storage::{map_rusqlite_error, StorageError};
use futures::TryFutureExt;
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct Task {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub pipeline_config_version: i64,
    pub task_id: String,
    pub description: String,
    pub image: String,
    pub registry_auth: String,
    pub depends_on: String,
    pub variables: String,
    pub entrypoint: String,
    pub command: String,
    pub inject_api_token: bool,
}

impl From<&Row<'_>> for Task {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            pipeline_config_version: row.get_unwrap("pipeline_config_version"),
            task_id: row.get_unwrap("task_id"),
            description: row.get_unwrap("description"),
            image: row.get_unwrap("image"),
            registry_auth: row.get_unwrap("registry_auth"),
            depends_on: row.get_unwrap("depends_on"),
            variables: row.get_unwrap("variables"),
            entrypoint: row.get_unwrap("entrypoint"),
            command: row.get_unwrap("command"),
            inject_api_token: row.get_unwrap("inject_api_token"),
        }
    }
}

#[derive(Iden)]
enum TaskTable {
    Table,
    NamespaceId,
    PipelineId,
    PipelineConfigVersion,
    TaskId,
    Description,
    Image,
    RegistryAuth,
    DependsOn,
    Variables,
    Entrypoint,
    Command,
    InjectApiToken,
}

pub fn insert(conn: &mut Connection, task: &Task) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(TaskTable::Table)
        .columns([
            TaskTable::NamespaceId,
            TaskTable::PipelineId,
            TaskTable::PipelineConfigVersion,
            TaskTable::TaskId,
            TaskTable::Description,
            TaskTable::Image,
            TaskTable::RegistryAuth,
            TaskTable::DependsOn,
            TaskTable::Variables,
            TaskTable::Entrypoint,
            TaskTable::Command,
            TaskTable::InjectApiToken,
        ])
        .values_panic([
            task.namespace_id.clone().into(),
            task.pipeline_id.clone().into(),
            task.pipeline_config_version.into(),
            task.task_id.clone().into(),
            task.description.clone().into(),
            task.image.clone().into(),
            task.registry_auth.clone().into(),
            task.depends_on.clone().into(),
            task.variables.clone().into(),
            task.entrypoint.clone().into(),
            task.command.clone().into(),
            task.inject_api_token.into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
) -> Result<Vec<Task>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TaskTable::NamespaceId,
            TaskTable::PipelineId,
            TaskTable::PipelineConfigVersion,
            TaskTable::TaskId,
            TaskTable::Description,
            TaskTable::Image,
            TaskTable::RegistryAuth,
            TaskTable::DependsOn,
            TaskTable::Variables,
            TaskTable::Entrypoint,
            TaskTable::Command,
            TaskTable::InjectApiToken,
        ])
        .from(TaskTable::Table)
        .and_where(Expr::col(TaskTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(TaskTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(TaskTable::PipelineConfigVersion).eq(version))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut tasks: Vec<Task> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        tasks.push(Task::from(row));
    }

    Ok(tasks)
}

// Currently a task is embedded within a task_execution anyway so there isn't any need for a user to ever just get a
// task. But for the sake of standardization we'll keep this crud function here.
#[allow(dead_code)]
pub fn get(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
    task_id: &str,
) -> Result<Task, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TaskTable::NamespaceId,
            TaskTable::PipelineId,
            TaskTable::PipelineConfigVersion,
            TaskTable::TaskId,
            TaskTable::Description,
            TaskTable::Image,
            TaskTable::RegistryAuth,
            TaskTable::DependsOn,
            TaskTable::Variables,
            TaskTable::Entrypoint,
            TaskTable::Command,
            TaskTable::InjectApiToken,
        ])
        .from(TaskTable::Table)
        .and_where(Expr::col(TaskTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(TaskTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(TaskTable::PipelineConfigVersion).eq(version))
        .and_where(Expr::col(TaskTable::TaskId).eq(task_id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Task::from(row));
    }

    Err(StorageError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        namespaces::{self, Namespace},
        pipeline_configs::{self, PipelineConfig},
        pipeline_metadata::{self, PipelineMetadata},
        tests::TestHarness,
        Executable,
    };

    fn setup() -> Result<(TestHarness, impl Executable), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

        let namespace = Namespace {
            id: "default".into(),
            name: "Default".into(),
            description: String::new(),
            created: String::new(),
            modified: String::new(),
        };

        namespaces::insert(&mut conn, &namespace).unwrap();

        let pipeline = PipelineMetadata {
            namespace_id: "default".into(),
            pipeline_id: "test".into(),
            state: String::new(),
            created: String::new(),
            modified: String::new(),
        };

        pipeline_metadata::insert(&mut conn, &pipeline).unwrap();

        let new_pipeline_config = PipelineConfig {
            namespace_id: "default".to_string(),
            pipeline_id: "test".to_string(),
            version: 1,
            parallelism: 4,
            name: "New Test Pipeline".to_string(),
            description: "A newly inserted test pipeline".to_string(),
            registered: "2023-01-01".to_string(),
            deprecated: "none".to_string(),
            state: "active".to_string(),
        };

        pipeline_configs::insert(&mut conn, &new_pipeline_config).unwrap();

        let task = Task {
            namespace_id: "default".into(),
            pipeline_id: "test".into(),
            pipeline_config_version: 1,
            task_id: "new_task_id".into(),
            description: "A new task".into(),
            image: "rust:1.43".into(),
            registry_auth: "auth_token".into(),
            depends_on: "task_id".into(),
            variables: "KEY=VALUE".into(),
            entrypoint: "/bin/sh".into(),
            command: "cargo test".into(),
            inject_api_token: false,
        };

        insert(&mut conn, &task)?;

        Ok((harness, conn))
    }

    fn test_list_pipeline_tasks() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let tasks = list(&mut conn, "default", "test", 1).expect("Failed to list tasks");
        assert_eq!(tasks.len(), 1, "Should list exactly one task");

        let task = &tasks[0];
        assert_eq!(task.task_id, "new_task_id", "Task ID should match");
        assert_eq!(task.pipeline_id, "test", "Pipeline should match");
    }

    fn test_get_pipeline_task() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fetched_task =
            get(&mut conn, "default", "test", 1, "new_task_id").expect("Failed to fetch task");

        assert_eq!(
            fetched_task.description, "A new task",
            "Descriptions should match"
        );
    }
}
