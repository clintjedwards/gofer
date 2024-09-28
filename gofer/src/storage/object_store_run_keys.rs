use crate::storage::{map_rusqlite_error, StorageError};
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct ObjectStoreRunKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub run_id: i64,
    pub key: String,
    pub created: String,
}

impl From<&Row<'_>> for ObjectStoreRunKey {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            run_id: row.get_unwrap("run_id"),
            key: row.get_unwrap("key"),
            created: row.get_unwrap("created"),
        }
    }
}

#[derive(Iden)]
enum ObjectStoreRunKeyTable {
    Table,
    NamespaceId,
    PipelineId,
    RunId,
    Key,
    Created,
}

pub fn insert(
    conn: &Connection,
    object_store_run_key: &ObjectStoreRunKey,
) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(ObjectStoreRunKeyTable::Table)
        .columns([
            ObjectStoreRunKeyTable::NamespaceId,
            ObjectStoreRunKeyTable::PipelineId,
            ObjectStoreRunKeyTable::RunId,
            ObjectStoreRunKeyTable::Key,
            ObjectStoreRunKeyTable::Created,
        ])
        .values_panic([
            object_store_run_key.namespace_id.clone().into(),
            object_store_run_key.pipeline_id.clone().into(),
            object_store_run_key.run_id.into(),
            object_store_run_key.key.clone().into(),
            object_store_run_key.created.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
) -> Result<Vec<ObjectStoreRunKey>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ObjectStoreRunKeyTable::NamespaceId,
            ObjectStoreRunKeyTable::PipelineId,
            ObjectStoreRunKeyTable::RunId,
            ObjectStoreRunKeyTable::Key,
            ObjectStoreRunKeyTable::Created,
        ])
        .from(ObjectStoreRunKeyTable::Table)
        .and_where(Expr::col(ObjectStoreRunKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ObjectStoreRunKeyTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(ObjectStoreRunKeyTable::RunId).eq(run_id))
        .order_by(ObjectStoreRunKeyTable::Created, Order::Asc)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<ObjectStoreRunKey> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(ObjectStoreRunKey::from(row));
    }

    Ok(objects)
}

pub fn delete(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: i64,
    key: &str,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(ObjectStoreRunKeyTable::Table)
        .and_where(Expr::col(ObjectStoreRunKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ObjectStoreRunKeyTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(ObjectStoreRunKeyTable::RunId).eq(run_id))
        .and_where(Expr::col(ObjectStoreRunKeyTable::Key).eq(key))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;

    fn setup() -> Result<(TestHarness, Connection), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_name".into(),
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

        let run = crate::storage::runs::Run {
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

        crate::storage::runs::insert(&mut conn, &run)?;

        let object_store_run_key = ObjectStoreRunKey {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            run_id: 1,
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &object_store_run_key)?;

        Ok((harness, conn))
    }

    fn test_list_object_store_run_keys() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let object_store_run_keys = list(&mut conn, "some_id", "some_pipeline_id", 1)
            .expect("Failed to list object_store_run_keys");

        // Assert that we got at least one object_store_run_key back
        assert!(
            !object_store_run_keys.is_empty(),
            "No object_store_run_keys returned"
        );

        // Assuming you want to check if the inserted object_store_run_key is in the list
        let some_object_store_run_key = object_store_run_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("ObjectStoreRunKey not found");
        assert_eq!(some_object_store_run_key.pipeline_id, "some_pipeline_id");
        assert_eq!(some_object_store_run_key.created, "some_time");
    }

    fn test_delete_object_store_run_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1, "some_id")
            .expect("Failed to delete object_store_run_key");
    }
}
