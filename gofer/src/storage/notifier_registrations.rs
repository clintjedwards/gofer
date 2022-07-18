use crate::storage::{Db, SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::notifier::{Registration, Status};
use sqlx::{sqlite::SqliteRow, Row};
use std::ops::Deref;
use std::str::FromStr;

impl Db {
    /// Return all notifiers; limited to 200 rows in any one response.
    pub async fn list_notifier_registrations(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Registration>, StorageError> {
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
        FROM notifier_registrations
        LIMIT ?
        OFFSET ?;
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| Registration {
            name: row.get("name"),
            image: row.get("image"),
            user: row.get("user"),
            pass: row.get("pass"),
            variables: {
                let variables_json = row.get::<String, _>("variables");
                serde_json::from_str(&variables_json).unwrap()
            },
            created: row.get::<i64, _>("created") as u64,
            status: Status::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into notifier status enum".to_string(),
                })
                .unwrap(),
        })
        .fetch_all(&mut conn)
        .await;

        result.map_err(|e| StorageError::Unknown(e.to_string()))
    }

    /// Create a new notifier registration.
    pub async fn create_notifier_registration(
        &self,
        notifier_registration: &Registration,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        INSERT INTO notifier_registrations (name, image, user, pass, variables, created, status)
        VALUES (?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(&notifier_registration.name)
        .bind(&notifier_registration.image)
        .bind(&notifier_registration.user)
        .bind(&notifier_registration.pass)
        .bind(serde_json::to_string(&notifier_registration.variables).unwrap())
        .bind(notifier_registration.created as i64)
        .bind(&notifier_registration.status.to_string())
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

    /// Get details on a specific notifier_registration.
    pub async fn get_notifier_registration(
        &self,
        name: &str,
    ) -> Result<Registration, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        SELECT name, image, user, pass, variables, created, status
        FROM notifier_registrations
        WHERE name = ?;
            "#,
        )
        .bind(name)
        .map(|row: SqliteRow| Registration {
            name: row.get("name"),
            image: row.get("image"),
            user: row.get("user"),
            pass: row.get("pass"),
            variables: {
                let variables_json = row.get::<String, _>("variables");
                serde_json::from_str(&variables_json).unwrap()
            },
            created: row.get::<i64, _>("created") as u64,
            status: Status::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into notifier status enum".to_string(),
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

    /// Update a specific notifier_registration.
    pub async fn update_notifier_registration(
        &self,
        notifier_registration: &Registration,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE notifier_registrations
        SET image = ?, user = ?, pass = ?, variables = ?, status = ?
        WHERE name = ?;
            "#,
        )
        .bind(&notifier_registration.image)
        .bind(&notifier_registration.user)
        .bind(&notifier_registration.pass)
        .bind(serde_json::to_string(&notifier_registration.variables).unwrap())
        .bind(&notifier_registration.status.to_string())
        .bind(&notifier_registration.name)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }

    pub async fn delete_notifier_registration(&self, name: &str) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        DELETE FROM notifier_registrations
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
