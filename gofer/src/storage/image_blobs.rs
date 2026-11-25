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
