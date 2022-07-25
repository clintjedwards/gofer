use std::ops::{Deref, Not};

use crate::storage::{SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::task_run::{State, Status, StatusReason, TaskRun};
use gofer_models::Variable;
use sqlx::{sqlite::SqliteRow, QueryBuilder, Row, Sqlite, SqliteConnection};
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct UpdatableFields {
    pub started: Option<u64>,
    pub ended: Option<u64>,
    pub exit_code: Option<u8>,
    pub failure: Option<StatusReason>,
    pub logs_expired: Option<bool>,
    pub logs_removed: Option<bool>,
    pub state: Option<State>,
    pub status: Option<Status>,
    pub scheduler_id: Option<String>,
    pub variables: Option<Vec<Variable>>,
}

/// Return all task_run for a given namespace/pipeline/run; limited to 200 rows per response.
pub async fn list(
    conn: &mut SqliteConnection,
    offset: u64,
    limit: u64,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
) -> Result<Vec<TaskRun>, StorageError> {
    let mut limit = limit;

    if limit == 0 || limit > MAX_ROW_LIMIT {
        limit = MAX_ROW_LIMIT;
    }

    // First we need to get the general task_run information.
    let task_runs = sqlx::query(
            r#"
SELECT namespace, pipeline, run, id, task, created, started, ended, exit_code, failure,
logs_expired, logs_removed, state, status, scheduler_id, variables
FROM task_runs
WHERE namespace = ? AND pipeline = ? AND run = ?
LIMIT ?
OFFSET ?;"#,
        )
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(run_id as i64)
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| TaskRun {
            namespace: row.get("namespace"),
            pipeline: row.get("pipeline"),
            run: row.get::<i64, _>("run") as u64,
            id: row.get("id"),
            task: {
                let task_json = row.get::<String, _>("task");
                serde_json::from_str(&task_json).unwrap()
            },
            created: row.get::<i64, _>("created") as u64,
            started: row.get::<i64, _>("started") as u64,
            ended: row.get::<i64, _>("ended") as u64,
            exit_code: row.get("exit_code"),
            status_reason: {
                let failure = row.get::<String, _>("failure");
                failure.is_empty().not().then(|| serde_json::from_str(&failure).unwrap())
            },
            logs_expired: {
                let logs_expired = match row.get::<i64, _>("logs_expired") {
                    0 => false,
                    1 => true,
                    _ => panic!("could not parse value into task_run logs_expired; only allowed values are 0 and 1")
                };
                logs_expired
            },
            logs_removed: {
                let logs_removed = match row.get::<i64, _>("logs_expired") {
                    0 => false,
                    1 => true,
                    _ => panic!("could not parse value into task_run logs_expired; only allowed values are 0 and 1")
                };
                logs_removed
            },
            state: State::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into task_run state enum".to_string(),
                })
                .unwrap(),
            status: Status::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into task_run status enum".to_string(),
                })
                .unwrap(),
            scheduler_id: row.get("scheduler_id"),
            variables: {
                let variables_json = row.get::<String, _>("variables");
                serde_json::from_str(&variables_json).unwrap()
            },
        })
        .fetch_all(conn)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

    Ok(task_runs)
}

