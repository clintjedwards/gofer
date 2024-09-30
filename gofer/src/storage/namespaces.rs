use crate::storage::{epoch_milli, map_rusqlite_error, StorageError};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;
use tokio_rusqlite::{Connection, Row};

#[derive(Clone, Debug, Default)]
pub struct Namespace {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created: String,
    pub modified: String,
}

impl From<&Row<'_>> for Namespace {
    fn from(row: &Row) -> Self {
        Self {
            id: row.get_unwrap("id"),
            name: row.get_unwrap("name"),
            description: row.get_unwrap("description"),
            created: row.get_unwrap("created"),
            modified: row.get_unwrap("modified"),
        }
    }
}

#[derive(Iden)]
enum NamespaceTable {
    Table,
    Id,
    Name,
    Description,
    Created,
    Modified,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub modified: String,
}

impl Default for UpdatableFields {
    fn default() -> Self {
        Self {
            name: Default::default(),
            description: Default::default(),
            modified: epoch_milli().to_string(),
        }
    }
}

pub fn insert(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    namespace: &Namespace,
) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(NamespaceTable::Table)
        .columns([
            NamespaceTable::Id,
            NamespaceTable::Name,
            NamespaceTable::Description,
            NamespaceTable::Created,
            NamespaceTable::Modified,
        ])
        .values_panic([
            namespace.id.clone().into(),
            namespace.name.clone().into(),
            namespace.description.clone().into(),
            namespace.created.clone().into(),
            namespace.modified.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<Namespace>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            NamespaceTable::Id,
            NamespaceTable::Name,
            NamespaceTable::Description,
            NamespaceTable::Created,
            NamespaceTable::Modified,
        ])
        .from(NamespaceTable::Table)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<Namespace> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(Namespace::from(row));
    }

    Ok(objects)
}

pub fn get(conn: &Connection, id: &str) -> Result<Namespace, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            NamespaceTable::Id,
            NamespaceTable::Name,
            NamespaceTable::Description,
            NamespaceTable::Created,
            NamespaceTable::Modified,
        ])
        .from(NamespaceTable::Table)
        .and_where(Expr::col(NamespaceTable::Id).eq(id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Namespace::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(conn: &Connection, id: &str, fields: UpdatableFields) -> Result<(), StorageError> {
    let mut query = Query::update();
    query
        .table(NamespaceTable::Table)
        .values([(NamespaceTable::Modified, fields.modified.clone().into())]);

    if let Some(value) = fields.name {
        query.value(NamespaceTable::Name, value);
    }

    if let Some(value) = fields.description {
        query.value(NamespaceTable::Description, value);
    }

    query.and_where(Expr::col(NamespaceTable::Id).eq(id));
    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(NamespaceTable::Table)
        .and_where(Expr::col(NamespaceTable::Id).eq(id))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;

    fn setup() -> Result<
        (
            TestHarness,
            r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
        ),
        Box<dyn std::error::Error>,
    > {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

        let namespace = Namespace {
            id: "some_id".into(),
            name: "some_name".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        insert(&conn, &namespace)?;

        Ok((harness, conn))
    }

    fn test_list_namespaces() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let namespaces = list(&mut conn).expect("Failed to list namespaces");

        // Assert that we got at least one namespace back
        assert!(!namespaces.is_empty(), "No namespaces returned");

        // Assuming you want to check if the inserted namespace is in the list
        let some_namespace = namespaces
            .iter()
            .find(|n| n.id == "some_id")
            .expect("Namespace not found");
        assert_eq!(some_namespace.name, "some_name");
        assert_eq!(some_namespace.description, "some_description");
    }

    fn test_insert_namespace() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let new_namespace = Namespace {
            id: "new_id".into(),
            name: "new_name".into(),
            description: "new_description".into(),
            created: "some_time".into(),
            modified: "some_other_time".into(),
        };

        insert(&conn, &new_namespace).expect("Failed to insert namespace");

        let retrieved_namespace = get(&mut conn, "new_id").expect("Failed to retrieve namespace");

        assert_eq!(retrieved_namespace.id, "new_id");
        assert_eq!(retrieved_namespace.name, "new_name");
        assert_eq!(retrieved_namespace.description, "new_description");
    }

    fn test_get_namespace() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let namespace = get(&mut conn, "some_id").expect("Failed to get namespace");

        assert_eq!(namespace.id, "some_id");
        assert_eq!(namespace.name, "some_name");

        assert!(
            get(&mut conn, "non_existent").is_err(),
            "Unexpectedly found a namespace"
        );
    }

    fn test_update_namespace() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            name: Some("updated_name".into()),
            description: Some("updated_description".into()),
            modified: "updated_time".into(),
        };

        update(&mut conn, "some_id", fields_to_update).expect("Failed to update namespace");

        let updated_namespace =
            get(&mut conn, "some_id").expect("Failed to retrieve updated namespace");

        assert_eq!(updated_namespace.name, "updated_name");
        assert_eq!(updated_namespace.description, "updated_description");
    }

    fn test_delete_namespace() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id").expect("Failed to delete namespace");

        assert!(
            get(&mut conn, "some_id").is_err(),
            "Namespace was not deleted"
        );
    }
}
