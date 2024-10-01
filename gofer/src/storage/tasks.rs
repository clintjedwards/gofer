use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
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

pub async fn insert(conn: &mut SqliteConnection, task: &Task) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO tasks (namespace_id, pipeline_id, pipeline_config_version, task_id, \
        description, image, registry_auth, depends_on, variables, entrypoint, command, \
        inject_api_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .bind(&task.namespace_id)
    .bind(&task.pipeline_id)
    .bind(task.pipeline_config_version)
    .bind(&task.task_id)
    .bind(&task.description)
    .bind(&task.image)
    .bind(&task.registry_auth)
    .bind(&task.depends_on)
    .bind(&task.variables)
    .bind(&task.entrypoint)
    .bind(&task.command)
    .bind(task.inject_api_token);

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
    version: i64,
) -> Result<Vec<Task>, StorageError> {
    let query = sqlx::query_as::<_, Task>(
        "SELECT namespace_id, pipeline_id, pipeline_config_version, task_id, description, image, \
        registry_auth, depends_on, variables, entrypoint, command, inject_api_token FROM \
        tasks WHERE namespace_id = ? AND pipeline_id = ? AND pipeline_config_version = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(version);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

// Currently a task is embedded within a task_execution anyway so there isn't any need for a user to ever just get a
// task. But for the sake of standardization we'll keep this crud function here.
#[allow(dead_code)]
pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
    task_id: &str,
) -> Result<Task, StorageError> {
    let query = sqlx::query_as::<_, Task>("SELECT namespace_id, pipeline_id, pipeline_config_version, task_id, description, image, \
        registry_auth, depends_on, variables, entrypoint, command, inject_api_token FROM \
        tasks WHERE namespace_id = ? AND pipeline_id = ? AND pipeline_config_version = ? AND task_id = ?;")
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(version)
        .bind(task_id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        namespaces::{self, Namespace},
        pipeline_configs::{self, PipelineConfig},
        pipeline_metadata::{self, PipelineMetadata},
        tests::TestHarness,
    };
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        let namespace = Namespace {
            id: "default".into(),
            name: "Default".into(),
            description: String::new(),
            created: String::new(),
            modified: String::new(),
        };

        namespaces::insert(&mut conn, &namespace).await.unwrap();

        let pipeline = PipelineMetadata {
            namespace_id: "default".into(),
            pipeline_id: "test".into(),
            state: String::new(),
            created: String::new(),
            modified: String::new(),
        };

        pipeline_metadata::insert(&mut conn, &pipeline)
            .await
            .unwrap();

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

        pipeline_configs::insert(&mut conn, &new_pipeline_config)
            .await
            .unwrap();

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

        insert(&mut conn, &task).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_pipeline_tasks() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let tasks = list(&mut conn, "default", "test", 1)
            .await
            .expect("Failed to list tasks");
        assert_eq!(tasks.len(), 1, "Should list exactly one task");

        let task = &tasks[0];
        assert_eq!(task.task_id, "new_task_id", "Task ID should match");
        assert_eq!(task.pipeline_id, "test", "Pipeline should match");
    }

    #[tokio::test]
    async fn test_get_pipeline_task() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fetched_task = get(&mut conn, "default", "test", 1, "new_task_id")
            .await
            .expect("Failed to fetch task");

        assert_eq!(
            fetched_task.description, "A new task",
            "Descriptions should match"
        );
    }
}
