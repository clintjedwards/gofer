use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct Deployment {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub deployment_id: i64,
    pub start_version: i64,
    pub end_version: i64,
    pub started: String,
    pub ended: String,
    pub state: String,
    pub status: String,
    pub status_reason: String,
    pub logs: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub ended: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub logs: Option<String>,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    deployment: &Deployment,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO deployments (namespace_id, pipeline_id, deployment_id, start_version, end_version, started, ended, \
        state, status, status_reason, logs) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .bind(&deployment.namespace_id)
    .bind(&deployment.pipeline_id)
    .bind(deployment.deployment_id)
    .bind(deployment.start_version)
    .bind(deployment.end_version)
    .bind(&deployment.started)
    .bind(&deployment.ended)
    .bind(&deployment.state)
    .bind(&deployment.status)
    .bind(&deployment.status_reason)
    .bind(&deployment.logs);

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
) -> Result<Vec<Deployment>, StorageError> {
    let query = sqlx::query_as::<_, Deployment>(
        "SELECT namespace_id, pipeline_id, deployment_id, start_version, end_version, started, ended, state, status, \
        status_reason, logs FROM deployments WHERE namespace_id = ? AND pipeline_id = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn list_running(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<Deployment>, StorageError> {
    let query = sqlx::query_as::<_, Deployment>(
        "SELECT namespace_id, pipeline_id, deployment_id, start_version, end_version, started, ended, state, status, \
        status_reason, logs FROM deployments WHERE namespace_id = ? AND pipeline_id = ? AND state = 'RUNNING';",
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
    deployment_id: i64,
) -> Result<Deployment, StorageError> {
    let query = sqlx::query_as::<_, Deployment>(
        "SELECT namespace_id, pipeline_id, deployment_id, start_version, \
    end_version, started, ended, state, status, status_reason, logs FROM deployments \
    WHERE namespace_id = ? AND pipeline_id = ? AND deployment_id = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(deployment_id);

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
) -> Result<Deployment, StorageError> {
    let query = sqlx::query_as::<_, Deployment>(
        "SELECT namespace_id, pipeline_id, deployment_id, start_version, \
    end_version, started, ended, state, status, status_reason, logs FROM deployments \
    WHERE namespace_id = ? AND pipeline_id = ? ORDER BY deployment_id DESC LIMIT 1;",
    )
    .bind(namespace_id)
    .bind(pipeline_id);

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
    deployment_id: i64,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE deployments SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.ended {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("ended = ");
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

    if let Some(value) = &fields.status {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.status_reason {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status_reason = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.logs {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("logs = ");
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
    update_query.push(" AND deployment_id = ");
    update_query.push_bind(deployment_id);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

// For the time being there is no need to delete a deployment and normally a deployment should not be deleted.
// But we might make an admin route that allows this.
#[allow(dead_code)]
pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    deployment_id: i64,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "DELETE FROM deployments WHERE namespace_id = ? AND pipeline_id = ? AND deployment_id = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(deployment_id);

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
    use crate::storage::tests::TestHarness;
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let mut conn = harness.write_conn().await.unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_pipeline_id".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace).await?;

        let pipeline_metadata = crate::storage::pipeline_metadata::PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::pipeline_metadata::insert(&mut conn, &pipeline_metadata).await?;

        let new_pipeline_config = crate::storage::pipeline_configs::PipelineConfig {
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

        crate::storage::pipeline_configs::insert(&mut conn, &new_pipeline_config)
            .await
            .expect("Failed to insert pipeline_config");

        let deployment = Deployment {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            deployment_id: 1,
            start_version: 1,
            end_version: 2,
            started: "2021-01-01T00:00:00Z".to_string(),
            ended: "2021-01-01T01:00:00Z".to_string(),
            state: "Deploymenting".to_string(),
            status: "Active".to_string(),
            status_reason: "No issues".to_string(),
            logs: "some_logs".into(),
        };

        insert(&mut conn, &deployment).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_deployments() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let deployments = list(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to list deployments");

        assert!(!deployments.is_empty(), "No deployments returned");

        let some_deployment = deployments
            .iter()
            .find(|n| n.deployment_id == 1)
            .expect("Deployment not found");
        assert_eq!(some_deployment.pipeline_id, "some_pipeline_id");
        assert_eq!(some_deployment.state, "Deploymenting");
    }

    #[tokio::test]
    async fn test_get_deployment() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let deployment = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to get deployment");

        assert_eq!(deployment.pipeline_id, "some_pipeline_id");
        assert_eq!(deployment.state, "Deploymenting");
    }

    #[tokio::test]
    async fn test_update_deployment() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            ended: Some("2021-01-01T02:00:00Z".to_string()),
            state: Some("Failed".to_string()),
            status: Some("Error".to_string()),
            status_reason: Some("Encountered an error".to_string()),
            logs: None,
        };

        update(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            1,
            fields_to_update,
        )
        .await
        .expect("Failed to update deployment");

        let updated_deployment = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to retrieve updated deployment");

        assert_eq!(updated_deployment.state, "Failed");
        assert_eq!(updated_deployment.status, "Error");
    }

    #[tokio::test]
    async fn test_delete_deployment() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to delete deployment");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", 1)
                .await
                .is_err(),
            "Deployment was not deleted"
        );
    }
}
