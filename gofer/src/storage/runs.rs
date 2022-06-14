use std::ops::Deref;

use crate::models::{Run, RunState, RunStatus};
use crate::storage::{Db, SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use sqlx::{sqlite::SqliteRow, Acquire, Execute, QueryBuilder, Row, Sqlite};
use std::str::FromStr;

impl Db {
    /// Return all runs for a given namespace/pipeline; limited to 200 rows per response.
    pub async fn list_runs(
        &self,
        offset: u64,
        limit: u64,
        namespace: &str,
        pipeline: &str,
    ) -> Result<Vec<Run>, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut limit = limit;

        if limit == 0 || limit > MAX_ROW_LIMIT {
            limit = MAX_ROW_LIMIT;
        }

        // First we need to get the general run information.
        let runs = sqlx::query(
            r#"
        SELECT namespace, pipeline, id, started, ended, state, status, failure_info,
        task_runs, trigger, variables, store_info
        FROM runs
        WHERE namespace = ? AND pipeline = ?
        ORDER BY started DESC
        LIMIT ?
        OFFSET ?;
            "#,
        )
        .bind(namespace)
        .bind(pipeline)
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| Run {
            namespace: row.get("namespace"),
            pipeline: row.get("pipeline"),
            started: row.get::<i64, _>("started") as u64,
            ended: row.get::<i64, _>("ended") as u64,
            id: row.get::<i64, _>("id") as u64,
            state: RunState::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into run state enum".to_string(),
                })
                .unwrap(),
            status: RunStatus::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into run status enum".to_string(),
                })
                .unwrap(),
            failure_info: {
                let failure_info = row.get::<String, _>("failure_info");
                if failure_info.is_empty() {
                    None
                } else {
                    serde_json::from_str(&failure_info).unwrap()
                }
            },
            task_runs: {
                let task_run = row.get::<String, _>("task_runs");
                serde_json::from_str(&task_run).unwrap()
            },
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
                if store_info.is_empty() {
                    None
                } else {
                    serde_json::from_str(&store_info).unwrap()
                }
            },
        })
        .fetch_all(&mut conn)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

        Ok(runs)
    }

    /// Create a new run.
    pub async fn create_run(&self, run: &Run) -> Result<u64, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut tx = conn
            .begin()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        struct LastRun {
            id: u64,
        }

        let last_run = match sqlx::query(
            r#"
            SELECT id
            FROM runs
            WHERE namespace = ? AND pipeline = ?
            ORDER BY started DESC
            LIMIT 1;
                "#,
        )
        .bind(&run.namespace)
        .bind(&run.pipeline)
        .map(|row: SqliteRow| LastRun {
            id: row.get::<i64, _>("id") as u64,
        })
        .fetch_one(&mut tx)
        .await
        {
            Ok(last_run) => last_run,
            Err(storage_err) => match storage_err {
                sqlx::Error::RowNotFound => LastRun { id: 0 },
                _ => panic!("{}", storage_err.to_string()),
            },
        };

        let next_id = last_run.id + 1;

        sqlx::query(
            r#"
        INSERT INTO runs (namespace, pipeline, id, started, ended, state, status, failure_info,
            task_runs, trigger, variables, store_info)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(&run.namespace)
        .bind(&run.pipeline)
        .bind(next_id as i64)
        .bind(run.started as i64)
        .bind(run.ended as i64)
        .bind(run.state.to_string())
        .bind(run.status.to_string())
        .bind({
            if run.failure_info.is_none() {
                None
            } else {
                Some(serde_json::to_string(&run.failure_info).unwrap())
            }
        })
        .bind(serde_json::to_string(&run.task_runs).unwrap())
        .bind(serde_json::to_string(&run.trigger).unwrap())
        .bind(serde_json::to_string(&run.variables).unwrap())
        .bind({
            if run.store_info.is_none() {
                None
            } else {
                Some(serde_json::to_string(&run.store_info).unwrap())
            }
        })
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
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .unwrap();

        Ok(next_id)
    }

    /// Get details on a specific run.
    pub async fn get_run(
        &self,
        namespace: &str,
        pipeline: &str,
        id: u64,
    ) -> Result<Run, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let run = sqlx::query(
            r#"
        SELECT namespace, pipeline, id, started, ended, state, status, failure_info,
        task_runs, trigger, variables, store_info
        FROM runs
        WHERE namespace = ? AND pipeline = ? AND id = ?;
            "#,
        )
        .bind(namespace)
        .bind(pipeline)
        .bind(id as i64)
        .map(|row: SqliteRow| Run {
            namespace: row.get("namespace"),
            pipeline: row.get("pipeline"),
            started: row.get::<i64, _>("started") as u64,
            ended: row.get::<i64, _>("ended") as u64,
            id: row.get::<i64, _>("id") as u64,
            state: RunState::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into run state enum".to_string(),
                })
                .unwrap(),
            status: RunStatus::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into run status enum".to_string(),
                })
                .unwrap(),
            failure_info: {
                let failure_info = row.get::<String, _>("failure_info");
                if failure_info.is_empty() {
                    None
                } else {
                    serde_json::from_str(&failure_info).unwrap()
                }
            },
            task_runs: {
                let task_run_json = row.get::<String, _>("task_runs");
                serde_json::from_str(&task_run_json).unwrap()
            },
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
                if store_info.is_empty() {
                    None
                } else {
                    serde_json::from_str(&store_info).unwrap()
                }
            },
        })
        .fetch_one(&mut conn)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(run)
    }

    /// Get details on several runs.
    pub async fn batch_get_runs(
        &self,
        namespace: &str,
        pipeline: &str,
        ids: &Vec<u64>,
    ) -> Result<Vec<Run>, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut run_query: QueryBuilder<Sqlite> = QueryBuilder::new(
            r#"SELECT namespace, pipeline, id, started, ended, state, status, failure_info,
            task_runs, trigger, variables, store_info
            FROM runs
            WHERE "#,
        );

        run_query.push("namespace = ");
        run_query.push_bind(namespace);
        run_query.push(" AND pipeline = ");
        run_query.push_bind(pipeline);
        run_query.push(" AND id IN (");

        for (index, id) in ids.iter().enumerate() {
            run_query.push_bind(*id as i64);

            if index + 1 != ids.len() {
                run_query.push(", ");
            }
        }

        run_query.push(");");
        let run_query = run_query.build();

        let runs = run_query
            .map(|row: SqliteRow| Run {
                namespace: row.get("namespace"),
                pipeline: row.get("pipeline"),
                started: row.get::<i64, _>("started") as u64,
                ended: row.get::<i64, _>("ended") as u64,
                id: row.get::<i64, _>("id") as u64,
                state: RunState::from_str(row.get("state"))
                    .map_err(|_| StorageError::Parse {
                        value: row.get("state"),
                        column: "state".to_string(),
                        err: "could not parse value into run state enum".to_string(),
                    })
                    .unwrap(),
                status: RunStatus::from_str(row.get("status"))
                    .map_err(|_| StorageError::Parse {
                        value: row.get("status"),
                        column: "status".to_string(),
                        err: "could not parse value into run status enum".to_string(),
                    })
                    .unwrap(),
                failure_info: {
                    let failure_info = row.get::<String, _>("failure_info");
                    if failure_info.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&failure_info).unwrap()
                    }
                },
                task_runs: {
                    let task_run_json = row.get::<String, _>("task_runs");
                    serde_json::from_str(&task_run_json).unwrap()
                },
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
                    if store_info.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&store_info).unwrap()
                    }
                },
            })
            .fetch_all(&mut conn)
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => StorageError::NotFound,
                _ => StorageError::Unknown(e.to_string()),
            })
            .await?;

        Ok(runs)
    }

    pub async fn update_run_state(
        &self,
        namespace: &str,
        pipeline: &str,
        id: u64,
        state: RunState,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE runs
        SET state = ?
        WHERE namespace = ? AND pipeline = ? AND id = ?;
            "#,
        )
        .bind(state.to_string())
        .bind(namespace)
        .bind(pipeline)
        .bind(id as i64)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(())
    }

    pub async fn update_run_status(
        &self,
        namespace: &str,
        pipeline: &str,
        id: u64,
        status: RunStatus,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE runs
        SET status = ?
        WHERE namespace = ? AND pipeline = ? AND id = ?;
            "#,
        )
        .bind(status.to_string())
        .bind(namespace)
        .bind(pipeline)
        .bind(id as i64)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(())
    }

    /// Update a specific run.
    pub async fn update_run(&self, run: &Run) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE runs
        SET ended = ?, state = ?, status = ?, failure_info = ?, task_runs = ?, trigger = ?, variables = ?,
        store_info = ?
        WHERE namespace = ? AND pipeline = ? AND id = ?;
            "#,
        )
        .bind(run.ended as i64)
        .bind(run.state.to_string())
        .bind(run.status.to_string())
        .bind({
            if run.failure_info.is_none() {
                None
            } else {
                Some(serde_json::to_string(&run.failure_info).unwrap())
            }
        })
        .bind(serde_json::to_string(&run.task_runs).unwrap())
        .bind(serde_json::to_string(&run.trigger).unwrap())
        .bind(serde_json::to_string(&run.variables).unwrap())
        .bind({
            if run.store_info.is_none() {
                None
            } else {
                Some(serde_json::to_string(&run.store_info).unwrap())
            }
        })
        .bind(&run.namespace)
        .bind(&run.pipeline)
        .bind(run.id as i64)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(())
    }

    pub async fn delete_run(
        &self,
        namespace: &str,
        pipeline: &str,
        id: u64,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        DELETE FROM runs
        WHERE namespace = ? AND pipeline = ? AND id = ?;
            "#,
        )
        .bind(namespace)
        .bind(pipeline)
        .bind(id as i64)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await
    }
}
