use crate::storage::{StorageError, map_sqlx_error};
use futures::TryFutureExt;
use sqlx::{FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct ImageBlob {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub digest: String,
    pub diff_id: String,
    pub size: i64,
    pub last_seen: String,
}

/// Uploading a blob again is fine, it just counts as having seen it again.
pub async fn upsert(
    conn: &mut SqliteConnection,
    image_blob: &ImageBlob,
) -> Result<(), StorageError> {
    let sql = "INSERT INTO image_blobs (namespace_id, pipeline_id, digest, diff_id, size, last_seen) \
        VALUES (?, ?, ?, ?, ?, ?) \
        ON CONFLICT (namespace_id, pipeline_id, digest) \
        DO UPDATE SET diff_id = excluded.diff_id, size = excluded.size, last_seen = excluded.last_seen;";

    let query = sqlx::query(sql)
        .bind(&image_blob.namespace_id)
        .bind(&image_blob.pipeline_id)
        .bind(&image_blob.digest)
        .bind(&image_blob.diff_id)
        .bind(image_blob.size)
        .bind(&image_blob.last_seen);

    query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    digest: &str,
) -> Result<ImageBlob, StorageError> {
    let sql = "SELECT namespace_id, pipeline_id, digest, diff_id, size, last_seen FROM image_blobs \
        WHERE namespace_id = ? AND pipeline_id = ? AND digest = ?;";

    let query = sqlx::query_as::<_, ImageBlob>(sql)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(digest);

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update_last_seen(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    digest: &str,
    last_seen: &str,
) -> Result<(), StorageError> {
    let sql = "UPDATE image_blobs SET last_seen = ? \
    WHERE namespace_id = ? AND pipeline_id = ? AND digest = ?;";

    let query = sqlx::query(sql)
        .bind(last_seen)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(digest);

    let result = query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    // Callers rely on this to tell which blobs a pipeline hasn't uploaded yet.
    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound);
    }

    Ok(())
}

/// The manifests of the images a pipeline keeps: those of its newest `newest` versions, and of any version that's live,
/// being deployed, or has runs that haven't finished.
pub async fn list_kept_images(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    newest: i64,
) -> Result<Vec<String>, StorageError> {
    let sql = "SELECT DISTINCT image_digest FROM tasks \
        WHERE namespace_id = ? AND pipeline_id = ? AND pipeline_config_version IN ( \
            SELECT version FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? AND ( \
                version IN (SELECT version FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? \
                    ORDER BY version DESC LIMIT ?) \
                OR state = 'live' \
                OR version IN (SELECT start_version FROM deployments WHERE namespace_id = ? AND pipeline_id = ? \
                    AND state = 'running') \
                OR version IN (SELECT end_version FROM deployments WHERE namespace_id = ? AND pipeline_id = ? \
                    AND state = 'running') \
                OR version IN (SELECT pipeline_config_version FROM runs WHERE namespace_id = ? AND pipeline_id = ? \
                    AND state != 'complete') \
            ) \
        );";

    // Every subquery is for this one pipeline, so each needs it again.
    let query = sqlx::query_scalar::<_, String>(sql)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(newest)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(namespace_id)
        .bind(pipeline_id);

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

/// The blobs a pipeline was last told about, or uploaded, before `before`, in epoch milliseconds.
pub async fn list_seen_before(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    before: u64,
) -> Result<Vec<String>, StorageError> {
    let sql = "SELECT digest FROM image_blobs \
        WHERE namespace_id = ? AND pipeline_id = ? AND CAST(last_seen AS INTEGER) < ?;";

    let query = sqlx::query_scalar::<_, String>(sql)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(before as i64);

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

/// Every pipeline that has blobs, including ones that were never registered.
pub async fn list_pipelines(
    conn: &mut SqliteConnection,
) -> Result<Vec<(String, String)>, StorageError> {
    let sql = "SELECT DISTINCT namespace_id, pipeline_id FROM image_blobs;";

    sqlx::query_as::<_, (String, String)>(sql)
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

/// Whether any pipeline has this blob.
pub async fn is_used(conn: &mut SqliteConnection, digest: &str) -> Result<bool, StorageError> {
    let sql = "SELECT EXISTS (SELECT 1 FROM image_blobs WHERE digest = ?);";

    sqlx::query_scalar::<_, bool>(sql)
        .bind(digest)
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    digest: &str,
) -> Result<(), StorageError> {
    let sql = "DELETE FROM image_blobs WHERE namespace_id = ? AND pipeline_id = ? AND digest = ?;";

    sqlx::query(sql)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(digest)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

/// Drops all of a pipeline's blobs, for when the pipeline is deleted. They have no foreign key to it, so this doesn't
/// happen on its own.
pub async fn delete_all(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<(), StorageError> {
    let sql = "DELETE FROM image_blobs WHERE namespace_id = ? AND pipeline_id = ?;";

    sqlx::query(sql)
        .bind(namespace_id)
        .bind(pipeline_id)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;
    use sqlx::{Sqlite, pool::PoolConnection};

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

        // The pipeline is missing on purpose; a new pipeline's blobs are uploaded before it exists.
        let image_blob = ImageBlob {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            digest: "some_digest".into(),
            diff_id: "some_diff_id".into(),
            size: 10,
            last_seen: "some_time".into(),
        };

        upsert(&mut conn, &image_blob).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_get_image_blob() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let image_blob = get(&mut conn, "some_id", "some_pipeline_id", "some_digest")
            .await
            .expect("Failed to get image blob");

        assert_eq!(image_blob.diff_id, "some_diff_id");
        assert_eq!(image_blob.size, 10);
        assert_eq!(image_blob.last_seen, "some_time");
    }

    #[tokio::test]
    async fn test_image_blobs_belong_to_one_pipeline() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let result = get(&mut conn, "some_id", "other_pipeline_id", "some_digest").await;

        assert_eq!(result.unwrap_err(), StorageError::NotFound);
    }

    #[tokio::test]
    async fn test_upsert_existing_image_blob() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let image_blob = ImageBlob {
            last_seen: "some_new_time".into(),
            ..get(&mut conn, "some_id", "some_pipeline_id", "some_digest")
                .await
                .expect("Failed to get image blob")
        };

        upsert(&mut conn, &image_blob)
            .await
            .expect("Failed to upsert image blob");

        let image_blob = get(&mut conn, "some_id", "some_pipeline_id", "some_digest")
            .await
            .expect("Failed to get image blob");

        assert_eq!(image_blob.last_seen, "some_new_time");
    }

    #[tokio::test]
    async fn test_update_last_seen_image_blob() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        update_last_seen(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            "some_digest",
            "some_new_time",
        )
        .await
        .expect("Failed to update image blob");

        let image_blob = get(&mut conn, "some_id", "some_pipeline_id", "some_digest")
            .await
            .expect("Failed to get image blob");

        assert_eq!(image_blob.last_seen, "some_new_time");

        let result = update_last_seen(
            &mut conn,
            "some_id",
            "other_pipeline_id",
            "some_digest",
            "some_new_time",
        )
        .await;

        assert_eq!(result.unwrap_err(), StorageError::NotFound);
    }
}
