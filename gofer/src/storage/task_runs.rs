use std::ops::Deref;

use crate::models::{TaskRun, TaskRunState, TaskRunStatus};
use crate::storage::{Db, SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use sqlx::{sqlite::SqliteRow, Acquire, Row};
use std::str::FromStr;

impl Db {
    /// Return all task_run for a given namespace/pipeline/run; limited to 200 rows per response.
    pub async fn list_task_runs(
        &self,
        offset: u64,
        limit: u64,
        namespace: &str,
        pipeline: &str,
        run: u64,
    ) -> Result<Vec<TaskRun>, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut limit = limit;

        if limit == 0 || limit > MAX_ROW_LIMIT {
            limit = MAX_ROW_LIMIT;
        }

        // First we need to get the general task_run information.
        let task_runs = sqlx::query(
            r#"
        SELECT namespace, pipeline, run, id, task, created, started, ended, exit_code, failure,
        logs_expired, logs_removed, state, status, scheduler_id
        FROM task_runs
        WHERE namespace = ? AND pipeline = ? AND run = ?
        LIMIT ?
        OFFSET ?;
            "#,
        )
        .bind(namespace)
        .bind(pipeline)
        .bind(run as i64)
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
            failure: {
                let failure = row.get::<String, _>("failure");
                if failure.is_empty() {
                    None
                } else {
                    serde_json::from_str(&failure).unwrap()
                }
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
            state: TaskRunState::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into task_run state enum".to_string(),
                })
                .unwrap(),
            status: TaskRunStatus::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into task_run status enum".to_string(),
                })
                .unwrap(),
            scheduler_id: row.get("scheduler_id"),
        })
        .fetch_all(&mut conn)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

        Ok(task_runs)
    }

    /// Create a new task_run.
    pub async fn create_task_run(&self, task_run: &TaskRun) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut tx = conn
            .begin()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        INSERT INTO task_runs (namespace, pipeline, run, id, task, created, started, ended,
            exit_code, failure, logs_expired, logs_removed, state, status, scheduler_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
            "#,
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
        .bind({
            if task_run.failure.is_none() {
                None
            } else {
                Some(serde_json::to_string(&task_run.failure).unwrap())
            }
        })
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

        Ok(())
    }

    /// Get details on a specific task_run.
    pub async fn get_task_run(
        &self,
        namespace: &str,
        pipeline: &str,
        run: u64,
        id: &str,
    ) -> Result<TaskRun, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let task_run = sqlx::query(
            r#"
        SELECT namespace, pipeline, run, id, task, created, started, ended, exit_code, failure,
        logs_expired, logs_removed, state, status, scheduler_id
        FROM task_runs
        WHERE namespace = ? AND pipeline = ? AND run = ? AND id = ?;
            "#,
        )
        .bind(namespace)
        .bind(pipeline)
        .bind(run as i64)
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
            failure: {
                let failure = row.get::<String, _>("failure");
                if failure.is_empty() {
                    None
                } else {
                    serde_json::from_str(&failure).unwrap()
                }
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
            state: TaskRunState::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into task_run state enum".to_string(),
                })
                .unwrap(),
            status: TaskRunStatus::from_str(row.get("status"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("status"),
                    column: "status".to_string(),
                    err: "could not parse value into task_run status enum".to_string(),
                })
                .unwrap(),
            scheduler_id: row.get("scheduler_id"),
        })
        .fetch_one(&mut conn)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(task_run)
    }

    pub async fn update_task_run_state(
        &self,
        namespace: &str,
        pipeline: &str,
        run: u64,
        id: &str,
        state: TaskRunState,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE task_runs
        SET state = ?
        WHERE namespace = ? AND pipeline = ? AND Run = ? AND id = ?;
            "#,
        )
        .bind(state.to_string())
        .bind(namespace)
        .bind(pipeline)
        .bind(run as i64)
        .bind(id)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(())
    }

    pub async fn update_task_run_status(
        &self,
        namespace: &str,
        pipeline: &str,
        run: u64,
        id: &str,
        status: TaskRunStatus,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE task_runs
        SET status = ?
        WHERE namespace = ? AND pipeline = ? AND run = ? AND id = ?;
            "#,
        )
        .bind(status.to_string())
        .bind(namespace)
        .bind(pipeline)
        .bind(run as i64)
        .bind(id)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(())
    }

    /// Update a specific task_run.
    pub async fn update_task_run(&self, task_run: &TaskRun) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        UPDATE task_runs
        SET started = ?, ended = ?, exit_code = ?, failure = ?, logs_expired = ?, logs_removed = ?,
        state = ?, status = ?, scheduler_id = ?
        WHERE namespace = ? AND pipeline = ? AND run =? AND id = ?;
            "#,
        )
        .bind(task_run.started as i64)
        .bind(task_run.ended as i64)
        .bind(task_run.exit_code)
        .bind({
            if task_run.failure.is_none() {
                None
            } else {
                Some(serde_json::to_string(&task_run.failure).unwrap())
            }
        })
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
        .bind(&task_run.namespace)
        .bind(&task_run.pipeline)
        .bind(task_run.run as i64)
        .bind(&task_run.id)
        .execute(&mut conn)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        Ok(())
    }

    pub async fn delete_task_run(
        &self,
        namespace: &str,
        pipeline: &str,
        run: u64,
        id: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        DELETE FROM task_runs
        WHERE namespace = ? AND pipeline = ? AND run = ? AND id = ?;
            "#,
        )
        .bind(namespace)
        .bind(pipeline)
        .bind(run as i64)
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
