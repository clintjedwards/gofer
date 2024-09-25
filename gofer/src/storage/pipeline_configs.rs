use crate::storage::{map_rusqlite_error, Executable, StorageError};
use futures::TryFutureExt;
use rusqlite::Row;
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct PipelineConfig {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub version: i64,
    pub parallelism: i64,
    pub name: String,
    pub description: String,
    pub registered: String,
    pub deprecated: String,
    pub state: String,
}

impl From<&Row<'_>> for PipelineConfig {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            version: row.get_unwrap("version"),
            parallelism: row.get_unwrap("parallelism"),
            name: row.get_unwrap("name"),
            description: row.get_unwrap("description"),
            registered: row.get_unwrap("registered"),
            deprecated: row.get_unwrap("deprecated"),
            state: row.get_unwrap("state"),
        }
    }
}

#[derive(Iden)]
enum PipelineConfigTable {
    Table,
    NamespaceId,
    PipelineId,
    Version,
    Parallelism,
    Name,
    Description,
    Registered,
    Deprecated,
    State,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub deprecated: Option<String>,
    pub state: Option<String>,
}

pub fn insert(conn: &dyn Executable, pipeline_config: &PipelineConfig) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(PipelineConfigTable::Table)
        .columns([
            PipelineConfigTable::NamespaceId,
            PipelineConfigTable::PipelineId,
            PipelineConfigTable::Version,
            PipelineConfigTable::Parallelism,
            PipelineConfigTable::Name,
            PipelineConfigTable::Description,
            PipelineConfigTable::Registered,
            PipelineConfigTable::Deprecated,
            PipelineConfigTable::State,
        ])
        .values_panic([
            pipeline_config.namespace_id.clone().into(),
            pipeline_config.pipeline_id.clone().into(),
            pipeline_config.version.into(),
            pipeline_config.parallelism.into(),
            pipeline_config.name.clone().into(),
            pipeline_config.description.clone().into(),
            pipeline_config.registered.clone().into(),
            pipeline_config.deprecated.clone().into(),
            pipeline_config.state.clone().into(),
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
) -> Result<Vec<PipelineConfig>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            PipelineConfigTable::NamespaceId,
            PipelineConfigTable::PipelineId,
            PipelineConfigTable::Version,
            PipelineConfigTable::Parallelism,
            PipelineConfigTable::Name,
            PipelineConfigTable::Description,
            PipelineConfigTable::Registered,
            PipelineConfigTable::Deprecated,
            PipelineConfigTable::State,
        ])
        .from(PipelineConfigTable::Table)
        .and_where(Expr::col(PipelineConfigTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineConfigTable::PipelineId).eq(pipeline_id))
        .order_by(PipelineConfigTable::Version, Order::Desc)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<PipelineConfig> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(PipelineConfig::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
) -> Result<PipelineConfig, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            PipelineConfigTable::NamespaceId,
            PipelineConfigTable::PipelineId,
            PipelineConfigTable::Version,
            PipelineConfigTable::Parallelism,
            PipelineConfigTable::Name,
            PipelineConfigTable::Description,
            PipelineConfigTable::Registered,
            PipelineConfigTable::Deprecated,
            PipelineConfigTable::State,
        ])
        .from(PipelineConfigTable::Table)
        .and_where(Expr::col(PipelineConfigTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineConfigTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(PipelineConfigTable::Version).eq(version))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(PipelineConfig::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn get_latest(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<PipelineConfig, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            PipelineConfigTable::NamespaceId,
            PipelineConfigTable::PipelineId,
            PipelineConfigTable::Version,
            PipelineConfigTable::Parallelism,
            PipelineConfigTable::Name,
            PipelineConfigTable::Description,
            PipelineConfigTable::Registered,
            PipelineConfigTable::Deprecated,
            PipelineConfigTable::State,
        ])
        .from(PipelineConfigTable::Table)
        .and_where(Expr::col(PipelineConfigTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineConfigTable::PipelineId).eq(pipeline_id))
        .order_by(PipelineConfigTable::Version, Order::Desc)
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(PipelineConfig::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn get_latest_w_state(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    state: &str,
) -> Result<PipelineConfig, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            PipelineConfigTable::NamespaceId,
            PipelineConfigTable::PipelineId,
            PipelineConfigTable::Version,
            PipelineConfigTable::Parallelism,
            PipelineConfigTable::Name,
            PipelineConfigTable::Description,
            PipelineConfigTable::Registered,
            PipelineConfigTable::Deprecated,
            PipelineConfigTable::State,
        ])
        .from(PipelineConfigTable::Table)
        .and_where(Expr::col(PipelineConfigTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineConfigTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(PipelineConfigTable::State).eq(state))
        .order_by(PipelineConfigTable::Version, Order::Desc)
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(PipelineConfig::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(PipelineConfigTable::Table);

    if let Some(value) = fields.deprecated {
        query.value(PipelineConfigTable::Deprecated, value.into());
    }

    if let Some(value) = fields.state {
        query.value(PipelineConfigTable::State, value.into());
    }

    if query.is_empty_values() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query
        .and_where(Expr::col(PipelineConfigTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineConfigTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(PipelineConfigTable::Version).eq(version));

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    version: i64,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(PipelineConfigTable::Table)
        .and_where(Expr::col(PipelineConfigTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(PipelineConfigTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(PipelineConfigTable::Version).eq(version))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        pipeline_configs::PipelineConfig,
        pipeline_metadata::{self, PipelineMetadata},
        tests::TestHarness,
        Executable,
    };

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

        let pipeline_metadata = PipelineMetadata {
            namespace_id: "some_id".into(),
            pipeline_id: "some_pipeline_id".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        pipeline_metadata::insert(&mut conn, &pipeline_metadata)?;

        let new_pipeline_config = PipelineConfig {
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

        insert(&mut conn, &new_pipeline_config).expect("Failed to insert pipeline_config");

        Ok((harness, conn))
    }

    fn test_list_pipeline_configs() -> Result<(), Box<dyn std::error::Error>> {
        let (_harness, mut conn) = setup()?;

        let pipeline_configs = list(&mut conn, "some_id", "some_pipeline_id")
            .expect("Failed to list pipeline_configs");

        assert!(!pipeline_configs.is_empty(), "No pipeline_configs returned");

        let some_pipeline_config = pipeline_configs
            .iter()
            .find(|n| n.namespace_id == "some_id" && n.pipeline_id == "some_pipeline_id")
            .expect("PipelineConfig not found");
        assert_eq!(some_pipeline_config.state, "active");

        Ok(())
    }

    fn test_update_pipeline_config() -> Result<(), Box<dyn std::error::Error>> {
        let (_harness, mut conn) = setup()?;

        let fields_to_update = UpdatableFields {
            deprecated: Some("2024-01-01".to_string()),
            state: Some("deprecated".to_string()),
        };

        update(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            1,
            fields_to_update,
        )
        .expect("Failed to update pipeline_config");

        let updated_pipeline_config = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .expect("Failed to retrieve updated pipeline_config");

        assert_eq!(updated_pipeline_config.state, "deprecated");
        assert_eq!(updated_pipeline_config.deprecated, "2024-01-01");

        Ok(())
    }

    fn test_delete_pipeline_config() -> Result<(), Box<dyn std::error::Error>> {
        let (_harness, mut conn) = setup()?;

        delete(&mut conn, "some_id", "some_pipeline_id", 1)
            .expect("Failed to delete pipeline_config");

        let result = get(&mut conn, "some_id", "some_pipeline_id", 1);
        assert!(result.is_err(), "PipelineConfig was not deleted");

        Ok(())
    }
}
