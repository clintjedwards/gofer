use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct Run {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub pipeline_config_version: i64,
    pub run_id: i64,
    pub started: String,
    pub ended: String,
    pub state: String,
    pub status: String,
    pub status_reason: String,
    pub initiator: String,
    pub variables: String,
    pub token_id: Option<String>,
    pub store_objects_expired: bool,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub ended: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub variables: Option<String>,
    pub store_objects_expired: Option<bool>,
}

pub async fn insert(conn: &mut SqliteConnection, run: &Run) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO runs (namespace_id, pipeline_id, pipeline_config_version, run_id, \
        started, ended, state, status, status_reason, initiator, variables, token_id, store_objects_expired)\
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);",
    )
    .bind(&run.namespace_id)
    .bind(&run.pipeline_id)
    .bind(run.pipeline_config_version)
    .bind(run.run_id)
    .bind(&run.started)
    .bind(&run.ended)
    .bind(&run.state)
    .bind(&run.status)
    .bind(&run.status_reason)
    .bind(&run.initiator)
    .bind(&run.variables)
    .bind(&run.token_id)
    .bind(run.store_objects_expired);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

/// Sorted by run_id ascending by default.
pub async fn list(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    offset: i64,
    limit: i64,
    reverse: bool,
) -> Result<Vec<Run>, StorageError> {
    let order_by = if reverse { "DESC" } else { "ASC" };

    let query_str = format!(
        "SELECT namespace_id, pipeline_id, pipeline_config_version, run_id, started, ended, \
    state, status, status_reason, initiator, variables, token_id, store_objects_expired FROM \
    runs WHERE namespace_id = ? AND pipeline_id = ? ORDER BY run_id {} LIMIT ? OFFSET ?;",
        order_by
    );

    let query = sqlx::query_as::<_, Run>(&query_str)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(limit)
        .bind(offset);

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
    run_id: i64,
) -> Result<Run, StorageError> {
    let query = sqlx::query_as::<_, Run>("SELECT namespace_id, pipeline_id, pipeline_config_version, run_id, started, \
    ended, state, status, status_reason, initiator, variables, token_id, store_objects_expired FROM runs WHERE \
    namespace_id = ? AND pipeline_id = ? AND run_id = ?;",)
        .bind(namespace_id)
        .bind(pipeline_id)
        .bind(run_id);

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
) -> Result<Run, StorageError> {
    let query = sqlx::query_as::<_, Run>("SELECT namespace_id, pipeline_id, pipeline_config_version, run_id, started, \
    ended, state, status, status_reason, initiator, variables, token_id, store_objects_expired FROM runs WHERE \
    namespace_id = ? AND pipeline_id = ? Order By run_id DESC;",)
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
    run_id: i64,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE runs SET "#);
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

    if let Some(value) = &fields.variables {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("variables = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.store_objects_expired {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("store_objects_expired = ");
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
    update_query.push(" AND run_id = ");
    update_query.push_bind(run_id);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

// For the time being there is no need to delete a run and normally a run should not be deleted. But we might make
// an admin route that allows this.
#[allow(dead_code)]
pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
) -> Result<(), StorageError> {
    let query =
        sqlx::query("DELETE FROM runs WHERE namespace_id = ? AND pipeline_id = ? AND run_id = ?;")
            .bind(namespace_id)
            .bind(pipeline_id)
            .bind(run_id);

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

        let run1 = Run {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            pipeline_config_version: 1,
            run_id: 1,
            started: "2021-01-01T00:00:00Z".to_string(),
            ended: "2021-01-01T01:00:00Z".to_string(),
            state: "Running".to_string(),
            status: "Active".to_string(),
            status_reason: "No issues".to_string(),
            initiator: "UserA".to_string(),
            variables: "key=value".to_string(),
            token_id: Some("some_id".into()),
            store_objects_expired: false,
        };

        let run2 = Run {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            pipeline_config_version: 1,
            run_id: 2,
            started: "2021-01-01T00:00:00Z".to_string(),
            ended: "2021-01-01T01:00:00Z".to_string(),
            state: "Running".to_string(),
            status: "Active".to_string(),
            status_reason: "No issues".to_string(),
            initiator: "UserA".to_string(),
            variables: "key=value".to_string(),
            token_id: Some("some_id".into()),
            store_objects_expired: false,
        };

        let run3 = Run {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            pipeline_config_version: 1,
            run_id: 3,
            started: "2021-01-01T00:00:00Z".to_string(),
            ended: "2021-01-01T01:00:00Z".to_string(),
            state: "Running".to_string(),
            status: "Active".to_string(),
            status_reason: "No issues".to_string(),
            initiator: "UserA".to_string(),
            variables: "key=value".to_string(),
            token_id: Some("some_id".into()),
            store_objects_expired: false,
        };

        insert(&mut conn, &run1).await?;
        insert(&mut conn, &run2).await?;
        insert(&mut conn, &run3).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_runs() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        // Test fetching with sorting by run_id ascending
        let runs_asc = list(&mut conn, "some_id", "some_pipeline_id", 0, 10, false)
            .await
            .expect("Failed to list runs in ascending order");

        assert_eq!(runs_asc.len(), 3, "Should return all runs");
        assert_eq!(runs_asc[0].run_id, 1, "First run should have run_id 1");
        assert_eq!(runs_asc[1].run_id, 2, "Second run should have run_id 2");
        assert_eq!(runs_asc[2].run_id, 3, "Third run should have run_id 3");

        // Test fetching with sorting by run_id descending
        let runs_desc = list(&mut conn, "some_id", "some_pipeline_id", 0, 10, true)
            .await
            .expect("Failed to list runs in descending order");

        assert_eq!(runs_desc.len(), 3, "Should return all runs");
        assert_eq!(runs_desc[0].run_id, 3, "First run should have run_id 3");
        assert_eq!(runs_desc[1].run_id, 2, "Second run should have run_id 2");
        assert_eq!(runs_desc[2].run_id, 1, "Third run should have run_id 1");

        // Test limit and offset
        let limited_runs = list(&mut conn, "some_id", "some_pipeline_id", 1, 1, false)
            .await
            .expect("Failed to list runs with limit and offset");

        assert_eq!(limited_runs.len(), 1, "Should return one run due to limit");
        assert_eq!(
            limited_runs[0].run_id, 2,
            "Should return the second run due to offset"
        );
    }

    #[tokio::test]
    async fn test_get_latest_run() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        // Test fetching with sorting by run_id ascending
        let run = get_latest(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to get last run");

        assert_eq!(run.run_id, 3, "latest run should be 3");
    }

    #[tokio::test]
    async fn test_get_run() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let run = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to get run");

        assert_eq!(run.pipeline_id, "some_pipeline_id");
        assert_eq!(run.state, "Running");
    }

    #[tokio::test]
    async fn test_update_run() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            ended: Some("2021-01-01T02:00:00Z".to_string()),
            state: Some("Failed".to_string()),
            status: Some("Error".to_string()),
            status_reason: Some("Encountered an error".to_string()),
            variables: Some("key1=value1,key2=value2".to_string()),
            store_objects_expired: Some(true),
        };

        update(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            1,
            fields_to_update,
        )
        .await
        .expect("Failed to update run");

        let updated_run = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to retrieve updated run");

        assert_eq!(updated_run.state, "Failed");
        assert_eq!(updated_run.status, "Error");
    }

    #[tokio::test]
    async fn test_delete_run() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1)
            .await
            .expect("Failed to delete run");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", 1)
                .await
                .is_err(),
            "Run was not deleted"
        );
    }
}
