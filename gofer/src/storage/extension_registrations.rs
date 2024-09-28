use crate::storage::{epoch_milli, map_rusqlite_error, StorageError};
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct ExtensionRegistration {
    pub extension_id: String,
    pub image: String,
    pub registry_auth: String,
    pub settings: String,
    pub created: String,
    pub modified: String,
    pub status: String,
    pub key_id: String,
    pub additional_roles: String,
}

impl From<&Row<'_>> for ExtensionRegistration {
    fn from(row: &Row) -> Self {
        Self {
            extension_id: row.get_unwrap("extension_id"),
            image: row.get_unwrap("image"),
            registry_auth: row.get_unwrap("registry_auth"),
            settings: row.get_unwrap("settings"),
            created: row.get_unwrap("created"),
            modified: row.get_unwrap("modified"),
            status: row.get_unwrap("status"),
            key_id: row.get_unwrap("key_id"),
            additional_roles: row.get_unwrap("additional_roles"),
        }
    }
}

#[derive(Iden)]
enum ExtensionRegistrationTable {
    Table,
    ExtensionId,
    Image,
    RegistryAuth,
    Settings,
    Created,
    Modified,
    Status,
    KeyId,
    AdditionalRoles,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub image: Option<String>,
    pub registry_auth: Option<String>,
    pub settings: Option<String>,
    pub status: Option<String>,
    pub key_id: Option<String>,
    pub additional_roles: Option<String>,
    pub modified: String,
}

impl Default for UpdatableFields {
    fn default() -> Self {
        Self {
            image: Default::default(),
            registry_auth: Default::default(),
            settings: Default::default(),
            status: Default::default(),
            key_id: Default::default(),
            additional_roles: Default::default(),
            modified: epoch_milli().to_string(),
        }
    }
}

pub fn insert(conn: &Connection, registration: &ExtensionRegistration) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(ExtensionRegistrationTable::Table)
        .columns([
            ExtensionRegistrationTable::ExtensionId,
            ExtensionRegistrationTable::Image,
            ExtensionRegistrationTable::RegistryAuth,
            ExtensionRegistrationTable::Settings,
            ExtensionRegistrationTable::Created,
            ExtensionRegistrationTable::Modified,
            ExtensionRegistrationTable::Status,
            ExtensionRegistrationTable::KeyId,
            ExtensionRegistrationTable::AdditionalRoles,
        ])
        .values_panic([
            registration.extension_id.clone().into(),
            registration.image.clone().into(),
            registration.registry_auth.clone().into(),
            registration.settings.clone().into(),
            registration.created.clone().into(),
            registration.modified.clone().into(),
            registration.status.clone().into(),
            registration.key_id.clone().into(),
            registration.additional_roles.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<ExtensionRegistration>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ExtensionRegistrationTable::ExtensionId,
            ExtensionRegistrationTable::Image,
            ExtensionRegistrationTable::RegistryAuth,
            ExtensionRegistrationTable::Settings,
            ExtensionRegistrationTable::Created,
            ExtensionRegistrationTable::Modified,
            ExtensionRegistrationTable::Status,
            ExtensionRegistrationTable::KeyId,
            ExtensionRegistrationTable::AdditionalRoles,
        ])
        .from(ExtensionRegistrationTable::Table)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<ExtensionRegistration> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(ExtensionRegistration::from(row));
    }

    Ok(objects)
}

pub fn get(conn: &Connection, extension_id: &str) -> Result<ExtensionRegistration, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ExtensionRegistrationTable::ExtensionId,
            ExtensionRegistrationTable::Image,
            ExtensionRegistrationTable::RegistryAuth,
            ExtensionRegistrationTable::Settings,
            ExtensionRegistrationTable::Created,
            ExtensionRegistrationTable::Modified,
            ExtensionRegistrationTable::Status,
            ExtensionRegistrationTable::KeyId,
            ExtensionRegistrationTable::AdditionalRoles,
        ])
        .from(ExtensionRegistrationTable::Table)
        .and_where(Expr::col(ExtensionRegistrationTable::ExtensionId).eq(extension_id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(ExtensionRegistration::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &Connection,
    extension_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(ExtensionRegistrationTable::Table);

    if let Some(value) = fields.image {
        query.value(ExtensionRegistrationTable::Image, value);
    }

    if let Some(value) = fields.registry_auth {
        query.value(ExtensionRegistrationTable::RegistryAuth, value);
    }

    if let Some(value) = fields.settings {
        query.value(ExtensionRegistrationTable::Settings, value);
    }

    if let Some(value) = fields.status {
        query.value(ExtensionRegistrationTable::Status, value);
    }

    if let Some(value) = fields.key_id {
        query.value(ExtensionRegistrationTable::KeyId, value);
    }

    if let Some(value) = fields.additional_roles {
        query.value(ExtensionRegistrationTable::AdditionalRoles, value);
    }

    if query.get_values().is_empty() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query.value(ExtensionRegistrationTable::Modified, fields.modified);

    query.and_where(Expr::col(ExtensionRegistrationTable::ExtensionId).eq(extension_id));

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(conn: &Connection, extension_id: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(ExtensionRegistrationTable::Table)
        .and_where(Expr::col(ExtensionRegistrationTable::ExtensionId).eq(extension_id))
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

        let registration = ExtensionRegistration {
            extension_id: "ext123".to_string(),
            image: "http://example.com/image.png".to_string(),
            registry_auth: "auth_token".to_string(),
            settings: "var1=value1,var2=value2".to_string(),
            created: "2023-04-15T12:34:56".to_string(),
            modified: String::new(),
            status: "Active".to_string(),
            additional_roles: "[some_role_here]".to_string(),
            key_id: "key456".to_string(),
        };

        insert(&mut conn, &registration)?;

        Ok((harness, conn))
    }

    fn test_list_extension_registrations() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let extension_registrations =
            list(&mut conn).expect("Failed to list extension_registrations");

        // Assert that we got at least one extension_registration back
        assert!(
            !extension_registrations.is_empty(),
            "No extension_registrations returned"
        );

        // Assuming you want to check if the inserted extension_registration is in the list
        let some_extension_registration = extension_registrations
            .iter()
            .find(|n| n.extension_id == "ext123")
            .expect("ExtensionRegistration not found");
        assert_eq!(
            some_extension_registration.settings,
            "var1=value1,var2=value2"
        );
        assert_eq!(some_extension_registration.status, "Active");
    }

    fn test_get_extension_registration() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let extension_registration =
            get(&mut conn, "ext123").expect("Failed to get extension_registration");

        assert_eq!(extension_registration.settings, "var1=value1,var2=value2");
        assert_eq!(extension_registration.status, "Active");
    }

    fn test_update_extension_registration() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields = UpdatableFields {
            image: Some("new_image.png".to_string()),
            registry_auth: Some("new_auth".to_string()),
            settings: Some("new_settings".to_string()),
            status: Some("Active".to_string()),
            key_id: Some("12345".to_string()),
            additional_roles: Some("some_other_role_here".to_string()),
            modified: "".to_string(),
        };

        update(&mut conn, "ext123", fields).unwrap();

        let updated = get(&mut conn, "ext123").expect("Failed to retrieve updated namespace");

        assert_eq!(updated.image, "new_image.png".to_string());
        assert_eq!(updated.status, "Active".to_string());
    }

    fn test_delete_extension_registration() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "ext123").expect("Failed to delete extension_registration");

        assert!(
            get(&mut conn, "ext123").is_err(),
            "ExtensionRegistration was not deleted"
        );
    }
}
