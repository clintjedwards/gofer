use crate::storage::{epoch_milli, map_rusqlite_error, StorageError};
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct PipelineMetadata {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub state: String,
    pub created: String,
    pub modified: String,
}

impl From<&Row<'_>> for PipelineMetadata {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            state: row.get_unwrap("state"),
            created: row.get_unwrap("created"),
            modified: row.get_unwrap("modified"),
        }
    }
}

#[derive(Iden)]
enum PipelineMetadataTable {
    Table,
    NamespaceId,
    PipelineId,
    State,
    Created,
    Modified,
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

pub fn insert(conn: &Connection, pipeline_metadata: &PipelineMetadata) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(PipelineMetadataTable::Table)
        .columns([
            PipelineMetadataTable::NamespaceId,
            PipelineMetadataTable::PipelineId,
            PipelineMetadataTable::State,
            PipelineMetadataTable::Created,
            PipelineMetadataTable::Modified,
        ])
        .values_panic([
            pipeline_metadata.namespace_id.clone().into(),
            pipeline_metadata.pipeline_id.clone().into(),
            pipeline_metadata.state.clone().into(),
            pipeline_metadata.created.clone().into(),
            pipeline_metadata.modified.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(conn: &Connection, namespace_id: &str) -> Result<Vec<PipelineMetadata>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            PipelineMetadataTable::NamespaceId,
            PipelineMetadataTable::PipelineId,
            PipelineMetadataTable::State,
            PipelineMetadataTable::Created,
            PipelineMetadataTable::Modified,
        ])
        .from(PipelineMetadataTable::Table)
        .and_where(Expr::col(PipelineMetadataTable::NamespaceId).eq(namespace_id))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<PipelineMetadata> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(PipelineMetadata::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<PipelineMetadata, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            PipelineMetadataTable::NamespaceId,
            PipelineMetadataTable::PipelineId,
            PipelineMetadataTable::State,
            PipelineMetadataTable::Created,
            PipelineMetadataTable::Modified,
        ])
        .from(PipelineMetadataTable::Table)
        .and_where(Expr::col(PipelineMetadataTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineMetadataTable::PipelineId).eq(pipeline_id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(PipelineMetadata::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(PipelineMetadataTable::Table);

    if let Some(value) = fields.state {
        query.value(PipelineMetadataTable::State, value.into());
    }

    query.value(PipelineMetadataTable::Modified, fields.modified.into());

    if query.is_empty_values() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query
        .and_where(Expr::col(PipelineMetadataTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineMetadataTable::PipelineId).eq(pipeline_id));

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(PipelineMetadataTable::Table)
        .and_where(Expr::col(PipelineMetadataTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineMetadataTable::PipelineId).eq(pipeline_id))
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

        let pipeline_metadata = PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        insert(&mut conn, &pipeline_metadata)?;

        Ok((harness, conn))
    }

    fn test_list_pipeline_metadatas() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let pipeline_metadatas =
            list(&mut conn, "some_id").expect("Failed to list pipeline_metadatas");

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

    fn test_insert_pipeline_metadata() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let new_pipeline_metadata = PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "new_pipeline_id".into(),
            state: "new_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        insert(&mut conn, &new_pipeline_metadata).expect("Failed to insert pipeline_metadata");

        let retrieved_pipeline_metadata = get(&mut conn, "some_id", "new_pipeline_id")
            .expect("Failed to retrieve pipeline_metadata");

        assert_eq!(retrieved_pipeline_metadata.namespace_id, "some_id");
        assert_eq!(retrieved_pipeline_metadata.pipeline_id, "new_pipeline_id");
        assert_eq!(retrieved_pipeline_metadata.state, "new_state");
    }

    fn test_get_pipeline_metadata() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let pipeline_metadata =
            get(&mut conn, "some_id", "some_pipeline_id").expect("Failed to get pipeline_metadata");

        assert_eq!(pipeline_metadata.namespace_id, "some_id");
        assert_eq!(pipeline_metadata.pipeline_id, "some_pipeline_id");
    }

    fn test_update_pipeline_metadata() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            state: Some("updated_state".into()),
            modified: "updated_time".into(),
        };

        update(&mut conn, "some_id", "some_pipeline_id", fields_to_update)
            .expect("Failed to update pipeline_metadata");

        let updated_pipeline_metadata = get(&mut conn, "some_id", "some_pipeline_id")
            .expect("Failed to retrieve updated pipeline_metadata");

        assert_eq!(updated_pipeline_metadata.state, "updated_state");
    }

    fn test_delete_pipeline_metadata() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id")
            .expect("Failed to delete pipeline_metadata");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id").is_err(),
            "PipelineMetadata was not deleted"
        );
    }
}
