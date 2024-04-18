use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct ObjectStorePipelineKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub key: String,
    pub created: String,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    object_store_pipeline_key: &ObjectStorePipelineKey,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO object_store_pipeline_keys (namespace_id, pipeline_id, key, created) VALUES (?, ?, ?, ?);",
    )
    .bind(&object_store_pipeline_key.namespace_id)
    .bind(&object_store_pipeline_key.pipeline_id)
    .bind(&object_store_pipeline_key.key)
    .bind(&object_store_pipeline_key.created);

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
) -> Result<Vec<ObjectStorePipelineKey>, StorageError> {
    let query = sqlx::query_as::<_, ObjectStorePipelineKey>(
        "SELECT namespace_id, pipeline_id, key, created FROM object_store_pipeline_keys \
        WHERE namespace_id = ? AND pipeline_id = ? ORDER BY created ASC;",
    )
    .bind(namespace_id)
    .bind(pipeline_id);

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
    key: &str,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "DELETE FROM object_store_pipeline_keys \
    WHERE namespace_id = ? AND pipeline_id = ? AND key = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
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
        let mut conn = harness.conn().await.unwrap();

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

        let object_store_pipeline_key = ObjectStorePipelineKey {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &object_store_pipeline_key).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_object_store_pipeline_keys() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let object_store_pipeline_keys = list(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to list object_store_pipeline_keys");

        // Assert that we got at least one object_store_pipeline_key back
        assert!(
            !object_store_pipeline_keys.is_empty(),
            "No object_store_pipeline_keys returned"
        );

        // Assuming you want to check if the inserted object_store_pipeline_key is in the list
        let some_object_store_pipeline_key = object_store_pipeline_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("ObjectStorePipelineKey not found");
        assert_eq!(
            some_object_store_pipeline_key.pipeline_id,
            "some_pipeline_id"
        );
        assert_eq!(some_object_store_pipeline_key.created, "some_time");
    }

    #[tokio::test]
    async fn test_delete_object_store_pipeline_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", "some_id")
            .await
            .expect("Failed to delete object_store_pipeline_key");
    }
}
