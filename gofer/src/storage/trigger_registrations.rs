use std::{ops::Deref, str::FromStr};

use crate::storage::{SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::trigger::{Registration, Status};
use sqlx::{sqlite::SqliteRow, QueryBuilder, Row, Sqlite, SqliteConnection};

#[derive(Debug, Default)]
pub struct UpdatableFields {
    pub image: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub variables: Option<String>,
    pub status: Option<Status>,
}

/// Return all triggers; limited to 200 rows in any one response.
pub async fn list(
    conn: &mut SqliteConnection,
    offset: u64,
    limit: u64,
) -> Result<Vec<Registration>, StorageError> {
    let mut limit = limit;

    if limit == 0 || limit > MAX_ROW_LIMIT {
        limit = MAX_ROW_LIMIT;
    }

    sqlx::query(
        r#"
SELECT name, image, user, pass, variables, created, status
FROM trigger_registrations
LIMIT ?
OFFSET ?;"#,
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
                err: "could not parse value into trigger status enum".to_string(),
            })
            .unwrap(),
    })
    .fetch_all(conn)
    .await
    .map_err(|e| StorageError::Unknown(e.to_string()))
}

/// Insert a new trigger registration.
pub async fn insert(
    conn: &mut SqliteConnection,
    trigger_registration: &Registration,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
INSERT INTO trigger_registrations (name, image, user, pass, variables, created, status)
VALUES (?, ?, ?, ?, ?, ?, ?);"#,
    )
    .bind(&trigger_registration.name)
    .bind(&trigger_registration.image)
    .bind(&trigger_registration.user)
    .bind(&trigger_registration.pass)
    .bind(serde_json::to_string(&trigger_registration.variables).unwrap())
    .bind(trigger_registration.created as i64)
    .bind(&trigger_registration.status.to_string())
    .execute(conn)
    .map_ok(|_| ())
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
    .await
}

/// Get details on a specific trigger_registration.
pub async fn get(conn: &mut SqliteConnection, name: &str) -> Result<Registration, StorageError> {
    sqlx::query(
        r#"
SELECT name, image, user, pass, variables, created, status
FROM trigger_registrations
WHERE name = ?;"#,
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
                err: "could not parse value into trigger status enum".to_string(),
            })
            .unwrap(),
    })
    .fetch_one(conn)
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

/// Update a specific trigger_registration.
pub async fn update(
    conn: &mut SqliteConnection,
    name: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE trigger_registrations SET "#);

    let mut updated_fields_total = 0;

    if let Some(image) = fields.image {
        update_query.push("image = ");
        update_query.push_bind(image);
        updated_fields_total += 1;
    }

    if let Some(user) = fields.user {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("user = ");
        update_query.push_bind(user);
        updated_fields_total += 1;
    }

    if let Some(pass) = fields.pass {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("pass = ");
        update_query.push_bind(pass);
        updated_fields_total += 1;
    }

    if let Some(variables) = fields.variables {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("variables = ");
        update_query.push_bind(variables);
        updated_fields_total += 1;
    }

    if let Some(status) = fields.status {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status = ");
        update_query.push_bind(status.to_string());
    }

    update_query.push(" WHERE name = ");
    update_query.push_bind(name);
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

pub async fn delete(conn: &mut SqliteConnection, name: &str) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM trigger_registrations
WHERE name = ?;"#,
    )
    .bind(name)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}
