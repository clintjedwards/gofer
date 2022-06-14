use std::{ops::Deref, str::FromStr};

use crate::storage::{Db, SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::TriggerRegistration;
use sqlx::{sqlite::SqliteRow, Row};

impl Db {
    /// Return all triggers; limited to 200 rows in any one response.
    pub async fn list_trigger_registrations(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TriggerRegistration>, StorageError> {
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
        SELECT name, image, user, pass, variables, created, status
        FROM trigger_registrations
        LIMIT ?
        OFFSET ?;
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| TriggerRegistration {
            name: row.get("name"),
            image: row.get("image"),
            user: row.get("user"),
            pass: row.get("pass"),
            variables: {
                let variables_json = row.get::<String, _>("variables");
                serde_json::from_str(&variables_json).unwrap()
            },
            created: row.get::<i64, _>("created") as u64,
            status: gofer_models::TriggerStatus::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into trigger status enum".to_string(),
                })
                .unwrap(),
        })
        .fetch_all(&mut conn)
        .await;

        result.map_err(|e| StorageError::Unknown(e.to_string()))
    }

    /// Create a new trigger registration.
    pub async fn create_trigger_registration(
        &self,
        trigger_registration: &TriggerRegistration,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        INSERT INTO trigger_registrations (name, image, user, pass, variables, created, status)
        VALUES (?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(&trigger_registration.name)
        .bind(&trigger_registration.image)
        .bind(&trigger_registration.user)
        .bind(&trigger_registration.pass)
        .bind(serde_json::to_string(&trigger_registration.variables).unwrap())
        .bind(trigger_registration.created as i64)
        .bind(&trigger_registration.status.to_string())
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

    /// Get details on a specific trigger_registration.
    pub async fn get_trigger_registration(
        &self,
        name: &str,
    ) -> Result<TriggerRegistration, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        SELECT name, image, user, pass, variables, created, status
        FROM trigger_registrations
        WHERE name = ?;
            "#,
        )
        .bind(name)
        .map(|row: SqliteRow| TriggerRegistration {
            name: row.get("name"),
            image: row.get("image"),
            user: row.get("user"),
            pass: row.get("pass"),
            variables: {
                let variables_json = row.get::<String, _>("variables");
                serde_json::from_str(&variables_json).unwrap()
            },
            created: row.get::<i64, _>("created") as u64,
            status: gofer_models::TriggerStatus::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into trigger status enum".to_string(),
                })
                .unwrap(),
        })
        .fetch_one(&mut conn)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }

    /// Update a specific trigger_registration.
    pub async fn update_trigger_registration(
        &self,
        trigger_registration: &TriggerRegistration,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE trigger_registrations
        SET image = ?, user = ?, pass = ?, variables = ?, status = ?
        WHERE name = ?;
            "#,
        )
        .bind(&trigger_registration.image)
        .bind(&trigger_registration.user)
        .bind(&trigger_registration.pass)
        .bind(serde_json::to_string(&trigger_registration.variables).unwrap())
        .bind(&trigger_registration.status.to_string())
        .bind(&trigger_registration.name)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }

    pub async fn delete_trigger_registration(&self, name: &str) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        DELETE FROM trigger_registrations
        WHERE name = ?;
            "#,
        )
        .bind(name)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }
}
