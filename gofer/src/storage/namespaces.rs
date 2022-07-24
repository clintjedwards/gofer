use crate::storage::{SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::namespace::Namespace;
use sqlx::{sqlite::SqliteRow, QueryBuilder, Row, Sqlite, SqliteConnection};
use std::ops::Deref;

#[derive(Debug, Default)]
pub struct UpdatableFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub modified: Option<u64>,
}

/// Return all namespaces; limited to 200 rows in any one response.
pub async fn list(
    conn: &mut SqliteConnection,
    offset: u64,
    limit: u64,
) -> Result<Vec<Namespace>, StorageError> {
    let mut limit = limit;

    if limit == 0 || limit > MAX_ROW_LIMIT {
        limit = MAX_ROW_LIMIT;
    }

    sqlx::query(
        r#"
SELECT id, name, description, created, modified
FROM namespaces
ORDER BY id
LIMIT ?
OFFSET ?;"#,
    )
    .bind(limit as i64)
    .bind(offset as i64)
    .map(|row: SqliteRow| Namespace {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        created: row.get::<i64, _>("created") as u64,
        modified: row.get::<i64, _>("modified") as u64,
    })
    .fetch_all(conn)
    .map_err(|e| StorageError::Unknown(e.to_string()))
    .await
}

/// Insert a new namespace.
pub async fn insert(
    conn: &mut SqliteConnection,
    namespace: &Namespace,
) -> Result<(), StorageError> {
    sqlx::query(
r#"INSERT INTO namespaces (id, name, description, created, modified) VALUES (?, ?, ?, ?, ?);"#)
    .bind(&namespace.id)
    .bind(&namespace.name)
    .bind(&namespace.description)
    .bind(namespace.created as i64)
    .bind(namespace.modified as i64)
    .execute(conn)
    .map_err(|e| match e {
        sqlx::Error::Database(database_err) => {
            if let Some(err_code) = database_err.code() {
                if err_code.deref() == SqliteErrors::Constraint.value() {
                    return StorageError::Exists;
                }
            }
            return StorageError::Unknown(database_err.message().to_string());
        }
        _ => StorageError::Unknown("".to_string()),
    })
    .await?;

    Ok(())
}

/// Get details on a specific namespace.
pub async fn get(conn: &mut SqliteConnection, id: &str) -> Result<Namespace, StorageError> {
    sqlx::query(
        r#"
SELECT id, name, description, created, modified
FROM namespaces
WHERE id = ?;"#,
    )
    .bind(id)
    .map(|row: SqliteRow| Namespace {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        created: row.get::<i64, _>("created") as u64,
        modified: row.get::<i64, _>("modified") as u64,
    })
    .fetch_one(conn)
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

/// Update a specific namespace.
pub async fn update(
    conn: &mut SqliteConnection,
    id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE namespaces SET "#);

    let mut updated_fields_total = 0;

    if let Some(name) = fields.name {
        update_query.push("name = ");
        update_query.push_bind(name);
        updated_fields_total += 1;
    }

    if let Some(description) = fields.description {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("description = ");
        update_query.push_bind(description);
        updated_fields_total += 1;
    }

    if let Some(modified) = fields.modified {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("modified = ");
        update_query.push_bind(modified as i64);
    }

    update_query.push(" WHERE id = ");
    update_query.push_bind(id);
    update_query.push(";");

    let update_query = update_query.build();

    update_query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
}

pub async fn delete(conn: &mut SqliteConnection, id: &str) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM namespaces
WHERE id = ?;"#,
    )
    .bind(id)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}
