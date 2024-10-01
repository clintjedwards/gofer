use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct ObjectStoreRunKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub run_id: i64,
    pub key: String,
    pub created: String,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    object_store_run_key: &ObjectStoreRunKey,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO object_store_run_keys (namespace_id, pipeline_id, run_id, key, created) VALUES (?, ?, ?, ?, ?);",
    )
    .bind(&object_store_run_key.namespace_id)
    .bind(&object_store_run_key.pipeline_id)
    .bind(object_store_run_key.run_id)
    .bind(&object_store_run_key.key)
    .bind(&object_store_run_key.created);

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
) -> Result<Vec<ObjectStoreRunKey>, StorageError> {
    let query = sqlx::query_as::<_, ObjectStoreRunKey>(
        "SELECT namespace_id, pipeline_id, run_id, key, created FROM object_store_run_keys \
        WHERE namespace_id = ? AND pipeline_id = ? AND run_id = ? ORDER BY created ASC;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(run_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    key: &str,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "DELETE FROM object_store_run_keys WHERE namespace_id = ? AND pipeline_id = ? AND run_id = ? AND key = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(run_id)
    .bind(key);

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
            name: "some_name".into(),
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

        let object_store_run_key = ObjectStoreRunKey {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            run_id: 1,
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &object_store_run_key).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_object_store_run_keys() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let object_store_run_keys = list(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to list object_store_run_keys");

        // Assert that we got at least one object_store_run_key back
        assert!(
            !object_store_run_keys.is_empty(),
            "No object_store_run_keys returned"
        );

        // Assuming you want to check if the inserted object_store_run_key is in the list
        let some_object_store_run_key = object_store_run_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("ObjectStoreRunKey not found");
        assert_eq!(some_object_store_run_key.pipeline_id, "some_pipeline_id");
        assert_eq!(some_object_store_run_key.created, "some_time");
    }

    #[tokio::test]
    async fn test_delete_object_store_run_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1, "some_id")
            .await
            .expect("Failed to delete object_store_run_key");
    }
}
