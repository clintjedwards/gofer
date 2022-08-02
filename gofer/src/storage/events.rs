use crate::storage::{SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::event::Event;
use sqlx::{sqlite::SqliteRow, Row, SqliteConnection};
use std::ops::Deref;

/// Return all events; limited to 200 rows in any one response.
/// The reverse parameter allows the sorting the events in reverse chronological order (newest event first).
pub async fn list(
    conn: &mut SqliteConnection,
    offset: u64,
    limit: u64,
    reverse: bool,
) -> Result<Vec<Event>, StorageError> {
    let mut limit = limit;

    if limit == 0 || limit > MAX_ROW_LIMIT {
        limit = MAX_ROW_LIMIT;
    }

    let query_str = r#"SELECT id, kind, emitted
FROM events
ORDER BY id ASC
LIMIT ?
OFFSET ?;"#;

    let query_str = if reverse {
        query_str.replacen("ASC", "DESC", 1)
    } else {
        query_str.to_string()
    };

    sqlx::query(&query_str)
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| Event {
            id: row.get::<i64, _>("id") as u64,
            kind: {
                let kind = row.get::<String, _>("kind");
                serde_json::from_str(&kind).unwrap()
            },
            emitted: row.get::<i64, _>("emitted") as u64,
        })
        .fetch_all(conn)
        .await
        .map_err(|e| StorageError::Unknown(e.to_string()))
}

/// Insert a new event.
pub async fn insert(conn: &mut SqliteConnection, event: &Event) -> Result<u64, StorageError> {
    let result = sqlx::query(
        r#"
INSERT INTO events (kind, emitted)
VALUES (?, ?);"#,
    )
    .bind(serde_json::to_string(&event.kind).unwrap())
    .bind(event.emitted as i64)
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

    Ok(result.last_insert_rowid() as u64)
}

/// Get details on a specific event.
pub async fn get(conn: &mut SqliteConnection, id: u64) -> Result<Event, StorageError> {
    sqlx::query(
        r#"
SELECT id, kind, emitted
FROM events
WHERE id = ?;"#,
    )
    .bind(id as i64)
    .map(|row: SqliteRow| Event {
        id: row.get::<i64, _>("id") as u64,
        kind: {
            let kind = row.get::<String, _>("kind");
            serde_json::from_str(&kind).unwrap()
        },
        emitted: row.get::<i64, _>("emitted") as u64,
    })
    .fetch_one(conn)
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

pub async fn delete(conn: &mut SqliteConnection, id: u64) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM events
WHERE id = ?;"#,
    )
    .bind(id as i64)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}