/// Insert a new task_run.
pub async fn insert(conn: &mut SqliteConnection, task_run: &TaskRun) -> Result<(), StorageError> {
    sqlx::query(
        r#"
INSERT INTO task_runs (namespace, pipeline, run, id, task, created, started, ended,
    exit_code, failure, logs_expired, logs_removed, state, status, scheduler_id, variables)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"#,
    )
    .bind(&task_run.namespace)
    .bind(&task_run.pipeline)
    .bind(task_run.run as i64)
    .bind(&task_run.id)
    .bind(serde_json::to_string(&task_run.task).unwrap())
    .bind(task_run.created as i64)
    .bind(task_run.started as i64)
    .bind(task_run.ended as i64)
    .bind(task_run.exit_code)
    .bind(
        task_run
            .status_reason
            .is_none()
            .not()
            .then(|| serde_json::to_string(&task_run.status_reason).unwrap()),
    )
    .bind({
        let task_run_bool: i32 = match task_run.logs_expired {
            false => 0,
            true => 1,
        };

        task_run_bool
    })
    .bind({
        let task_run_bool: i32 = match task_run.logs_removed {
            false => 0,
            true => 1,
        };

        task_run_bool
    })
    .bind(task_run.state.to_string())
    .bind(task_run.status.to_string())
    .bind(&task_run.scheduler_id)
    .bind(serde_json::to_string(&task_run.variables).unwrap())
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

/// Get details on a specific task_run.
pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    id: &str,
) -> Result<TaskRun, StorageError> {
    let task_run = sqlx::query(
            r#"
SELECT namespace, pipeline, run, id, task, created, started, ended, exit_code, failure,
logs_expired, logs_removed, state, status, scheduler_id, variables
FROM task_runs
WHERE namespace = ? AND pipeline = ? AND run = ? AND id = ?;"#,
        )
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(run_id as i64)
        .bind(id)
        .map(|row: SqliteRow| TaskRun {
            namespace: row.get("namespace"),
            pipeline: row.get("pipeline"),
            run: row.get::<i64, _>("run") as u64,
            id: row.get("id"),
            task: {
                let task_json = row.get::<String, _>("task");
                serde_json::from_str(&task_json).unwrap()
            },
            created: row.get::<i64, _>("created") as u64,
            started: row.get::<i64, _>("started") as u64,
            ended: row.get::<i64, _>("ended") as u64,
            exit_code: row.get("exit_code"),
            status_reason: {
                let failure = row.get::<String, _>("failure");
                failure.is_empty().not().then(||serde_json::from_str(&failure).unwrap())
            },
            logs_expired: {
                let logs_expired = match row.get::<i64, _>("logs_expired") {
                    0 => false,
                    1 => true,
                    _ => panic!("could not parse value into task_run logs_expired; only allowed values are 0 and 1")
                };
                logs_expired
            },
            logs_removed: {
                let logs_removed = match row.get::<i64, _>("logs_expired") {
                    0 => false,
                    1 => true,
                    _ => panic!("could not parse value into task_run logs_expired; only allowed values are 0 and 1")
                };
                logs_removed
            },
            state: State::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into task_run state enum".to_string(),
                })
                .unwrap(),
            status: Status::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into task_run status enum".to_string(),
                })
                .unwrap(),
            scheduler_id: row.get("scheduler_id"),
            variables: {
                let variables_json = row.get::<String, _>("variables");
                serde_json::from_str(&variables_json).unwrap()
            },
        })
        .fetch_one(conn)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

    Ok(task_run)
}

/// Update a specific task_run.
pub async fn update(
    conn: &mut SqliteConnection,
    task_run: &TaskRun,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE task_runs SET "#);

    let mut updated_fields_total = 0;

    if let Some(started) = fields.started {
        update_query.push("started = ");
        update_query.push_bind(started as i64);
        updated_fields_total += 1;
    }

    if let Some(ended) = fields.ended {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("ended = ");
        update_query.push_bind(ended as i64);
        updated_fields_total += 1;
    }

    if let Some(exit_code) = fields.exit_code {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("exit_code = ");
        update_query.push_bind(exit_code as i64);
        updated_fields_total += 1;
    }

    if let Some(failure) = fields.failure {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("failure = ");
        update_query.push_bind(serde_json::to_string(&failure).unwrap());
        updated_fields_total += 1;
    }

    if let Some(logs_expired) = fields.logs_expired {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("logs_expired = ");
        update_query.push_bind::<i64>({
            match logs_expired {
                false => 0,
                true => 1,
            }
        });
        updated_fields_total += 1;
    }

    if let Some(logs_removed) = fields.logs_removed {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("logs_removed = ");
        update_query.push_bind::<i64>({
            match logs_removed {
                false => 0,
                true => 1,
            }
        });
        updated_fields_total += 1;
    }

    if let Some(state) = fields.state {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("state = ");
        update_query.push_bind(state.to_string());
        updated_fields_total += 1;
    }

    if let Some(status) = fields.status {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status = ");
        update_query.push_bind(status.to_string());
        updated_fields_total += 1;
    }

    if let Some(scheduler_id) = fields.scheduler_id {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("scheduler_id = ");
        update_query.push_bind(scheduler_id);
        updated_fields_total += 1;
    }

    if let Some(variables) = fields.variables {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("variables = ");
        update_query.push_bind(serde_json::to_string(&variables).unwrap());
    }

    update_query.push(" WHERE namespace = ");
    update_query.push_bind(&task_run.namespace);

    update_query.push(" AND pipeline = ");
    update_query.push_bind(&task_run.pipeline);

    update_query.push(" AND run = ");
    update_query.push_bind(task_run.run as i64);

    update_query.push(" AND id = ");
    update_query.push_bind(&task_run.id);
    update_query.push(";");

    let update_query = update_query.build();

    update_query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

    Ok(())
}

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
    id: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM task_runs
WHERE namespace = ? AND pipeline = ? AND run = ? AND id = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(run_id as i64)
    .bind(id)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}
