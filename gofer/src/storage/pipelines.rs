use std::{collections::HashMap, ops::Deref};

use crate::models::{
    Pipeline, PipelineNotifierSettings, PipelineState, PipelineTriggerSettings, RunState, Task,
};
use crate::storage::{Db, SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use sqlx::{sqlite::SqliteRow, Acquire, Row};
use std::str::FromStr;

impl Db {
    /// Return all pipeline for a given namespace; limited to 200 rows per response.
    pub async fn list_pipelines(
        &self,
        offset: u64,
        limit: u64,
        namespace: &str,
    ) -> Result<Vec<Pipeline>, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut tx = conn
            .begin()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut limit = limit;

        if limit == 0 || limit > MAX_ROW_LIMIT {
            limit = MAX_ROW_LIMIT;
        }

        // First we need to get the general pipeline information.
        let mut pipelines = sqlx::query(
            r#"
        SELECT namespace, id, name, description, parallelism, created, modified, state, store_keys
        FROM pipelines
        WHERE namespace = ?
        ORDER BY created
        LIMIT ?
        OFFSET ?;
            "#,
        )
        .bind(namespace)
        .bind(limit as i64)
        .bind(offset as i64)
        .map(|row: SqliteRow| Pipeline {
            namespace: row.get("namespace"),
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            last_run_id: 0,
            last_run_time: 0,
            parallelism: row.get::<i64, _>("parallelism") as u64,
            created: row.get::<i64, _>("created") as u64,
            modified: row.get::<i64, _>("modified") as u64,
            state: PipelineState::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into pipeline state enum".to_string(),
                })
                .unwrap(),
            tasks: HashMap::new(),
            triggers: HashMap::new(),
            notifiers: HashMap::new(),
            store_keys: {
                let keys = row.get::<String, _>("store_keys");
                serde_json::from_str(&keys).unwrap()
            },
        })
        .fetch_all(&mut tx)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

        // Then we need to populate it with information from sister tables.
        for pipeline in &mut pipelines {
            struct Run {
                id: u64,
                started: u64,
            }

            let last_run = match sqlx::query(
                r#"
            SELECT id, started
            FROM runs
            WHERE namespace = ? AND pipeline = ?
            ORDER BY started DESC
            LIMIT 1;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .map(|row: SqliteRow| Run {
                id: row.get::<i64, _>("id") as u64,
                started: row.get::<i64, _>("started") as u64,
            })
            .fetch_one(&mut tx)
            .await
            {
                Ok(last_run) => last_run,
                Err(storage_err) => match storage_err {
                    sqlx::Error::RowNotFound => Run { id: 0, started: 0 },
                    _ => panic!("{}", storage_err.to_string()),
                },
            };

            pipeline.last_run_id = last_run.id;
            pipeline.last_run_time = last_run.started;

            let tasks = sqlx::query(
                r#"
            SELECT id, description, image, registry_auth, depends_on,
            variables, exec
            FROM tasks
            WHERE namespace = ? AND pipeline = ?;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .map(|row: SqliteRow| Task {
                id: row.get("id"),
                description: row.get("description"),
                image: row.get("image"),
                registry_auth: {
                    let registry_auth = row.get::<String, _>("registry_auth");
                    if registry_auth.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&registry_auth).unwrap()
                    }
                },
                depends_on: {
                    let depends_on = row.get::<String, _>("depends_on");
                    serde_json::from_str(&depends_on).unwrap()
                },
                variables: {
                    let variables = row.get::<String, _>("variables");
                    serde_json::from_str(&variables).unwrap()
                },
                exec: {
                    let exec = row.get::<String, _>("exec");
                    if exec.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&exec).unwrap()
                    }
                },
            })
            .fetch_all(&mut tx)
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await
            .unwrap();

            let tasks = tasks
                .into_iter()
                .map(|value| (value.id.clone(), value))
                .collect();

            pipeline.tasks = tasks;

            let triggers = sqlx::query(
                r#"
            SELECT kind, label, settings, error
            FROM pipeline_trigger_settings
            WHERE namespace = ? AND pipeline = ?;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .map(|row: SqliteRow| PipelineTriggerSettings {
                kind: row.get("kind"),
                label: row.get("label"),
                settings: {
                    let value = row.get::<String, _>("settings");
                    serde_json::from_str(&value).unwrap()
                },
                error: row.get("error"),
            })
            .fetch_all(&mut tx)
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await
            .unwrap();

            let triggers = triggers
                .into_iter()
                .map(|value| (value.label.clone(), value))
                .collect();

            pipeline.triggers = triggers;

            let notifiers = sqlx::query(
                r#"
            SELECT kind, label, settings, error
            FROM pipeline_notifier_settings
            WHERE namespace = ? AND pipeline = ?;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .map(|row: SqliteRow| PipelineNotifierSettings {
                kind: row.get("kind"),
                label: row.get("label"),
                settings: {
                    let value = row.get::<String, _>("settings");
                    serde_json::from_str(&value).unwrap()
                },
                error: row.get("error"),
            })
            .fetch_all(&mut tx)
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await
            .unwrap();

            let notifiers = notifiers
                .into_iter()
                .map(|value| (value.label.clone(), value))
                .collect();

            pipeline.notifiers = notifiers;
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .unwrap();

        Ok(pipelines)
    }

    /// Create a new pipeline.
    pub async fn create_pipeline(&self, pipeline: &Pipeline) -> Result<(), StorageError> {
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
        INSERT INTO pipelines (namespace, id, name, description, parallelism, state,
            created, modified, store_keys)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .bind(&pipeline.name)
        .bind(&pipeline.description)
        .bind(pipeline.parallelism as i64)
        .bind(pipeline.state.to_string())
        .bind(pipeline.created as i64)
        .bind(pipeline.modified as i64)
        .bind(serde_json::to_string(&pipeline.store_keys).unwrap())
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

        for task in pipeline.tasks.values() {
            sqlx::query(
                r#"
            INSERT INTO tasks (namespace, pipeline, id, description, image, registry_auth,
                depends_on, variables, exec)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&task.id)
            .bind(&task.description)
            .bind(&task.image)
            .bind({
                if task.registry_auth.is_none() {
                    None
                } else {
                    Some(serde_json::to_string(&task.registry_auth).unwrap())
                }
            })
            .bind(serde_json::to_string(&task.depends_on).unwrap())
            .bind(serde_json::to_string(&task.variables).unwrap())
            .bind(if task.exec.is_none() {
                None
            } else {
                Some(serde_json::to_string(&task.exec).unwrap())
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
        }

        for settings in pipeline.triggers.values() {
            sqlx::query(
                r#"
            INSERT INTO pipeline_trigger_settings (namespace, pipeline, kind, label, settings, error)
            VALUES (?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&settings.kind)
            .bind(&settings.label)
            .bind(serde_json::to_string(&settings.settings).unwrap())
            .bind(&settings.error)
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
        }

        for settings in pipeline.notifiers.values() {
            sqlx::query(
                r#"
            INSERT INTO pipeline_notifier_settings (namespace, pipeline, kind, label, settings, error)
            VALUES (?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&settings.kind)
            .bind(&settings.label)
            .bind(serde_json::to_string(&settings.settings).unwrap())
            .bind(&settings.error)
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
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .unwrap();

        Ok(())
    }

    /// Get details on a specific pipeline.
    pub async fn get_pipeline(&self, namespace: &str, id: &str) -> Result<Pipeline, StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut tx = conn
            .begin()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut pipeline = sqlx::query(
            r#"
            SELECT namespace, id, name, description, parallelism, created, modified, state, store_keys
            FROM pipelines
            WHERE namespace = ? AND id = ?
            ORDER BY id
            LIMIT 1;
                "#,
        )
        .bind(namespace)
        .bind(id)
        .map(|row: SqliteRow|
            Pipeline {
            namespace: row.get("namespace"),
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            last_run_id: 0,
            last_run_time: 0,
            parallelism: row.get::<i64, _>("parallelism") as u64,
            created: row.get::<i64, _>("created") as u64,
            modified: row.get::<i64, _>("modified") as u64,
            state: PipelineState::from_str(row.get("state"))
                .map_err(|_| {
                    StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into pipeline state enum".to_string(),
                }})
                .unwrap(),
            tasks: HashMap::new(),
            triggers: HashMap::new(),
            notifiers: HashMap::new(),
            store_keys: {
                let keys = row.get::<String, _>("store_keys");
                serde_json::from_str(&keys).unwrap()
            },
        })
        .fetch_one(&mut tx)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        struct Run {
            id: u64,
            started: u64,
        }

        let last_run = match sqlx::query(
            r#"
        SELECT id, started
        FROM runs
        WHERE namespace = ? AND pipeline = ?
        ORDER BY started DESC
        LIMIT 1;
            "#,
        )
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .map(|row: SqliteRow| Run {
            id: row.get::<i64, _>("id") as u64,
            started: row.get::<i64, _>("started") as u64,
        })
        .fetch_one(&mut tx)
        .await
        {
            Ok(last_run) => last_run,
            Err(storage_err) => match storage_err {
                sqlx::Error::RowNotFound => Run { id: 0, started: 0 },
                _ => panic!("{}", storage_err.to_string()),
            },
        };

        pipeline.last_run_id = last_run.id;
        pipeline.last_run_time = last_run.started;

        let tasks = sqlx::query(
            r#"
        SELECT namespace, pipeline, id, description, image, registry_auth, depends_on,
        variables, exec
        FROM tasks
        WHERE namespace = ? AND pipeline = ?;
            "#,
        )
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .map(|row: SqliteRow| Task {
            id: row.get("id"),
            description: row.get("description"),
            image: row.get("image"),
            registry_auth: {
                let registry_auth = row.get::<String, _>("registry_auth");
                if registry_auth.is_empty() {
                    None
                } else {
                    serde_json::from_str(&registry_auth).unwrap()
                }
            },
            depends_on: {
                let depends_on = row.get::<String, _>("depends_on");
                serde_json::from_str(&depends_on).unwrap()
            },
            variables: {
                let variables = row.get::<String, _>("variables");
                serde_json::from_str(&variables).unwrap()
            },
            exec: {
                let exec = row.get::<String, _>("exec");
                if exec.is_empty() {
                    None
                } else {
                    serde_json::from_str(&exec).unwrap()
                }
            },
        })
        .fetch_all(&mut tx)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await
        .unwrap();

        let tasks = tasks
            .into_iter()
            .map(|value| (value.id.clone(), value))
            .collect();

        pipeline.tasks = tasks;

        let triggers = sqlx::query(
            r#"
        SELECT kind, label, settings, error
        FROM pipeline_trigger_settings
        WHERE namespace = ? AND pipeline = ?;
            "#,
        )
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .map(|row: SqliteRow| PipelineTriggerSettings {
            kind: row.get("kind"),
            label: row.get("label"),
            settings: {
                let value = row.get::<String, _>("settings");
                serde_json::from_str(&value).unwrap()
            },
            error: row.get("error"),
        })
        .fetch_all(&mut tx)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await
        .unwrap();

        let triggers = triggers
            .into_iter()
            .map(|value| (value.label.clone(), value))
            .collect();

        pipeline.triggers = triggers;

        let notifiers = sqlx::query(
            r#"
        SELECT kind, label, settings, error
        FROM pipeline_notifier_settings
        WHERE namespace = ? AND pipeline = ?;
            "#,
        )
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .map(|row: SqliteRow| PipelineNotifierSettings {
            kind: row.get("kind"),
            label: row.get("label"),
            settings: {
                let value = row.get::<String, _>("settings");
                serde_json::from_str(&value).unwrap()
            },
            error: row.get("error"),
        })
        .fetch_all(&mut tx)
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await
        .unwrap();

        let notifiers = notifiers
            .into_iter()
            .map(|value| (value.label.clone(), value))
            .collect();

        pipeline.notifiers = notifiers;

        tx.commit()
            .await
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .unwrap();

        Ok(pipeline)
    }

    pub async fn update_pipeline_state(
        &self,
        namespace: &str,
        id: &str,
        state: PipelineState,
    ) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let mut tx = conn
            .begin()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        let pipeline = sqlx::query(
            r#"
                SELECT namespace, id, name, description, parallelism, created, modified, state
                FROM pipelines
                WHERE namespace = ? AND id = ?
                ORDER BY id
                LIMIT 1;
            "#,
        )
        .bind(namespace)
        .bind(id)
        .map(|row: SqliteRow| Pipeline {
            namespace: row.get("namespace"),
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            last_run_id: 0,
            last_run_time: 0,
            parallelism: row.get::<i64, _>("parallelism") as u64,
            created: row.get::<i64, _>("created") as u64,
            modified: row.get::<i64, _>("modified") as u64,
            state: PipelineState::from_str(row.get("state"))
                .map_err(|_| StorageError::Parse {
                    value: row.get("state"),
                    column: "state".to_string(),
                    err: "could not parse value into pipeline state enum".to_string(),
                })
                .unwrap(),
            tasks: HashMap::new(),
            triggers: HashMap::new(),
            notifiers: HashMap::new(),
            store_keys: {
                let keys = row.get::<String, _>("store_keys");
                serde_json::from_str(&keys).unwrap()
            },
        })
        .fetch_one(&mut tx)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        struct Run {
            state: RunState,
        }

        let last_run: Option<Run> = match sqlx::query(
            r#"
        SELECT state
        FROM runs
        WHERE namespace = ? AND pipeline = ?
        ORDER BY started DESC
        LIMIT 1;
            "#,
        )
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .map(|row: SqliteRow| Run {
            state: RunState::from_str(&row.get::<String, _>("state")).unwrap(),
        })
        .fetch_one(&mut tx)
        .await
        {
            Ok(last_run) => Some(last_run),
            Err(storage_err) => match storage_err {
                sqlx::Error::RowNotFound => None,
                _ => panic!("{}", storage_err.to_string()),
            },
        };

        if let Some(last_run) = last_run {
            if last_run.state != RunState::Complete {
                return Err(StorageError::FailedPrecondition);
            }
        }

        sqlx::query(
            r#"
        UPDATE pipelines
        SET name = ?, description = ?, parallelism = ?, state = ?, modified = ?, store_keys = ?
        WHERE namespace = ? AND id = ?;
            "#,
        )
        .bind(&pipeline.name)
        .bind(&pipeline.description)
        .bind(pipeline.parallelism as i64)
        .bind(state.to_string())
        .bind(pipeline.modified as i64)
        .bind(serde_json::to_string(&pipeline.store_keys).unwrap())
        .execute(&mut tx)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .unwrap();

        Ok(())
    }

    /// Update a specific pipeline.
    pub async fn update_pipeline(&self, pipeline: &Pipeline) -> Result<(), StorageError> {
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
        UPDATE pipelines
        SET name = ?, description = ?, parallelism = ?, state = ?, modified = ?, store_keys = ?
        WHERE namespace = ? AND id = ?;
            "#,
        )
        .bind(&pipeline.name)
        .bind(&pipeline.description)
        .bind(pipeline.parallelism as i64)
        .bind(pipeline.state.to_string())
        .bind(pipeline.modified as i64)
        .bind(serde_json::to_string(&pipeline.store_keys).unwrap())
        .bind(&pipeline.namespace)
        .bind(&pipeline.id)
        .execute(&mut tx)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

        for (id, task) in &pipeline.tasks {
            sqlx::query(
                r#"
            DELETE FROM tasks
            WHERE namespace = ? AND pipeline = ? AND id = ?;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(id)
            .execute(&mut tx)
            .map_ok(|_| ())
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => StorageError::NotFound,
                _ => StorageError::Unknown(e.to_string()),
            })
            .await?;

            sqlx::query(
                r#"
            INSERT INTO tasks (namespace, pipeline, id, description, image, registry_auth,
                depends_on, variables, exec)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(&pipeline.id)
            .bind(&pipeline.namespace)
            .bind(&task.id)
            .bind(&task.description)
            .bind(&task.image)
            .bind({
                if task.registry_auth.is_none() {
                    None
                } else {
                    Some(serde_json::to_string(&task.registry_auth).unwrap())
                }
            })
            .bind(serde_json::to_string(&task.depends_on).unwrap())
            .bind(serde_json::to_string(&task.variables).unwrap())
            .bind({
                if task.exec.is_none() {
                    None
                } else {
                    Some(serde_json::to_string(&task.exec).unwrap())
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
        }

        for settings in pipeline.triggers.values() {
            sqlx::query(
                r#"
            DELETE FROM pipeline_trigger_settings
            WHERE namespace = ? AND pipeline = ? AND label = ?;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&settings.label)
            .execute(&mut tx)
            .map_ok(|_| ())
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => StorageError::NotFound,
                _ => StorageError::Unknown(e.to_string()),
            })
            .await?;

            sqlx::query(
                r#"
            INSERT INTO pipeline_trigger_settings (namespace, pipeline, kind, label, settings, error)
            VALUES (?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&settings.kind)
            .bind(&settings.label)
            .bind(&settings.error)
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
        }

        for settings in pipeline.notifiers.values() {
            sqlx::query(
                r#"
            DELETE FROM pipeline_notifier_settings
            WHERE namespace = ? AND pipeline = ? AND label = ?;
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&settings.label)
            .execute(&mut tx)
            .map_ok(|_| ())
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => StorageError::NotFound,
                _ => StorageError::Unknown(e.to_string()),
            })
            .await?;

            sqlx::query(
                r#"
            INSERT INTO pipeline_notifier_settings (namespace, pipeline, kind, label, settings, error)
            VALUES (?, ?, ?, ?, ?, ?);
                "#,
            )
            .bind(&pipeline.namespace)
            .bind(&pipeline.id)
            .bind(&settings.kind)
            .bind(&settings.label)
            .bind(&settings.error)
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
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .unwrap();

        Ok(())
    }

    pub async fn delete_pipeline(&self, namespace: &str, id: &str) -> Result<(), StorageError> {
        let mut conn = self
            .pool
            .acquire()
            .map_err(|e| StorageError::Unknown(e.to_string()))
            .await?;

        sqlx::query(
            r#"
        DELETE FROM pipelines
        WHERE namespace = ? AND id = ?;
            "#,
        )
        .bind(namespace)
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
