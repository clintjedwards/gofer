use crate::storage::{epoch_milli, map_rusqlite_error, Executable, StorageError};
use futures::TryFutureExt;
use rusqlite::Row;
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct System {
    pub bootstrap_token_created: bool,
    pub ignore_pipeline_run_events: bool,
}

impl From<&Row<'_>> for System {
    fn from(row: &Row) -> Self {
        Self {
            bootstrap_token_created: row.get_unwrap("bootstrap_token_created"),
            ignore_pipeline_run_events: row.get_unwrap("ignore_pipeline_run_events"),
        }
    }
}

#[derive(Iden)]
enum SystemTable {
    Table,
    BootstrapTokenCreated,
    IgnorePipelineRunEvents,
}

pub fn get_system_parameters(conn: &dyn Executable) -> Result<System, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            SystemTable::BootstrapTokenCreated,
            SystemTable::IgnorePipelineRunEvents,
        ])
        .from(SystemTable::Table)
        .and_where(Expr::col(SystemTable::Id).eq(1))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(System::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update_system_parameters(
    conn: &dyn Executable,
    bootstrap_token_created: Option<bool>,
    ignore_pipeline_run_events: Option<bool>,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(SystemTable::Table);

    if let Some(value) = bootstrap_token_created {
        query.value(SystemTable::BootstrapTokenCreated, value.into());
    }

    if let Some(value) = ignore_pipeline_run_events {
        query.value(SystemTable::IgnorePipelineRunEvents, value.into());
    }

    if query.is_empty_values() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query.and_where(Expr::col(SystemTable::Id).eq(1));
    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{tests::TestHarness, Executable};

    async fn setup() -> Result<(TestHarness, impl Executable), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let conn = harness.write_conn().unwrap();

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
