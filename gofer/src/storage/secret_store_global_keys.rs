use crate::storage::{map_rusqlite_error, Executable, StorageError};
use rusqlite::Row;
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct SecretStoreGlobalKey {
    pub key: String,
    pub namespaces: String,
    pub created: String,
}

impl From<&Row<'_>> for SecretStoreGlobalKey {
    fn from(row: &Row) -> Self {
        Self {
            key: row.get_unwrap("key"),
            namespaces: row.get_unwrap("namespaces"),
            created: row.get_unwrap("created"),
        }
    }
}

#[derive(Iden)]
enum SecretStoreGlobalKeyTable {
    Table,
    Key,
    Namespaces,
    Created,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub namespaces: Option<String>,
}

pub fn insert(
    conn: &dyn Executable,
    secret_store_global_key: &SecretStoreGlobalKey,
) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(SecretStoreGlobalKeyTable::Table)
        .columns([
            SecretStoreGlobalKeyTable::Key,
            SecretStoreGlobalKeyTable::Namespaces,
            SecretStoreGlobalKeyTable::Created,
        ])
        .values_panic([
            secret_store_global_key.key.clone().into(),
            secret_store_global_key.namespaces.clone().into(),
            secret_store_global_key.created.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(conn: &dyn Executable) -> Result<Vec<SecretStoreGlobalKey>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            SecretStoreGlobalKeyTable::Key,
            SecretStoreGlobalKeyTable::Namespaces,
            SecretStoreGlobalKeyTable::Created,
        ])
        .from(SecretStoreGlobalKeyTable::Table)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<SecretStoreGlobalKey> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(SecretStoreGlobalKey::from(row));
    }

    Ok(objects)
}

pub fn get(conn: &dyn Executable, key: &str) -> Result<SecretStoreGlobalKey, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            SecretStoreGlobalKeyTable::Key,
            SecretStoreGlobalKeyTable::Namespaces,
            SecretStoreGlobalKeyTable::Created,
        ])
        .from(SecretStoreGlobalKeyTable::Table)
        .and_where(Expr::col(SecretStoreGlobalKeyTable::Key).eq(key))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(SecretStoreGlobalKey::from(row));
    }

    Err(StorageError::NotFound)
}

// For now we don't allow users to update the namespaces for their global key, but we keep this around because one day
// we might.
#[allow(dead_code)]
pub fn update(
    conn: &dyn Executable,
    key: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(SecretStoreGlobalKeyTable::Table);

    if let Some(value) = fields.namespaces {
        query.value(SecretStoreGlobalKeyTable::Namespaces, value.into());
    }

    if query.is_empty_values() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query.and_where(Expr::col(SecretStoreGlobalKeyTable::Key).eq(key));

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(conn: &dyn Executable, key: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(SecretStoreGlobalKeyTable::Table)
        .and_where(Expr::col(SecretStoreGlobalKeyTable::Key).eq(key))
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

        let secret_store_global_key = SecretStoreGlobalKey {
            key: "some_id".into(),
            namespaces: "some_name".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &secret_store_global_key)?;

        Ok((harness, conn))
    }

    fn test_list_secret_store_global_keys() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let secret_store_global_keys =
            list(&mut conn).expect("Failed to list secret_store_global_keys");

        // Assert that we got at least one secret_store_global_key back
        assert!(
            !secret_store_global_keys.is_empty(),
            "No secret_store_global_keys returned"
        );

        // Assuming you want to check if the inserted secret_store_global_key is in the list
        let some_secret_store_global_key = secret_store_global_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("SecretStoreGlobalKey not found");
        assert_eq!(some_secret_store_global_key.namespaces, "some_name");
        assert_eq!(some_secret_store_global_key.created, "some_time");
    }

    fn test_get_secret_store_global_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let secret_store_global_key =
            get(&mut conn, "some_id").expect("Failed to get secret_store_global_key");

        assert_eq!(secret_store_global_key.key, "some_id");
        assert_eq!(secret_store_global_key.created, "some_time");

        assert!(
            get(&mut conn, "non_existent").is_err(),
            "Unexpectedly found a secret_store_global_key"
        );
    }

    fn test_update_secret_store_global_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            namespaces: Some("some_namespace".into()),
        };

        update(&mut conn, "some_id", fields_to_update)
            .expect("Failed to update secret_store_global_key");

        let updated_secret_store_global_key =
            get(&mut conn, "some_id").expect("Failed to retrieve updated secret_store_global_key");

        assert_eq!(updated_secret_store_global_key.namespaces, "some_namespace");
        assert_eq!(updated_secret_store_global_key.created, "some_time");
    }

    fn test_delete_secret_store_global_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id").expect("Failed to delete secret_store_global_key");

        assert!(
            get(&mut conn, "some_id").is_err(),
            "SecretStoreGlobalKey was not deleted"
        );
    }
}
