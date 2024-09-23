use crate::storage::{epoch_milli, map_sqlx_error, StorageError};
use futures::TryFutureExt;
use rusqlite::Connection;

#[derive(Clone, Debug, Default)]
pub struct Namespace {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created: String,
    pub modified: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub modified: String,
}

impl Default for UpdatableFields {
    fn default() -> Self {
        Self {
            name: Default::default(),
            description: Default::default(),
            modified: epoch_milli().to_string(),
        }
    }
}

pub async fn insert(conn: &mut Connection, namespace: &Namespace) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO namespaces (id, name, description, created, modified) VALUES (?, ?, ?, ?, ?);",
    )
    .bind(&namespace.id)
    .bind(&namespace.name)
    .bind(&namespace.description)
    .bind(&namespace.created)
    .bind(&namespace.modified);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<Namespace>, StorageError> {
    let query = sqlx::query_as::<_, Namespace>(
        "SELECT id, name, description, created, modified FROM namespaces;",
    );

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(conn: &mut SqliteConnection, id: &str) -> Result<Namespace, StorageError> {
    let query = sqlx::query_as::<_, Namespace>(
        "SELECT id, name, description, created, modified FROM namespaces WHERE id = ?;",
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
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE namespaces SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.name {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("name = ");
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

    update_query.push(", ");
    update_query.push("modified = ");
    update_query.push_bind(fields.modified);

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
    let query = sqlx::query("DELETE FROM namespaces WHERE id = ?;").bind(id);

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

        let namespace = Namespace {
            id: "some_id".into(),
            name: "some_name".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        insert(&mut conn, &namespace).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_namespaces() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let namespaces = list(&mut conn).await.expect("Failed to list namespaces");

        // Assert that we got at least one namespace back
        assert!(!namespaces.is_empty(), "No namespaces returned");

        // Assuming you want to check if the inserted namespace is in the list
        let some_namespace = namespaces
            .iter()
            .find(|n| n.id == "some_id")
            .expect("Namespace not found");
        assert_eq!(some_namespace.name, "some_name");
        assert_eq!(some_namespace.description, "some_description");
    }

    #[tokio::test]
    async fn test_insert_namespace() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let new_namespace = Namespace {
            id: "new_id".into(),
            name: "new_name".into(),
            description: "new_description".into(),
            created: "some_time".into(),
            modified: "some_other_time".into(),
        };

        insert(&mut conn, &new_namespace)
            .await
            .expect("Failed to insert namespace");

        let retrieved_namespace = get(&mut conn, "new_id")
            .await
            .expect("Failed to retrieve namespace");

        assert_eq!(retrieved_namespace.id, "new_id");
        assert_eq!(retrieved_namespace.name, "new_name");
        assert_eq!(retrieved_namespace.description, "new_description");
    }

    #[tokio::test]
    async fn test_get_namespace() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let namespace = get(&mut conn, "some_id")
            .await
            .expect("Failed to get namespace");

        assert_eq!(namespace.id, "some_id");
        assert_eq!(namespace.name, "some_name");

        assert!(
            get(&mut conn, "non_existent").await.is_err(),
            "Unexpectedly found a namespace"
        );
    }

    #[tokio::test]
    async fn test_update_namespace() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            name: Some("updated_name".into()),
            description: Some("updated_description".into()),
            modified: "updated_time".into(),
        };

        update(&mut conn, "some_id", fields_to_update)
            .await
            .expect("Failed to update namespace");

        let updated_namespace = get(&mut conn, "some_id")
            .await
            .expect("Failed to retrieve updated namespace");

        assert_eq!(updated_namespace.name, "updated_name");
        assert_eq!(updated_namespace.description, "updated_description");
    }

    #[tokio::test]
    async fn test_delete_namespace() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id")
            .await
            .expect("Failed to delete namespace");

        assert!(
            get(&mut conn, "some_id").await.is_err(),
            "Namespace was not deleted"
        );
    }
}
