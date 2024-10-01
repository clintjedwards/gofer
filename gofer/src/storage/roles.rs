use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct Role {
    pub id: String,
    pub description: String,
    pub permissions: String,
    pub system_role: bool,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub description: Option<String>,
    pub permissions: Option<String>,
}

pub async fn insert(conn: &mut SqliteConnection, role: &Role) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO roles (id, description, permissions, system_role) VALUES (?, ?, ?, ?);",
    )
    .bind(&role.id)
    .bind(&role.description)
    .bind(&role.permissions)
    .bind(role.system_role);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<Role>, StorageError> {
    let query =
        sqlx::query_as::<_, Role>("SELECT id, description, permissions, system_role FROM roles;");

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(conn: &mut SqliteConnection, id: &str) -> Result<Role, StorageError> {
    let query = sqlx::query_as::<_, Role>(
        "SELECT id, description, permissions, system_role FROM roles WHERE id = ?;",
    )
    .bind(id);

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
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE roles SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.permissions {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("permissions = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.description {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("description = ");
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
    let query = sqlx::query("DELETE FROM roles WHERE id = ?;").bind(id);

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

        let role = Role {
            id: "some_id".into(),
            description: "some_description".into(),
            permissions: "permissioning".into(),
            system_role: false,
        };

        insert(&mut conn, &role).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_roles() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let roles = list(&mut conn).await.expect("Failed to list roles");

        // Assert that we got at least one role back
        assert!(!roles.is_empty(), "No roles returned");

        for role in roles {
            match role.id.as_str() {
                "some_id" => {
                    assert_eq!(role.permissions, "permissioning");
                }
                _ => panic!("Unexpected role"),
            }
        }
    }

    #[tokio::test]
    async fn test_update_roles() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            description: None,
            permissions: Some("some_permissioning".into()),
        };

        update(&mut conn, "some_id", fields_to_update.clone())
            .await
            .expect("Failed to update role");

        let updated_role = get(&mut conn, "some_id")
            .await
            .expect("Failed to retrieve updated role");

        assert_eq!(
            fields_to_update.permissions.unwrap(),
            updated_role.permissions
        );
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fetched_role = get(&mut conn, "some_id").await.expect("Failed to get Role");
        assert_eq!(fetched_role.id, "some_id");
        assert_eq!(fetched_role.permissions, "permissioning",);
    }

    #[tokio::test]
    async fn test_delete() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id")
            .await
            .expect("Failed to delete Role");

        let result = get(&mut conn, "some_id").await;
        assert!(matches!(result, Err(StorageError::NotFound)));
    }
}
