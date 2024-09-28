use crate::storage::{map_rusqlite_error, StorageError};
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct ObjectStorePipelineKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub key: String,
    pub created: String,
}

impl From<&Row<'_>> for ObjectStorePipelineKey {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            key: row.get_unwrap("key"),
            created: row.get_unwrap("created"),
        }
    }
}

#[derive(Iden)]
enum ObjectStorePipelineKeyTable {
    Table,
    NamespaceId,
    PipelineId,
    Key,
    Created,
}

pub fn insert(
    conn: &Connection,
    object_store_pipeline_key: &ObjectStorePipelineKey,
) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(ObjectStorePipelineKeyTable::Table)
        .columns([
            ObjectStorePipelineKeyTable::NamespaceId,
            ObjectStorePipelineKeyTable::PipelineId,
            ObjectStorePipelineKeyTable::Key,
            ObjectStorePipelineKeyTable::Created,
        ])
        .values_panic([
            object_store_pipeline_key.namespace_id.clone().into(),
            object_store_pipeline_key.pipeline_id.clone().into(),
            object_store_pipeline_key.key.clone().into(),
            object_store_pipeline_key.created.clone().into(),
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
) -> Result<Vec<ObjectStorePipelineKey>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ObjectStorePipelineKeyTable::NamespaceId,
            ObjectStorePipelineKeyTable::PipelineId,
            ObjectStorePipelineKeyTable::Key,
            ObjectStorePipelineKeyTable::Created,
        ])
        .from(ObjectStorePipelineKeyTable::Table)
        .and_where(Expr::col(ObjectStorePipelineKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ObjectStorePipelineKeyTable::PipelineId).eq(pipeline_id))
        .order_by(ObjectStorePipelineKeyTable::Created, Order::Asc)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<ObjectStorePipelineKey> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(ObjectStorePipelineKey::from(row));
    }

    Ok(objects)
}

pub fn delete(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    key: &str,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(ObjectStorePipelineKeyTable::Table)
        .and_where(Expr::col(ObjectStorePipelineKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ObjectStorePipelineKeyTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(ObjectStorePipelineKeyTable::Key).eq(key))
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

        let object_store_pipeline_key = ObjectStorePipelineKey {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &object_store_pipeline_key)?;

        Ok((harness, conn))
    }

    fn test_list_object_store_pipeline_keys() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let object_store_pipeline_keys = list(&mut conn, "some_id", "some_pipeline_id")
            .expect("Failed to list object_store_pipeline_keys");

        // Assert that we got at least one object_store_pipeline_key back
        assert!(
            !object_store_pipeline_keys.is_empty(),
            "No object_store_pipeline_keys returned"
        );

        // Assuming you want to check if the inserted object_store_pipeline_key is in the list
        let some_object_store_pipeline_key = object_store_pipeline_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("ObjectStorePipelineKey not found");
        assert_eq!(
            some_object_store_pipeline_key.pipeline_id,
            "some_pipeline_id"
        );
        assert_eq!(some_object_store_pipeline_key.created, "some_time");
    }

    fn test_delete_object_store_pipeline_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", "some_id")
            .expect("Failed to delete object_store_pipeline_key");
    }
}
