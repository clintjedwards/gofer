use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

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

pub async fn insert(
    conn: &mut SqliteConnection,
    task_execution: &TaskExecution,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO task_executions (namespace_id, pipeline_id, run_id, task_id, task, created, started, ended, \
            exit_code, logs_expired, logs_removed, state, status, status_reason, variables) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .bind(&task_execution.namespace_id)
    .bind(&task_execution.pipeline_id)
    .bind(task_execution.run_id)
    .bind(&task_execution.task_id)
    .bind(&task_execution.task)
    .bind(&task_execution.created)
    .bind(&task_execution.started)
    .bind(&task_execution.ended)
    .bind(task_execution.exit_code)
    .bind(task_execution.logs_expired)
    .bind(task_execution.logs_removed)
    .bind(&task_execution.state)
    .bind(&task_execution.status)
    .bind(&task_execution.status_reason)
    .bind(&task_execution.variables);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
) -> Result<Vec<TaskExecution>, StorageError> {
    let query = sqlx::query_as::<_, TaskExecution>(
        "SELECT namespace_id, pipeline_id, run_id, task_id, task, created, started, ended, exit_code, logs_expired, \
        logs_removed, state, status, status_reason, variables FROM task_executions \
        WHERE namespace_id = ? AND pipeline_id = ? AND run_id = ?;",
    )
    .bind(namespace_id).bind(pipeline_id).bind(run_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    task_id: &str,
) -> Result<TaskExecution, StorageError> {
    let query = sqlx::query_as(
        "SELECT namespace_id, pipeline_id, run_id, task_id, task, created, started, ended, exit_code, logs_expired, \
        logs_removed, state, status, status_reason, variables FROM task_executions \
        WHERE namespace_id = ? AND pipeline_id = ? AND run_id = ? AND task_id = ?;"
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(run_id)
    .bind(task_id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    task_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE task_executions SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.started {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("started = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.ended {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("ended = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.exit_code {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("exit_code = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.state {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("state = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.status {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.status_reason {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status_reason = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.logs_expired {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("logs_expired = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.logs_removed {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("logs_removed = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.variables {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("variables = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(" WHERE namespace_id = ");
    update_query.push_bind(namespace_id);
    update_query.push(" AND pipeline_id = ");
    update_query.push_bind(pipeline_id);
    update_query.push(" AND run_id = ");
    update_query.push_bind(run_id);
    update_query.push(" AND task_id = ");
    update_query.push_bind(task_id);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

// For now we don't allow deletion of task_executions and there really shouldn't be a need for it, but in the future
// we might allow it through an admin route.
#[allow(dead_code)]
pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    task_id: &str,
) -> Result<(), StorageError> {
    let query =
        sqlx::query("DELETE FROM task_executions WHERE namespace_id = ? AND pipeline_id = ? AND run_id = ? AND task_id = ?;")
            .bind(namespace_id)
            .bind(pipeline_id).bind(run_id).bind(task_id);

    let sql = query.sql();

    query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_pipeline_id".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace).await?;

        let pipeline_metadata = crate::storage::pipeline_metadata::PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::pipeline_metadata::insert(&mut conn, &pipeline_metadata).await?;

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
            .await
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

        crate::storage::runs::insert(&mut conn, &run).await?;

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

        insert(&mut conn, &task_execution).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_task_executions() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let task_executions = list(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to list task_executions");

        assert!(!task_executions.is_empty(), "No task_executions returned");

        let some_task_execution = task_executions
            .iter()
            .find(|n| n.task_id == "task001")
            .expect("TaskExecution not found");
        assert_eq!(some_task_execution.pipeline_id, "some_pipeline_id");
        assert_eq!(some_task_execution.state, "some_state");
    }

    #[tokio::test]
    async fn test_get_task_execution() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let task_execution = get(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
            .await
            .expect("Failed to get task_execution");

        assert_eq!(task_execution.pipeline_id, "some_pipeline_id");
    }

    #[tokio::test]
    async fn test_update_task_execution() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

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
        .await
        .expect("Failed to update task_execution");

        let updated_task_execution = get(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
            .await
            .expect("Failed to retrieve updated task_execution");

        assert_eq!(updated_task_execution.state, "updated_state");
    }

    #[tokio::test]
    async fn test_delete_task_execution() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
            .await
            .expect("Failed to delete task_execution");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", 1, "task001")
                .await
                .is_err(),
            "TaskExecution was not deleted"
        );
    }
}
