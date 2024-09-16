use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct ObjectStoreExtensionKey {
    pub extension_id: String,
    pub key: String,
    pub created: String,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    object_store_extension_key: &ObjectStoreExtensionKey,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO object_store_extension_keys (extension_id, key, created) VALUES (?, ?, ?);",
    )
    .bind(&object_store_extension_key.extension_id)
    .bind(&object_store_extension_key.key)
    .bind(&object_store_extension_key.created);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(
    conn: &mut SqliteConnection,
    extension_id: &str,
) -> Result<Vec<ObjectStoreExtensionKey>, StorageError> {
    let query = sqlx::query_as::<_, ObjectStoreExtensionKey>(
        "SELECT extension_id, key, created FROM object_store_extension_keys \
        WHERE extension_id = ? ORDER BY created ASC;",
    )
    .bind(extension_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn delete(
    conn: &mut SqliteConnection,
    extension_id: &str,
    key: &str,
) -> Result<(), StorageError> {
    let query =
        sqlx::query("DELETE FROM object_store_extension_keys WHERE extension_id = ? AND key = ?;")
            .bind(extension_id)
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

        let extension = crate::storage::extension_registrations::ExtensionRegistration {
            extension_id: "some_id".into(),
            image: "some_image".into(),
            registry_auth: "some_auth".into(),
            settings: "some_settings".into(),
            created: "some_date".into(),
            modified: "some_modified".into(),
            status: "some_status".into(),
            additional_roles: "some_role".into(),
            key_id: "some_key_id".into(),
        };

        crate::storage::extension_registrations::insert(&mut conn, &extension).await?;

        let object_store_extension_key = ObjectStoreExtensionKey {
            extension_id: "some_id".into(),
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &object_store_extension_key).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_object_store_extension_keys() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let object_store_extension_keys = list(&mut conn, "some_id")
            .await
            .expect("Failed to list object_store_extension_keys");

        // Assert that we got at least one object_store_extension_key back
        assert!(
            !object_store_extension_keys.is_empty(),
            "No object_store_extension_keys returned"
        );

        // Assuming you want to check if the inserted object_store_extension_key is in the list
        let some_object_store_extension_key = object_store_extension_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("ObjectStoreExtensionKey not found");
        assert_eq!(some_object_store_extension_key.created, "some_time");
    }

    #[tokio::test]
    async fn test_delete_object_store_extension_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_key_id")
            .await
            .expect("Failed to delete object_store_extension_key");
    }
}
