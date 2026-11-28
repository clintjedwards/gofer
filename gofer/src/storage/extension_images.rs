//! The extension images stored in Gofer, and the blobs uploaded for them.

use crate::storage::{StorageError, map_sqlx_error};
use futures::TryFutureExt;
use sqlx::{FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow, PartialEq, Eq)]
pub struct ExtensionImage {
    pub name: String,
    pub tag: String,

    /// The digest of the image's manifest.
    pub digest: String,

    /// Gofer's own extensions, stored from its binary when it starts.
    pub builtin: bool,

    pub created: String,
}

#[derive(Clone, Debug, Default, FromRow)]
pub struct ExtensionImageBlob {
    pub digest: String,
    pub diff_id: String,
    pub size: i64,
    pub last_seen: String,
}

/// Points the name and tag at this image, whether or not they were taken. Callers decide whether replacing is allowed.
pub async fn upsert(
    conn: &mut SqliteConnection,
    image: &ExtensionImage,
) -> Result<(), StorageError> {
    let sql = "INSERT INTO extension_images (name, tag, digest, builtin, created) VALUES (?, ?, ?, ?, ?) \
        ON CONFLICT (name, tag) DO UPDATE SET digest = excluded.digest, builtin = excluded.builtin, \
        created = excluded.created;";

    sqlx::query(sql)
        .bind(&image.name)
        .bind(&image.tag)
        .bind(&image.digest)
        .bind(image.builtin)
        .bind(&image.created)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    name: &str,
    tag: &str,
) -> Result<ExtensionImage, StorageError> {
    let sql = "SELECT name, tag, digest, builtin, created FROM extension_images WHERE name = ? AND tag = ?;";

    sqlx::query_as::<_, ExtensionImage>(sql)
        .bind(name)
        .bind(tag)
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<ExtensionImage>, StorageError> {
    let sql =
        "SELECT name, tag, digest, builtin, created FROM extension_images ORDER BY name, tag;";

    sqlx::query_as::<_, ExtensionImage>(sql)
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn delete(
    conn: &mut SqliteConnection,
    name: &str,
    tag: &str,
) -> Result<(), StorageError> {
    let sql = "DELETE FROM extension_images WHERE name = ? AND tag = ?;";

    sqlx::query(sql)
        .bind(name)
        .bind(tag)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

/// Drops the built-in images an earlier version of Gofer stored, which have a different tag from this one's.
pub async fn delete_other_builtins(
    conn: &mut SqliteConnection,
    tag: &str,
) -> Result<(), StorageError> {
    let sql = "DELETE FROM extension_images WHERE builtin = 1 AND tag != ?;";

    sqlx::query(sql)
        .bind(tag)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

/// Uploading a blob again is fine, it just counts as having seen it again.
pub async fn upsert_blob(
    conn: &mut SqliteConnection,
    blob: &ExtensionImageBlob,
) -> Result<(), StorageError> {
    let sql = "INSERT INTO extension_image_blobs (digest, diff_id, size, last_seen) VALUES (?, ?, ?, ?) \
        ON CONFLICT (digest) DO UPDATE SET diff_id = excluded.diff_id, size = excluded.size, \
        last_seen = excluded.last_seen;";

    sqlx::query(sql)
        .bind(&blob.digest)
        .bind(&blob.diff_id)
        .bind(blob.size)
        .bind(&blob.last_seen)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get_blob(
    conn: &mut SqliteConnection,
    digest: &str,
) -> Result<ExtensionImageBlob, StorageError> {
    let sql =
        "SELECT digest, diff_id, size, last_seen FROM extension_image_blobs WHERE digest = ?;";

    sqlx::query_as::<_, ExtensionImageBlob>(sql)
        .bind(digest)
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update_blob_last_seen(
    conn: &mut SqliteConnection,
    digest: &str,
    last_seen: &str,
) -> Result<(), StorageError> {
    let sql = "UPDATE extension_image_blobs SET last_seen = ? WHERE digest = ?;";

    let result = sqlx::query(sql)
        .bind(last_seen)
        .bind(digest)
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    // Callers rely on this to tell which blobs haven't been uploaded yet.
    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound);
    }

    Ok(())
}

/// The blobs last told about, or uploaded, before `before`, in epoch milliseconds.
pub async fn list_blobs_seen_before(
    conn: &mut SqliteConnection,
    before: u64,
) -> Result<Vec<String>, StorageError> {
    let sql = "SELECT digest FROM extension_image_blobs WHERE CAST(last_seen AS INTEGER) < ?;";

    sqlx::query_scalar::<_, String>(sql)
        .bind(before as i64)
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn delete_blob(conn: &mut SqliteConnection, digest: &str) -> Result<(), StorageError> {
    let sql = "DELETE FROM extension_image_blobs WHERE digest = ?;";

    sqlx::query(sql)
        .bind(digest)
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;

    fn image(name: &str, tag: &str, digest: &str, builtin: bool) -> ExtensionImage {
        ExtensionImage {
            name: name.into(),
            tag: tag.into(),
            digest: digest.into(),
            builtin,
            created: "some_time".into(),
        }
    }

    #[tokio::test]
    async fn test_upsert_points_a_name_and_tag_at_another_image() {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        upsert(&mut conn, &image("slack", "1.0", "first", false))
            .await
            .unwrap();
        upsert(&mut conn, &image("slack", "1.0", "second", false))
            .await
            .unwrap();

        assert_eq!(
            get(&mut conn, "slack", "1.0").await.unwrap().digest,
            "second"
        );
        assert_eq!(list(&mut conn).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_other_builtins() {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        for stored in [
            image("cron", "0.15.0", "old", true),
            image("cron", "0.16.0", "new", true),
            image("slack", "0.15.0", "pushed", false),
        ] {
            upsert(&mut conn, &stored).await.unwrap();
        }

        delete_other_builtins(&mut conn, "0.16.0").await.unwrap();

        let left: Vec<(String, String)> = (list(&mut conn).await.unwrap().into_iter())
            .map(|image| (image.name, image.tag))
            .collect();
        assert_eq!(
            left,
            vec![
                ("cron".into(), "0.16.0".into()),
                ("slack".into(), "0.15.0".into())
            ]
        );
    }

    #[tokio::test]
    async fn test_blobs_seen_before() {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        for (digest, last_seen) in [("old", "100"), ("new", "300")] {
            upsert_blob(
                &mut conn,
                &ExtensionImageBlob {
                    digest: digest.into(),
                    diff_id: "some_diff_id".into(),
                    size: 10,
                    last_seen: last_seen.into(),
                },
            )
            .await
            .unwrap();
        }

        assert_eq!(
            list_blobs_seen_before(&mut conn, 200).await.unwrap(),
            vec!["old"]
        );

        update_blob_last_seen(&mut conn, "old", "400")
            .await
            .unwrap();
        assert!(
            list_blobs_seen_before(&mut conn, 200)
                .await
                .unwrap()
                .is_empty()
        );

        let missing = update_blob_last_seen(&mut conn, "missing", "400").await;
        assert_eq!(missing.unwrap_err(), StorageError::NotFound);
    }
}
