use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct Token {
    pub id: String,
    pub hash: String,
    pub created: String,
    pub token_type: String,
    pub namespaces: String,
    pub metadata: String,
    pub expires: String,
    pub disabled: bool,
    pub user: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub disabled: Option<bool>,
}

pub async fn insert(conn: &mut SqliteConnection, token: &Token) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO tokens (id, hash, created, token_type, namespaces, metadata, expires, disabled, user)\
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .bind(&token.id)
    .bind(&token.hash)
    .bind(&token.created)
    .bind(&token.token_type)
    .bind(&token.namespaces)
    .bind(&token.metadata)
    .bind(&token.expires)
    .bind(token.disabled)
    .bind(&token.user);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn management_token_exists(conn: &mut SqliteConnection) -> Result<bool, StorageError> {
    let query = sqlx::query_as::<_, Token>(
        "SELECT id, hash, created, token_type, \
        namespaces, metadata, expires, disabled, user FROM tokens WHERE token_type = 'management';",
    );

    let sql = query.sql();

    let result = query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await;

    if let Err(e) = result {
        match e {
            StorageError::NotFound => return Ok(false),
            _ => return Err(e),
        }
    };

    Ok(true)
}

pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<Token>, StorageError> {
    let query = sqlx::query_as::<_, Token>(
        "SELECT id, hash, created, token_type, namespaces, metadata, expires, disabled, user FROM tokens;",
    );

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get_by_id(conn: &mut SqliteConnection, id: &str) -> Result<Token, StorageError> {
    let query = sqlx::query_as::<_, Token>(
        "SELECT id, hash, created, token_type, namespaces, metadata, expires, \
        disabled, user FROM tokens WHERE id = ?;",
    )
    .bind(id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get_by_hash(conn: &mut SqliteConnection, hash: &str) -> Result<Token, StorageError> {
    let query = sqlx::query_as::<_, Token>(
        "SELECT id, hash, created, token_type, namespaces, metadata, expires,\
        disabled, user FROM tokens WHERE hash = ?;",
    )
    .bind(hash);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update(
    conn: &mut SqliteConnection,
    id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE tokens SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.disabled {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("disabled = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(" WHERE id = ");
    update_query.push_bind(id);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

pub async fn delete(conn: &mut SqliteConnection, id: &str) -> Result<(), StorageError> {
    let query = sqlx::query("DELETE FROM tokens WHERE id = ?;").bind(id);

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

        let token = Token {
            id: "some_id".into(),
            hash: "some_hash".into(),
            created: "some_time".into(),
            token_type: "management".into(),
            namespaces: "some_json_list".into(),
            metadata: "some_json_hashmap".into(),
            expires: "some_expiry".into(),
            user: "some_user".into(),
            disabled: false,
        };

        insert(&mut conn, &token).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_management_token_exists() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let exists = management_token_exists(&mut conn)
            .await
            .expect("Failed to check for management token");

        assert!(exists);

        delete(&mut conn, "some_id")
            .await
            .expect("Failed to delete Token");

        let exists = management_token_exists(&mut conn)
            .await
            .expect("Failed to check for management token");

        assert!(!exists);
    }

    #[tokio::test]
    async fn test_list_tokens() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let tokens = list(&mut conn).await.expect("Failed to list tokens");

        // Assert that we got at least one token back
        assert!(!tokens.is_empty(), "No tokens returned");

        for token in tokens {
            match token.id.as_str() {
                "some_id" => {
                    assert_eq!(token.hash, "some_hash");
                    assert_eq!(token.metadata, "some_json_hashmap");
                }
                _ => panic!("Unexpected token"),
            }
        }
    }

    #[tokio::test]
    async fn test_update_tokens() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            disabled: Some(true),
        };

        update(&mut conn, "some_id", fields_to_update)
            .await
            .expect("Failed to update token");

        let updated_token = get_by_id(&mut conn, "some_id")
            .await
            .expect("Failed to retrieve updated token");

        assert!(updated_token.disabled);
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fetched_token = get_by_id(&mut conn, "some_id")
            .await
            .expect("Failed to get Token");
        assert_eq!(fetched_token.id, "some_id");
        assert_eq!(fetched_token.namespaces, "some_json_list");
    }

    #[tokio::test]
    async fn test_delete() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id")
            .await
            .expect("Failed to delete Token");

        let result = get_by_id(&mut conn, "some_id").await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }
}
