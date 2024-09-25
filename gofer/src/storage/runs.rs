use crate::storage::{map_rusqlite_error, StorageError};
use futures::TryFutureExt;
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
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

// Implementing From<&Row> for Run to extract values from the rusqlite Row
impl From<&Row<'_>> for Run {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            pipeline_config_version: row.get_unwrap("pipeline_config_version"),
            run_id: row.get_unwrap("run_id"),
            started: row.get_unwrap("started"),
            ended: row.get_unwrap("ended"),
            state: row.get_unwrap("state"),
            status: row.get_unwrap("status"),
            status_reason: row.get_unwrap("status_reason"),
            initiator: row.get_unwrap("initiator"),
            variables: row.get_unwrap("variables"),
            token_id: row.get_unwrap("token_id"),
            store_objects_expired: row.get_unwrap("store_objects_expired"),
        }
    }
}

// Enum representing the columns of the Run table using the Iden trait from sea-query
#[derive(Iden)]
enum RunTable {
    Table,
    NamespaceId,
    PipelineId,
    PipelineConfigVersion,
    RunId,
    Started,
    Ended,
    State,
    Status,
    StatusReason,
    Initiator,
    Variables,
    TokenId,
    StoreObjectsExpired,
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

pub fn insert(conn: &mut Connection, run: &Run) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(RunTable::Table)
        .columns([
            RunTable::NamespaceId,
            RunTable::PipelineId,
            RunTable::PipelineConfigVersion,
            RunTable::RunId,
            RunTable::Started,
            RunTable::Ended,
            RunTable::State,
            RunTable::Status,
            RunTable::StatusReason,
            RunTable::Initiator,
            RunTable::Variables,
            RunTable::TokenId,
            RunTable::StoreObjectsExpired,
        ])
        .values_panic([
            run.namespace_id.clone().into(),
            run.pipeline_id.clone().into(),
            run.pipeline_config_version.into(),
            run.run_id.into(),
            run.started.clone().into(),
            run.ended.clone().into(),
            run.state.clone().into(),
            run.status.clone().into(),
            run.status_reason.clone().into(),
            run.initiator.clone().into(),
            run.variables.clone().into(),
            run.token_id.clone().into(),
            run.store_objects_expired.into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

/// Sorted by run_id ascending by default.
pub fn list(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
    offset: i64,
    limit: i64,
    reverse: bool,
) -> Result<Vec<Run>, StorageError> {
    let order = if reverse { Order::Desc } else { Order::Asc };

    let (sql, values) = Query::select()
        .columns([
            RunTable::NamespaceId,
            RunTable::PipelineId,
            RunTable::PipelineConfigVersion,
            RunTable::RunId,
            RunTable::Started,
            RunTable::Ended,
            RunTable::State,
            RunTable::Status,
            RunTable::StatusReason,
            RunTable::Initiator,
            RunTable::Variables,
            RunTable::TokenId,
            RunTable::StoreObjectsExpired,
        ])
        .from(RunTable::Table)
        .and_where(Expr::col(RunTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(RunTable::PipelineId).eq(pipeline_id))
        .order_by(RunTable::RunId, order)
        .limit(limit)
        .offset(offset)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<Run> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(Run::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
) -> Result<Run, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            RunTable::NamespaceId,
            RunTable::PipelineId,
            RunTable::PipelineConfigVersion,
            RunTable::RunId,
            RunTable::Started,
            RunTable::Ended,
            RunTable::State,
            RunTable::Status,
            RunTable::StatusReason,
            RunTable::Initiator,
            RunTable::Variables,
            RunTable::TokenId,
            RunTable::StoreObjectsExpired,
        ])
        .from(RunTable::Table)
        .and_where(Expr::col(RunTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(RunTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(RunTable::RunId).eq(run_id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Run::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn get_latest(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Run, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            RunTable::NamespaceId,
            RunTable::PipelineId,
            RunTable::PipelineConfigVersion,
            RunTable::RunId,
            RunTable::Started,
            RunTable::Ended,
            RunTable::State,
            RunTable::Status,
            RunTable::StatusReason,
            RunTable::Initiator,
            RunTable::Variables,
            RunTable::TokenId,
            RunTable::StoreObjectsExpired,
        ])
        .from(RunTable::Table)
        .and_where(Expr::col(RunTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(RunTable::PipelineId).eq(pipeline_id))
        .order_by(RunTable::RunId, Order::Desc)
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Run::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query
        .table(RunTable::Table)
        .and_where(Expr::col(RunTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(RunTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(RunTable::RunId).eq(run_id));

    if let Some(value) = fields.ended {
        query.value(RunTable::Ended, value.into());
    }

    if let Some(value) = fields.state {
        query.value(RunTable::State, value.into());
    }

    if let Some(value) = fields.status {
        query.value(RunTable::Status, value.into());
    }

    if let Some(value) = fields.status_reason {
        query.value(RunTable::StatusReason, value.into());
    }

    if let Some(value) = fields.variables {
        query.value(RunTable::Variables, value.into());
    }

    if let Some(value) = fields.store_objects_expired {
        query.value(RunTable::StoreObjectsExpired, value.into());
    }

    // If no fields were updated, return an error
    if query.values().is_empty() {
        return Err(StorageError::NoFieldsUpdated);
    }

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

// For the time being there is no need to delete a run and normally a run should not be deleted. But we might make
// an admin route that allows this.
#[allow(dead_code)]
pub fn delete(
    conn: &mut Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(RunTable::Table)
        .and_where(Expr::col(RunTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(RunTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(RunTable::RunId).eq(run_id))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{tests::TestHarness, Executable};

    fn setup() -> Result<(TestHarness, impl Executable), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_pipeline_id".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace)?;

        let pipeline_metadata = crate::storage::pipeline_metadata::PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::pipeline_metadata::insert(&mut conn, &pipeline_metadata)?;

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

        insert(&mut conn, &run1)?;
        insert(&mut conn, &run2)?;
        insert(&mut conn, &run3)?;

        Ok((harness, conn))
    }

    fn test_list_runs() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        // Test fetching with sorting by run_id ascending
        let runs_asc = list(&mut conn, "some_id", "some_pipeline_id", 0, 10, false)
            .expect("Failed to list runs in ascending order");

        assert_eq!(runs_asc.len(), 3, "Should return all runs");
        assert_eq!(runs_asc[0].run_id, 1, "First run should have run_id 1");
        assert_eq!(runs_asc[1].run_id, 2, "Second run should have run_id 2");
        assert_eq!(runs_asc[2].run_id, 3, "Third run should have run_id 3");

        // Test fetching with sorting by run_id descending
        let runs_desc = list(&mut conn, "some_id", "some_pipeline_id", 0, 10, true)
            .expect("Failed to list runs in descending order");

        assert_eq!(runs_desc.len(), 3, "Should return all runs");
        assert_eq!(runs_desc[0].run_id, 3, "First run should have run_id 3");
        assert_eq!(runs_desc[1].run_id, 2, "Second run should have run_id 2");
        assert_eq!(runs_desc[2].run_id, 1, "Third run should have run_id 1");

        // Test limit and offset
        let limited_runs = list(&mut conn, "some_id", "some_pipeline_id", 1, 1, false)
            .expect("Failed to list runs with limit and offset");

        assert_eq!(limited_runs.len(), 1, "Should return one run due to limit");
        assert_eq!(
            limited_runs[0].run_id, 2,
            "Should return the second run due to offset"
        );
    }

    fn test_get_latest_run() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        // Test fetching with sorting by run_id ascending
        let run =
            get_latest(&mut conn, "some_id", "some_pipeline_id").expect("Failed to get last run");

        assert_eq!(run.run_id, 3, "latest run should be 3");
    }

    fn test_get_run() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let run = get(&mut conn, "some_id", "some_pipeline_id", 1).expect("Failed to get run");

        assert_eq!(run.pipeline_id, "some_pipeline_id");
        assert_eq!(run.state, "Running");
    }

    fn test_update_run() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

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
        .expect("Failed to update run");

        let updated_run = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .expect("Failed to retrieve updated run");

        assert_eq!(updated_run.state, "Failed");
        assert_eq!(updated_run.status, "Error");
    }

    fn test_delete_run() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1).expect("Failed to delete run");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", 1).is_err(),
            "Run was not deleted"
        );
    }
}
