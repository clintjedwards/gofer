use crate::storage::{SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::run::{Run, State, Status, StatusReason, StoreInfo};
use gofer_models::Variable;
use sqlx::{sqlite::SqliteRow, Acquire, QueryBuilder, Row, Sqlite, SqliteConnection};
use std::ops::{Deref, Not};
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct UpdatableFields {
    pub ended: Option<u64>,
    pub state: Option<State>,
    pub status: Option<Status>,
    pub failure_info: Option<StatusReason>,
    pub variables: Option<Vec<Variable>>,
    pub store_info: Option<StoreInfo>,
}

/// Return all runs for a given namespace/pipeline; limited to 200 rows per response.
/// Returns runs by id(which is sequential) in descending order.
pub async fn list(
    conn: &mut SqliteConnection,
    offset: u64,
    limit: u64,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<Run>, StorageError> {
    let mut limit = limit;

    if limit == 0 || limit > MAX_ROW_LIMIT {
        limit = MAX_ROW_LIMIT;
    }

    let runs = sqlx::query(
        r#"
SELECT namespace, pipeline, id, started, ended, state, status, failure_info, trigger, variables, store_info
FROM runs
WHERE namespace = ? AND pipeline = ?
ORDER BY started DESC
LIMIT ?
OFFSET ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .map(|row: SqliteRow| Run {
        namespace: row.get("namespace"),
        pipeline: row.get("pipeline"),
        started: row.get::<i64, _>("started") as u64,
        ended: row.get::<i64, _>("ended") as u64,
        id: row.get::<i64, _>("id") as u64,
        state: State::from_str(row.get("state"))
            .map_err(|_| StorageError::Parse {
                value: row.get("state"),
                column: "state".to_string(),
                err: "could not parse value into run state enum".to_string(),
            })
            .unwrap(),
        status: Status::from_str(row.get("status"))
            .map_err(|_| StorageError::Parse {
                value: row.get("status"),
                column: "status".to_string(),
                err: "could not parse value into run status enum".to_string(),
            })
            .unwrap(),
        status_reason: {
            let failure_info = row.get::<String, _>("failure_info");
            failure_info
                .is_empty()
                .not()
                .then(|| serde_json::from_str(&failure_info).unwrap())
        },
        task_runs: vec![],
        trigger: {
            let trigger_info_json = row.get::<String, _>("trigger");
            serde_json::from_str(&trigger_info_json).unwrap()
        },
        variables: {
            let variables_json = row.get::<String, _>("variables");
            serde_json::from_str(&variables_json).unwrap()
        },
        store_info: {
            let store_info = row.get::<String, _>("store_info");
            store_info
                .is_empty()
                .not()
                .then(|| serde_json::from_str(&store_info).unwrap())
        },
    })
    .fetch_all(conn)
    .map_err(|e| StorageError::Unknown(e.to_string()))
    .await?;

    Ok(runs)
}

/// Insert a new run.
pub async fn insert(conn: &mut SqliteConnection, run: &Run) -> Result<u64, StorageError> {
    let mut tx = conn
        .begin()
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

    let last_run = list(&mut tx, 0, 1, &run.namespace, &run.pipeline).await?;

    let mut next_id = 1;

    if !last_run.is_empty() {
        next_id = last_run[0].id + 1;
    }

    sqlx::query(
        r#"
INSERT INTO runs (namespace, pipeline, id, started, ended, state, status, failure_info,
    trigger, variables, store_info)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"#,
    )
    .bind(&run.namespace)
    .bind(&run.pipeline)
    .bind(next_id as i64)
    .bind(run.started as i64)
    .bind(run.ended as i64)
    .bind(run.state.to_string())
    .bind(run.status.to_string())
    .bind(
        run.status_reason
            .is_none()
            .not()
            .then(|| serde_json::to_string(&run.status_reason).unwrap()),
    )
    .bind(serde_json::to_string(&run.trigger).unwrap())
    .bind(serde_json::to_string(&run.variables).unwrap())
    .bind(
        run.store_info
            .is_none()
            .not()
            .then(|| serde_json::to_string(&run.store_info).unwrap()),
    )
    .execute(&mut tx)
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

    tx.commit()
        .await
        .map_err(|e| StorageError::Unknown(e.to_string()))?;

    Ok(next_id)
}

/// Get details on a specific run.
pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    id: u64,
) -> Result<Run, StorageError> {
    sqlx::query(
        r#"
SELECT namespace, pipeline, id, started, ended, state, status, failure_info, trigger, variables, store_info
FROM runs
WHERE namespace = ? AND pipeline = ? AND id = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(id as i64)
    .map(|row: SqliteRow| Run {
        namespace: row.get("namespace"),
        pipeline: row.get("pipeline"),
        started: row.get::<i64, _>("started") as u64,
        ended: row.get::<i64, _>("ended") as u64,
        id: row.get::<i64, _>("id") as u64,
        state: State::from_str(row.get("state"))
            .map_err(|_| StorageError::Parse {
                value: row.get("state"),
                column: "state".to_string(),
                err: "could not parse value into run state enum".to_string(),
            })
            .unwrap(),
        status: Status::from_str(row.get("status"))
            .map_err(|_| StorageError::Parse {
                value: row.get("status"),
                column: "status".to_string(),
                err: "could not parse value into run status enum".to_string(),
            })
            .unwrap(),
        status_reason: {
            let failure_info = row.get::<String, _>("failure_info");
            failure_info
                .is_empty()
                .not()
                .then(|| serde_json::from_str(&failure_info).unwrap())
        },
        task_runs: vec![],
        trigger: {
            let trigger_info_json = row.get::<String, _>("trigger");
            serde_json::from_str(&trigger_info_json).unwrap()
        },
        variables: {
            let variables_json = row.get::<String, _>("variables");
            serde_json::from_str(&variables_json).unwrap()
        },
        store_info: {
            let store_info = row.get::<String, _>("store_info");
            store_info
                .is_empty()
                .not()
                .then(|| serde_json::from_str(&store_info).unwrap())
        },
    })
    .fetch_one(conn)
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

/// Update a specific run.
pub async fn update(
    conn: &mut SqliteConnection,
    run: &Run,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE runs SET "#);

    let mut updated_fields_total = 0;

    if let Some(ended) = fields.ended {
        update_query.push("ended = ");
        update_query.push_bind(ended as i64);
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

    if let Some(failure_info) = fields.failure_info {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("failure_info = ");
        update_query.push_bind(serde_json::to_string(&failure_info).unwrap());
        updated_fields_total += 1;
    }

    if let Some(variables) = fields.variables {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("variables = ");
        update_query.push_bind(serde_json::to_string(&variables).unwrap());
        updated_fields_total += 1;
    }

    if let Some(store_info) = fields.store_info {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("store_info = ");
        update_query.push_bind(serde_json::to_string(&store_info).unwrap());
    }

    update_query.push(" WHERE namespace = ");
    update_query.push_bind(&run.namespace);

    update_query.push(" AND pipeline = ");
    update_query.push_bind(&run.pipeline);

    update_query.push(" AND id = ");
    update_query.push_bind(run.id as i64);
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

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    id: u64,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM runs
WHERE namespace = ? AND pipeline = ? AND id = ?;
        "#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(id as i64)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}
