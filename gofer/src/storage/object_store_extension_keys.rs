use crate::storage::{map_rusqlite_error, StorageError};
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;
use tokio_rusqlite::{Connection, Row};

#[derive(Clone, Debug, Default)]
pub struct ObjectStoreExtensionKey {
    pub extension_id: String,
    pub key: String,
    pub created: String,
}

impl From<&Row<'_>> for ObjectStoreExtensionKey {
    fn from(row: &Row) -> Self {
        Self {
            extension_id: row.get_unwrap("extension_id"),
            key: row.get_unwrap("key"),
            created: row.get_unwrap("created"),
        }
    }
}

#[derive(Iden)]
enum ObjectStoreExtensionKeyTable {
    Table,
    ExtensionId,
    Key,
    Created,
}

pub fn insert(
    conn: &Connection,
    object_store_extension_key: &ObjectStoreExtensionKey,
) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(ObjectStoreExtensionKeyTable::Table)
        .columns([
            ObjectStoreExtensionKeyTable::ExtensionId,
            ObjectStoreExtensionKeyTable::Key,
            ObjectStoreExtensionKeyTable::Created,
        ])
        .values_panic([
            object_store_extension_key.extension_id.clone().into(),
            object_store_extension_key.key.clone().into(),
            object_store_extension_key.created.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(
    conn: &Connection,
    extension_id: &str,
) -> Result<Vec<ObjectStoreExtensionKey>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ObjectStoreExtensionKeyTable::ExtensionId,
            ObjectStoreExtensionKeyTable::Key,
            ObjectStoreExtensionKeyTable::Created,
        ])
        .from(ObjectStoreExtensionKeyTable::Table)
        .and_where(Expr::col(ObjectStoreExtensionKeyTable::ExtensionId).eq(extension_id))
        .order_by(ObjectStoreExtensionKeyTable::Created, Order::Asc)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<ObjectStoreExtensionKey> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(ObjectStoreExtensionKey::from(row));
    }

    Ok(objects)
}

pub fn delete(conn: &Connection, extension_id: &str, key: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(ObjectStoreExtensionKeyTable::Table)
        .and_where(Expr::col(ObjectStoreExtensionKeyTable::ExtensionId).eq(extension_id))
        .and_where(Expr::col(ObjectStoreExtensionKeyTable::Key).eq(key))
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

        let extension = crate::storage::extension_registrations::ExtensionRegistration {
            extension_id: "some_id".into(),
            image: "some_image".into(),
            registry_auth: "some_auth".into(),
            settings: "some_settings".into(),
            created: "some_date".into(),
            modified: "some_modified".into(),
            status: "some_status".into(),
            additional_roles: "some_role".into(),
            key_id: "some_key_id".into(),
        };

        crate::storage::extension_registrations::insert(&mut conn, &extension)?;

        let object_store_extension_key = ObjectStoreExtensionKey {
            extension_id: "some_id".into(),
            key: "some_id".into(),
            created: "some_time".into(),
        };

        insert(&mut conn, &object_store_extension_key)?;

        Ok((harness, conn))
    }

    fn test_list_object_store_extension_keys() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let object_store_extension_keys =
            list(&mut conn, "some_id").expect("Failed to list object_store_extension_keys");

        // Assert that we got at least one object_store_extension_key back
        assert!(
            !object_store_extension_keys.is_empty(),
            "No object_store_extension_keys returned"
        );

        // Assuming you want to check if the inserted object_store_extension_key is in the list
        let some_object_store_extension_key = object_store_extension_keys
            .iter()
            .find(|n| n.key == "some_id")
            .expect("ObjectStoreExtensionKey not found");
        assert_eq!(some_object_store_extension_key.created, "some_time");
    }

    fn test_delete_object_store_extension_key() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_key_id")
            .expect("Failed to delete object_store_extension_key");
    }
}
