use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct SecretStorePipelineKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub key: String,
    pub created: String,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    secret_store_pipeline_key: &SecretStorePipelineKey,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO secret_store_pipeline_keys (namespace_id, pipeline_id, key, created) VALUES (?, ?, ?, ?);",
    )
    .bind(&secret_store_pipeline_key.namespace_id)
    .bind(&secret_store_pipeline_key.pipeline_id)
    .bind(&secret_store_pipeline_key.key)
    .bind(&secret_store_pipeline_key.created);

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
) -> Result<Vec<SecretStorePipelineKey>, StorageError> {
    let query = sqlx::query_as::<_, SecretStorePipelineKey>(
        "SELECT namespace_id, pipeline_id, key, created FROM secret_store_pipeline_keys \
        WHERE namespace_id = ? AND pipeline_id = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id);

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
    key: &str,
) -> Result<SecretStorePipelineKey, StorageError> {
    let query = sqlx::query_as::<_, SecretStorePipelineKey>(
        "SELECT namespace_id, pipeline_id, key, created FROM secret_store_pipeline_keys \
        WHERE namespace_id = ? AND pipeline_id = ? AND key = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(key);

    let sql = query.sql();

    query
        .fetch_one(conn)
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
        "DELETE FROM secret_store_pipeline_keys \
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

        let secret_store_pipeline_key = SecretStorePipelineKey {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &secret_store_pipeline_key).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_secret_store_pipeline_keys() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let secret_store_pipeline_keys = list(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to list secret_store_pipeline_keys");

        // Assert that we got at least one secret_store_pipeline_key back
        assert!(
            !secret_store_pipeline_keys.is_empty(),
            "No secret_store_pipeline_keys returned"
        );

        // Assuming you want to check if the inserted secret_store_pipeline_key is in the list
        let some_secret_store_pipeline_key = secret_store_pipeline_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("SecretStorePipelineKey not found");
        assert_eq!(
            some_secret_store_pipeline_key.pipeline_id,
            "some_pipeline_id"
        );
        assert_eq!(some_secret_store_pipeline_key.created, "some_time");
    }

    #[tokio::test]
    async fn test_get_secret_store_pipeline_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let secret_store_pipeline_key = get(&mut conn, "some_id", "some_pipeline_id", "some_id")
            .await
            .expect("Failed to get secret_store_pipeline_key");

        assert_eq!(secret_store_pipeline_key.key, "some_id");
        assert_eq!(secret_store_pipeline_key.created, "some_time");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", "non_existent")
                .await
                .is_err(),
            "Unexpectedly found a secret_store_pipeline_key"
        );
    }

    #[tokio::test]
    async fn test_delete_secret_store_pipeline_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", "some_id")
            .await
            .expect("Failed to delete secret_store_pipeline_key");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", "some_id")
                .await
                .is_err(),
            "SecretStorePipelineKey was not deleted"
        );
    }
}
