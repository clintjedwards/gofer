use std::{ops::Deref, str::FromStr};

use crate::storage::{Db, SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::Event;
use sqlx::{sqlite::SqliteRow, Row};

impl Db {
    /// Return all events; limited to 200 rows in any one response.
    pub async fn list_events(&self, offset: u64, limit: u64) -> Result<Vec<Event>, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut limit = limit;

        if limit == 0 || limit > MAX_ROW_LIMIT {
            limit = MAX_ROW_LIMIT;
        }

        let result = sqlx::query(
            r#"
        SELECT id, kind, emitted, metadata
        FROM events
        ORDER BY id DESC
        LIMIT ?
        OFFSET ?;
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| Event {
            id: row.get::<i64, _>("id") as u64,
            kind: gofer_models::EventKind::from_str(row.get("kind"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("kind"),
                    column: "kind".to_string(),
                    err: "could not parse value into event kind enum".to_string(),
                })
                .unwrap(),
            emitted: row.get::<i64, _>("emitted") as u64,
            metadata: row.get("metadata"),
        })
        .fetch_all(&mut conn)
        .await;

        result.map_err(|e| StorageError::Unknown(e.to_string()))
    }

    /// Create a new event.
    pub async fn create_event(&self, event: &Event) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        INSERT INTO events (id, kind, emitted, metadata)
        VALUES (?, ?, ?, ?);
            "#,
        )
        .bind(event.id as i64)
        .bind(serde_json::to_string(&event.kind).unwrap())
        .bind(event.emitted as i64)
        .bind(&event.metadata)
        .execute(&mut conn)
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

    /// Get details on a specific event.
    pub async fn get_event(&self, id: &str) -> Result<Event, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        SELECT id, kind, emitted, metadata
        FROM events
        WHERE id = ?;
            "#,
        )
        .bind(id)
        .map(|row: SqliteRow| Event {
            id: row.get::<i64, _>("id") as u64,
            kind: gofer_models::EventKind::from_str(row.get("kind"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("kind"),
                    column: "kind".to_string(),
                    err: "could not parse value into event kind enum".to_string(),
                })
                .unwrap(),
            emitted: row.get::<i64, _>("emitted") as u64,
            metadata: row.get("metadata"),
        })
        .fetch_one(&mut conn)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }

    pub async fn delete_event(&self, id: &str) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        DELETE FROM events
        WHERE id = ?;
            "#,
        )
        .bind(id)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }
}
