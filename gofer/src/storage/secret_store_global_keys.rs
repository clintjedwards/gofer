use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct SecretStoreGlobalKey {
    pub key: String,
    pub namespaces: String,
    pub created: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub namespaces: Option<String>,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    secret_store_global_key: &SecretStoreGlobalKey,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO secret_store_global_keys (key, namespaces, created) VALUES (?, ?, ?);",
    )
    .bind(&secret_store_global_key.key)
    .bind(&secret_store_global_key.namespaces)
    .bind(&secret_store_global_key.created);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<SecretStoreGlobalKey>, StorageError> {
    let query = sqlx::query_as::<_, SecretStoreGlobalKey>(
        "SELECT key, namespaces, created FROM secret_store_global_keys;",
    );

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    key: &str,
) -> Result<SecretStoreGlobalKey, StorageError> {
    let query = sqlx::query_as::<_, SecretStoreGlobalKey>(
        "SELECT key, namespaces, created FROM secret_store_global_keys WHERE key = ?;",
    )
    .bind(key);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

// For now we don't allow users to update the namespaces for their global key, but we keep this around because one day
// we might.
#[allow(dead_code)]
pub async fn update(
    conn: &mut SqliteConnection,
    key: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE secret_store_global_keys SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.namespaces {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("namespaces = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(" WHERE key = ");
    update_query.push_bind(key);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

pub async fn delete(conn: &mut SqliteConnection, key: &str) -> Result<(), StorageError> {
    let query = sqlx::query("DELETE FROM secret_store_global_keys WHERE key = ?;").bind(key);

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

        let secret_store_global_key = SecretStoreGlobalKey {
            key: "some_id".into(),
            namespaces: "some_name".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &secret_store_global_key).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_secret_store_global_keys() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let secret_store_global_keys = list(&mut conn)
            .await
            .expect("Failed to list secret_store_global_keys");

        // Assert that we got at least one secret_store_global_key back
        assert!(
            !secret_store_global_keys.is_empty(),
            "No secret_store_global_keys returned"
        );

        // Assuming you want to check if the inserted secret_store_global_key is in the list
        let some_secret_store_global_key = secret_store_global_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("SecretStoreGlobalKey not found");
        assert_eq!(some_secret_store_global_key.namespaces, "some_name");
        assert_eq!(some_secret_store_global_key.created, "some_time");
    }

    #[tokio::test]
    async fn test_get_secret_store_global_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let secret_store_global_key = get(&mut conn, "some_id")
            .await
            .expect("Failed to get secret_store_global_key");

        assert_eq!(secret_store_global_key.key, "some_id");
        assert_eq!(secret_store_global_key.created, "some_time");

        assert!(
            get(&mut conn, "non_existent").await.is_err(),
            "Unexpectedly found a secret_store_global_key"
        );
    }

    #[tokio::test]
    async fn test_update_secret_store_global_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            namespaces: Some("some_namespace".into()),
        };

        update(&mut conn, "some_id", fields_to_update)
            .await
            .expect("Failed to update secret_store_global_key");

        let updated_secret_store_global_key = get(&mut conn, "some_id")
            .await
            .expect("Failed to retrieve updated secret_store_global_key");

        assert_eq!(updated_secret_store_global_key.namespaces, "some_namespace");
        assert_eq!(updated_secret_store_global_key.created, "some_time");
    }

    #[tokio::test]
    async fn test_delete_secret_store_global_key() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id")
            .await
            .expect("Failed to delete secret_store_global_key");

        assert!(
            get(&mut conn, "some_id").await.is_err(),
            "SecretStoreGlobalKey was not deleted"
        );
    }
}
