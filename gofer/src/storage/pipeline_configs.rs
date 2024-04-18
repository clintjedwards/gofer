use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct PipelineConfig {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub version: i64,
    pub parallelism: i64,
    pub name: String,
    pub description: String,
    pub registered: String,
    pub deprecated: String,
    pub state: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub deprecated: Option<String>,
    pub state: Option<String>,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    pipeline_config: &PipelineConfig,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO pipeline_configs (namespace_id, pipeline_id, version, parallelism, name, description, registered, \
            deprecated, state) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .bind(&pipeline_config.namespace_id)
    .bind(&pipeline_config.pipeline_id)
    .bind(pipeline_config.version)
    .bind(pipeline_config.parallelism)
    .bind(&pipeline_config.name)
    .bind(&pipeline_config.description)
    .bind(&pipeline_config.registered)
    .bind(&pipeline_config.deprecated)
    .bind(&pipeline_config.state);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<PipelineConfig>, StorageError> {
    let query = sqlx::query_as::<_, PipelineConfig>(
        "SELECT namespace_id, pipeline_id, version, parallelism, name, description, registered, deprecated, state \
        FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? ORDER BY version DESC;",
    )
    .bind(namespace_id)
    .bind(pipeline_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
) -> Result<PipelineConfig, StorageError> {
    let query = sqlx::query_as::<_, PipelineConfig>(
        "SELECT namespace_id, pipeline_id, version, parallelism, name, description, registered, deprecated, state \
        FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? AND version = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(version);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get_latest(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<PipelineConfig, StorageError> {
    let query = sqlx::query_as::<_, PipelineConfig>(
        "SELECT namespace_id, pipeline_id, version, parallelism, name, description, registered, deprecated, state \
        FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? Order By version DESC;",
    )
    .bind(namespace_id)
    .bind(pipeline_id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get_latest_w_state(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    state: &str,
) -> Result<PipelineConfig, StorageError> {
    let query = sqlx::query_as::<_, PipelineConfig>(
        "SELECT namespace_id, pipeline_id, version, parallelism, name, description, registered, deprecated, state \
        FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? AND state = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(state);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE pipeline_configs SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.deprecated {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("deprecated = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.state {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("state = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(" WHERE namespace_id = ");
    update_query.push_bind(namespace_id);
    update_query.push(" AND pipeline_id = ");
    update_query.push_bind(pipeline_id);
    update_query.push(" AND version = ");
    update_query.push_bind(version);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "DELETE FROM pipeline_configs WHERE namespace_id = ? AND pipeline_id = ? AND version = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(version);

    let sql = query.sql();

    query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        pipeline_configs::PipelineConfig,
        pipeline_metadata::{self, PipelineMetadata},
        tests::TestHarness,
    };
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let mut conn = harness.conn().await.unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_name".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace).await?;

        let pipeline_metadata = PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        pipeline_metadata::insert(&mut conn, &pipeline_metadata).await?;

        let new_pipeline_config = PipelineConfig {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            version: 1,
            parallelism: 4,
            name: "New Test Pipeline".to_string(),
            description: "A newly inserted test pipeline".to_string(),
            registered: "2023-01-01".to_string(),
            deprecated: "none".to_string(),
            state: "active".to_string(),
        };

        insert(&mut conn, &new_pipeline_config)
            .await
            .expect("Failed to insert pipeline_config");

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_pipeline_configs() -> Result<(), Box<dyn std::error::Error>> {
        let (_harness, mut conn) = setup().await?;

        let pipeline_configs = list(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to list pipeline_configs");

        assert!(!pipeline_configs.is_empty(), "No pipeline_configs returned");

        let some_pipeline_config = pipeline_configs
            .iter()
            .find(|n| n.namespace_id == "some_id" && n.pipeline_id == "some_pipeline_id")
            .expect("PipelineConfig not found");
        assert_eq!(some_pipeline_config.state, "active");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_pipeline_config() -> Result<(), Box<dyn std::error::Error>> {
        let (_harness, mut conn) = setup().await?;

        let fields_to_update = UpdatableFields {
            deprecated: Some("2024-01-01".to_string()),
            state: Some("deprecated".to_string()),
        };

        update(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            1,
            fields_to_update,
        )
        .await
        .expect("Failed to update pipeline_config");

        let updated_pipeline_config = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to retrieve updated pipeline_config");

        assert_eq!(updated_pipeline_config.state, "deprecated");
        assert_eq!(updated_pipeline_config.deprecated, "2024-01-01");

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_pipeline_config() -> Result<(), Box<dyn std::error::Error>> {
        let (_harness, mut conn) = setup().await?;

        delete(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to delete pipeline_config");

        let result = get(&mut conn, "some_id", "some_pipeline_id", 1).await;
        assert!(result.is_err(), "PipelineConfig was not deleted");

        Ok(())
    }
}
