use crate::storage::{epoch_milli, map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct PipelineMetadata {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub state: String,
    pub created: String,
    pub modified: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub state: Option<String>,
    pub modified: String,
}

impl Default for UpdatableFields {
    fn default() -> Self {
        Self {
            state: Default::default(),
            modified: epoch_milli().to_string(),
        }
    }
}

pub async fn insert(
    conn: &mut SqliteConnection,
    pipeline_metadata: &PipelineMetadata,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO pipeline_metadata (namespace_id, pipeline_id, state, created, modified) VALUES (?, ?, ?, ?, ?);",
    )
    .bind(&pipeline_metadata.namespace_id)
    .bind(&pipeline_metadata.pipeline_id)
    .bind(&pipeline_metadata.state)
    .bind(&pipeline_metadata.created)
    .bind(&pipeline_metadata.modified);

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
) -> Result<Vec<PipelineMetadata>, StorageError> {
    let query = sqlx::query_as::<_, PipelineMetadata>(
        "SELECT namespace_id, pipeline_id, state, created, modified FROM pipeline_metadata WHERE namespace_id = ?;",
    )
    .bind(namespace_id);

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
) -> Result<PipelineMetadata, StorageError> {
    let query = sqlx::query_as(
        "SELECT namespace_id, pipeline_id, state, created, modified FROM pipeline_metadata WHERE namespace_id = ? AND pipeline_id = ?;",
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
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE pipeline_metadata SET "#);
    let mut updated_fields_total = 0;

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

    update_query.push(", ");
    update_query.push("modified = ");
    update_query.push_bind(fields.modified);

    update_query.push(" WHERE namespace_id = ");
    update_query.push_bind(namespace_id);
    update_query.push(" AND pipeline_id = ");
    update_query.push_bind(pipeline_id);
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
) -> Result<(), StorageError> {
    let query =
        sqlx::query("DELETE FROM pipeline_metadata WHERE namespace_id = ? AND pipeline_id = ?;")
            .bind(namespace_id)
            .bind(pipeline_id);

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

        insert(&mut conn, &pipeline_metadata).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_pipeline_metadatas() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let pipeline_metadatas = list(&mut conn, "some_id")
            .await
            .expect("Failed to list pipeline_metadatas");

        // Assert that we got at least one pipeline_metadata back
        assert!(
            !pipeline_metadatas.is_empty(),
            "No pipeline_metadatas returned"
        );

        // Assuming you want to check if the inserted pipeline_metadata is in the list
        let some_pipeline_metadata = pipeline_metadatas
            .iter()
            .find(|n| n.namespace_id == "some_id")
            .expect("PipelineMetadata not found");
        assert_eq!(some_pipeline_metadata.pipeline_id, "some_pipeline_id");
        assert_eq!(some_pipeline_metadata.state, "some_state");
    }

    #[tokio::test]
    async fn test_insert_pipeline_metadata() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let new_pipeline_metadata = PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "new_pipeline_id".into(),
            state: "new_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        insert(&mut conn, &new_pipeline_metadata)
            .await
            .expect("Failed to insert pipeline_metadata");

        let retrieved_pipeline_metadata = get(&mut conn, "some_id", "new_pipeline_id")
            .await
            .expect("Failed to retrieve pipeline_metadata");

        assert_eq!(retrieved_pipeline_metadata.namespace_id, "some_id");
        assert_eq!(retrieved_pipeline_metadata.pipeline_id, "new_pipeline_id");
        assert_eq!(retrieved_pipeline_metadata.state, "new_state");
    }

    #[tokio::test]
    async fn test_get_pipeline_metadata() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let pipeline_metadata = get(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to get pipeline_metadata");

        assert_eq!(pipeline_metadata.namespace_id, "some_id");
        assert_eq!(pipeline_metadata.pipeline_id, "some_pipeline_id");
    }

    #[tokio::test]
    async fn test_update_pipeline_metadata() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            state: Some("updated_state".into()),
            modified: "updated_time".into(),
        };

        update(&mut conn, "some_id", "some_pipeline_id", fields_to_update)
            .await
            .expect("Failed to update pipeline_metadata");

        let updated_pipeline_metadata = get(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to retrieve updated pipeline_metadata");

        assert_eq!(updated_pipeline_metadata.state, "updated_state");
    }

    #[tokio::test]
    async fn test_delete_pipeline_metadata() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id")
            .await
            .expect("Failed to delete pipeline_metadata");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id").await.is_err(),
            "PipelineMetadata was not deleted"
        );
    }
}
