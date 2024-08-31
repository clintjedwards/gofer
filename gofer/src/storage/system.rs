use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct System {
    pub bootstrap_token_created: bool,
    pub ignore_pipeline_run_events: bool,
}

pub async fn get_system_parameters(conn: &mut SqliteConnection) -> Result<System, StorageError> {
    let query = sqlx::query_as::<_, System>(
        "SELECT bootstrap_token_created, ignore_pipeline_run_events FROM system; WHERE id = 1",
    );

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update_system_parameters(
    conn: &mut SqliteConnection,
    bootstrap_token_created: Option<bool>,
    ignore_pipeline_run_events: Option<bool>,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> = QueryBuilder::new(r#"UPDATE system SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &bootstrap_token_created {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("bootstrap_token_created = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &ignore_pipeline_run_events {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("ignore_pipeline_run_events = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(" WHERE id = 1;");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let conn = harness.conn().await.unwrap();

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_update_and_get_system() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        update_system_parameters(&mut conn, Some(true), Some(true))
            .await
            .expect("Failed to update token");

        let system_parameters = get_system_parameters(&mut conn)
            .await
            .expect("Failed to retrieve updated token");

        assert!(system_parameters.ignore_pipeline_run_events);
        assert!(system_parameters.bootstrap_token_created);
    }
}
