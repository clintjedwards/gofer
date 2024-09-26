use crate::storage::{map_rusqlite_error, Executable, StorageError};
use rusqlite::Row;
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct SecretStorePipelineKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub key: String,
    pub created: String,
}

impl From<&Row<'_>> for SecretStorePipelineKey {
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
enum SecretStorePipelineKeyTable {
    Table,
    NamespaceId,
    PipelineId,
    Key,
    Created,
}

pub fn insert(
    conn: &dyn Executable,
    secret_store_pipeline_key: &SecretStorePipelineKey,
) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(SecretStorePipelineKeyTable::Table)
        .columns([
            SecretStorePipelineKeyTable::NamespaceId,
            SecretStorePipelineKeyTable::PipelineId,
            SecretStorePipelineKeyTable::Key,
            SecretStorePipelineKeyTable::Created,
        ])
        .values_panic([
            secret_store_pipeline_key.namespace_id.clone().into(),
            secret_store_pipeline_key.pipeline_id.clone().into(),
            secret_store_pipeline_key.key.clone().into(),
            secret_store_pipeline_key.created.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<SecretStorePipelineKey>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            SecretStorePipelineKeyTable::NamespaceId,
            SecretStorePipelineKeyTable::PipelineId,
            SecretStorePipelineKeyTable::Key,
            SecretStorePipelineKeyTable::Created,
        ])
        .from(SecretStorePipelineKeyTable::Table)
        .and_where(Expr::col(SecretStorePipelineKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(SecretStorePipelineKeyTable::PipelineId).eq(pipeline_id))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<SecretStorePipelineKey> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(SecretStorePipelineKey::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    key: &str,
) -> Result<SecretStorePipelineKey, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            SecretStorePipelineKeyTable::NamespaceId,
            SecretStorePipelineKeyTable::PipelineId,
            SecretStorePipelineKeyTable::Key,
            SecretStorePipelineKeyTable::Created,
        ])
        .from(SecretStorePipelineKeyTable::Table)
        .and_where(Expr::col(SecretStorePipelineKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(SecretStorePipelineKeyTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(SecretStorePipelineKeyTable::Key).eq(key))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(SecretStorePipelineKey::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn delete(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    key: &str,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(SecretStorePipelineKeyTable::Table)
        .and_where(Expr::col(SecretStorePipelineKeyTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(SecretStorePipelineKeyTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(SecretStorePipelineKeyTable::Key).eq(key))
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

        let secret_store_pipeline_key = SecretStorePipelineKey {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &secret_store_pipeline_key)?;

        Ok((harness, conn))
    }

    fn test_list_secret_store_pipeline_keys() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let secret_store_pipeline_keys = list(&mut conn, "some_id", "some_pipeline_id")
            .expect("Failed to list secret_store_pipeline_keys");

        // Assert that we got at least one secret_store_pipeline_key back
        assert!(
            !secret_store_pipeline_keys.is_empty(),
            "No secret_store_pipeline_keys returned"
        );

        // Assuming you want to check if the inserted secret_store_pipeline_key is in the list
        let some_secret_store_pipeline_key = secret_store_pipeline_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("SecretStorePipelineKey not found");
        assert_eq!(
            some_secret_store_pipeline_key.pipeline_id,
            "some_pipeline_id"
        );
        assert_eq!(some_secret_store_pipeline_key.created, "some_time");
    }

    fn test_get_secret_store_pipeline_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let secret_store_pipeline_key = get(&mut conn, "some_id", "some_pipeline_id", "some_id")
            .expect("Failed to get secret_store_pipeline_key");

        assert_eq!(secret_store_pipeline_key.key, "some_id");
        assert_eq!(secret_store_pipeline_key.created, "some_time");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", "non_existent").is_err(),
            "Unexpectedly found a secret_store_pipeline_key"
        );
    }

    fn test_delete_secret_store_pipeline_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", "some_id")
            .expect("Failed to delete secret_store_pipeline_key");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", "some_id").is_err(),
            "SecretStorePipelineKey was not deleted"
        );
    }
}
