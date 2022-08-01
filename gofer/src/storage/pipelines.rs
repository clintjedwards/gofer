use crate::storage::{SqliteErrors, StorageError, MAX_ROW_LIMIT};
use futures::TryFutureExt;
use gofer_models::{pipeline, task};
use sqlx::{sqlite::SqliteRow, Acquire, QueryBuilder, Row, Sqlite, SqliteConnection};
use std::{collections::HashMap, ops::Deref};
use std::{ops::Not, str::FromStr};

#[derive(Debug, Default)]
pub struct UpdatableFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parallelism: Option<u64>,
    pub modified: Option<u64>,
    pub state: Option<pipeline::State>,
}

pub async fn list_tasks(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<task::Task>, StorageError> {
    sqlx::query(
        r#"
SELECT id, description, image, registry_auth, depends_on, variables, entrypoint, command
FROM tasks
WHERE namespace = ? AND pipeline = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .map(|row: SqliteRow| task::Task {
        id: row.get("id"),
        description: row.get("description"),
        image: row.get("image"),
        registry_auth: {
            let registry_auth = row.get::<String, _>("registry_auth");
            registry_auth
                .is_empty()
                .not()
                .then(|| serde_json::from_str(&registry_auth).unwrap())
        },
        depends_on: {
            let depends_on = row.get::<String, _>("depends_on");
            serde_json::from_str(&depends_on).unwrap()
        },
        variables: {
            let variables = row.get::<String, _>("variables");
            serde_json::from_str(&variables).unwrap()
        },
        entrypoint: {
            let entrypoint = row.get::<String, _>("entrypoint");
            serde_json::from_str(&entrypoint).unwrap()
        },
        command: {
            let command = row.get::<String, _>("command");
            serde_json::from_str(&command).unwrap()
        },
    })
    .fetch_all(conn)
    .map_err(|e| StorageError::Unknown(e.to_string()))
    .await
}

pub async fn list_trigger_settings(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<pipeline::TriggerSettings>, StorageError> {
    sqlx::query(
        r#"
SELECT kind, label, settings, error
FROM pipeline_trigger_settings
WHERE namespace = ? AND pipeline = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .map(|row: SqliteRow| pipeline::TriggerSettings {
        name: row.get("kind"),
        label: row.get("label"),
        settings: {
            let value = row.get::<String, _>("settings");
            serde_json::from_str(&value).unwrap()
        },
        error: row.get("error"),
    })
    .fetch_all(conn)
    .map_err(|e| StorageError::Unknown(e.to_string()))
    .await
}

pub async fn list_common_task_settings(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<pipeline::CommonTaskSettings>, StorageError> {
    sqlx::query(
        r#"
SELECT kind, label, settings, error
FROM pipeline_common_task_settings
WHERE namespace = ? AND pipeline = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .map(|row: SqliteRow| pipeline::CommonTaskSettings {
        name: row.get("kind"),
        label: row.get("label"),
        settings: {
            let value = row.get::<String, _>("settings");
            serde_json::from_str(&value).unwrap()
        },
        error: row.get("error"),
    })
    .fetch_all(conn)
    .map_err(|e| StorageError::Unknown(e.to_string()))
    .await
}

/// Return all pipeline for a given namespace; limited to 200 rows per response.
pub async fn list(
    conn: &mut SqliteConnection,
    offset: u64,
    limit: u64,
    namespace_id: &str,
) -> Result<Vec<pipeline::Pipeline>, StorageError> {
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
SELECT namespace, id, name, description, parallelism, created, modified, state
FROM pipelines
WHERE namespace = ?
ORDER BY created
LIMIT ?
OFFSET ?;"#,
    )
    .bind(namespace_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .map(|row: SqliteRow| pipeline::Pipeline {
        namespace: row.get("namespace"),
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        parallelism: row.get::<i64, _>("parallelism") as u64,
        created: row.get::<i64, _>("created") as u64,
        modified: row.get::<i64, _>("modified") as u64,
        state: pipeline::State::from_str(row.get("state"))
            .map_err(|_| StorageError::Parse {
                value: row.get("state"),
                column: "state".to_string(),
                err: "could not parse value into pipeline state enum".to_string(),
            })
            .unwrap(),
        tasks: HashMap::new(),
        triggers: HashMap::new(),
        common_tasks: HashMap::new(),
        store_keys: vec![],
    })
    .fetch_all(&mut tx)
    .map_err(|e| StorageError::Unknown(e.to_string()))
    .await?;

    // Then we need to populate it with information from sister tables.
    for pipeline in &mut pipelines {
        let tasks = list_tasks(&mut tx, namespace_id, &pipeline.id).await?;

        let tasks = tasks
            .into_iter()
            .map(|value| (value.id.clone(), value))
            .collect();

        pipeline.tasks = tasks;

        let triggers = list_trigger_settings(&mut tx, namespace_id, &pipeline.id).await?;

        pipeline.triggers = triggers
            .into_iter()
            .map(|value| (value.label.clone(), value))
            .collect();

        let common_tasks = list_common_task_settings(&mut tx, namespace_id, &pipeline.id).await?;

        pipeline.common_tasks = common_tasks
            .into_iter()
            .map(|value| (value.label.clone(), value))
            .collect();
    }

    tx.commit()
        .await
        .map_err(|e| StorageError::Unknown(e.to_string()))?;

    Ok(pipelines)
}

pub async fn insert_task(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    task: &task::Task,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
INSERT INTO tasks (namespace, pipeline, id, description, image, registry_auth,
    depends_on, variables, entrypoint, command)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(&task.id)
    .bind(&task.description)
    .bind(&task.image)
    .bind(
        task.registry_auth
            .is_none()
            .not()
            .then(|| Some(serde_json::to_string(&task.registry_auth).unwrap())),
    )
    .bind(serde_json::to_string(&task.depends_on).unwrap())
    .bind(serde_json::to_string(&task.variables).unwrap())
    .bind(serde_json::to_string(&task.entrypoint).unwrap())
    .bind(serde_json::to_string(&task.command).unwrap())
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

pub async fn insert_trigger_settings(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    settings: &pipeline::TriggerSettings,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
INSERT INTO pipeline_trigger_settings (namespace, pipeline, kind, label, settings, error) VALUES (?, ?, ?, ?, ?, ?);"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(&settings.name)
    .bind(&settings.label)
    .bind(serde_json::to_string(&settings.settings).unwrap())
    .bind(&settings.error)
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

pub async fn insert_common_task_settings(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    settings: &pipeline::CommonTaskSettings,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
INSERT INTO pipeline_common_task_settings (namespace, pipeline, kind, label, settings, error) VALUES (?, ?, ?, ?, ?, ?);"#,
    )
    .bind(&namespace_id)
    .bind(&pipeline_id)
    .bind(&settings.name)
    .bind(&settings.label)
    .bind(serde_json::to_string(&settings.settings).unwrap())
    .bind(&settings.error)
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

/// Insert a new pipeline.
pub async fn insert(
    conn: &mut SqliteConnection,
    pipeline: &pipeline::Pipeline,
) -> Result<(), StorageError> {
    let mut tx = conn
        .begin()
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

    sqlx::query(
        r#"
INSERT INTO pipelines (namespace, id, name, description, parallelism, state,
    created, modified)
VALUES (?, ?, ?, ?, ?, ?, ?, ?);"#,
    )
    .bind(&pipeline.namespace)
    .bind(&pipeline.id)
    .bind(&pipeline.name)
    .bind(&pipeline.description)
    .bind(pipeline.parallelism as i64)
    .bind(pipeline.state.to_string())
    .bind(pipeline.created as i64)
    .bind(pipeline.modified as i64)
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
        insert_task(&mut tx, &pipeline.namespace, &pipeline.id, task).await?;
    }

    for settings in pipeline.triggers.values() {
        insert_trigger_settings(&mut tx, &pipeline.namespace, &pipeline.id, settings).await?;
    }

    for settings in pipeline.common_tasks.values() {
        insert_common_task_settings(&mut tx, &pipeline.namespace, &pipeline.id, settings).await?;
    }

    tx.commit()
        .await
        .map_err(|e| StorageError::Unknown(e.to_string()))
}

/// Get details on a specific pipeline.
pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<pipeline::Pipeline, StorageError> {
    let mut tx = conn
        .begin()
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

    let mut pipeline = sqlx::query(
        r#"
SELECT namespace, id, name, description, parallelism, created, modified, state
FROM pipelines
WHERE namespace = ? AND id = ?
ORDER BY id
LIMIT 1;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .map(|row: SqliteRow| pipeline::Pipeline {
        namespace: row.get("namespace"),
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        parallelism: row.get::<i64, _>("parallelism") as u64,
        created: row.get::<i64, _>("created") as u64,
        modified: row.get::<i64, _>("modified") as u64,
        state: pipeline::State::from_str(row.get("state"))
            .map_err(|_| StorageError::Parse {
                value: row.get("state"),
                column: "state".to_string(),
                err: "could not parse value into pipeline state enum".to_string(),
            })
            .unwrap(),
        tasks: HashMap::new(),
        triggers: HashMap::new(),
        common_tasks: HashMap::new(),
        store_keys: vec![],
    })
    .fetch_one(&mut tx)
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await?;

    let tasks = list_tasks(&mut tx, namespace_id, pipeline_id).await?;
    pipeline.tasks = tasks
        .into_iter()
        .map(|value| (value.id.clone(), value))
        .collect();

    let triggers = list_trigger_settings(&mut tx, namespace_id, pipeline_id).await?;
    pipeline.triggers = triggers
        .into_iter()
        .map(|value| (value.label.clone(), value))
        .collect();

    let common_tasks = list_common_task_settings(&mut tx, namespace_id, pipeline_id).await?;
    pipeline.common_tasks = common_tasks
        .into_iter()
        .map(|value| (value.label.clone(), value))
        .collect();

    tx.commit()
        .await
        .map_err(|e| StorageError::Unknown(e.to_string()))?;

    Ok(pipeline)
}

pub async fn delete_task(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    task_id: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM tasks
WHERE namespace = ? AND pipeline = ? AND id = ?;"#,
    )
    .bind(&namespace_id)
    .bind(&pipeline_id)
    .bind(task_id)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

pub async fn delete_trigger_settings(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    label: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM pipeline_trigger_settings
WHERE namespace = ? AND pipeline = ? AND label = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(label)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

pub async fn delete_common_task_settings(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    label: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM pipeline_common_task_settings
WHERE namespace = ? AND pipeline = ? AND label = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(label)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}

/// Update a specific pipeline.
pub async fn update(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut tx = conn
        .begin()
        .map_err(|e| StorageError::Unknown(e.to_string()))
        .await?;

    let pipeline = get(&mut tx, namespace_id, id).await?;

    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE pipelines SET "#);

    let mut updated_fields_total = 0;

    if let Some(name) = fields.name {
        update_query.push("name = ");
        update_query.push_bind(name);
        updated_fields_total += 1;
    }

    if let Some(description) = fields.description {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("description = ");
        update_query.push_bind(description);
        updated_fields_total += 1;
    }

    if let Some(parallelism) = fields.parallelism {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("parallelism = ");
        update_query.push_bind(parallelism as i64);
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

    if let Some(modified) = fields.modified {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("modified = ");
        update_query.push_bind(modified.to_string());
    }

    update_query.push(" WHERE namespace = ");
    update_query.push_bind(namespace_id);

    update_query.push(" AND id = ");
    update_query.push_bind(id);
    update_query.push(";");

    let update_query = update_query.build();

    update_query
        .execute(&mut tx)
        .map_ok(|_| ())
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StorageError::NotFound,
            _ => StorageError::Unknown(e.to_string()),
        })
        .await?;

    for task in pipeline.tasks.values() {
        delete_task(&mut tx, &pipeline.namespace, &pipeline.id, &task.id).await?;
        insert_task(&mut tx, &pipeline.namespace, &pipeline.id, task).await?;
    }

    for settings in pipeline.triggers.values() {
        delete_trigger_settings(&mut tx, &pipeline.namespace, &pipeline.id, &settings.label)
            .await?;
        insert_trigger_settings(&mut tx, &pipeline.namespace, &pipeline.id, settings).await?;
    }

    for settings in pipeline.common_tasks.values() {
        delete_common_task_settings(&mut tx, &pipeline.namespace, &pipeline.id, &settings.label)
            .await?;
        insert_common_task_settings(&mut tx, &pipeline.namespace, &pipeline.id, settings).await?;
    }

    tx.commit()
        .await
        .map_err(|e| StorageError::Unknown(e.to_string()))
}

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        r#"
DELETE FROM pipelines
WHERE namespace = ? AND id = ?;"#,
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .execute(conn)
    .map_ok(|_| ())
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StorageError::NotFound,
        _ => StorageError::Unknown(e.to_string()),
    })
    .await
}
