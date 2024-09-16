use crate::storage::{epoch_milli, map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
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

pub async fn insert(
    conn: &mut SqliteConnection,
    registration: &ExtensionRegistration,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO extension_registrations (extension_id, image, registry_auth, settings, created, modified, \
        status, key_id, additional_roles) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);"
    )
    .bind(&registration.extension_id)
    .bind(&registration.image)
    .bind(&registration.registry_auth)
    .bind(&registration.settings)
    .bind(&registration.created)
    .bind(&registration.modified)
    .bind(&registration.status)
    .bind(&registration.key_id)
    .bind(&registration.additional_roles);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list(conn: &mut SqliteConnection) -> Result<Vec<ExtensionRegistration>, StorageError> {
    let query = sqlx::query_as::<_, ExtensionRegistration>(
        "SELECT extension_id, image, registry_auth, settings, \
        created, modified, status, key_id, additional_roles FROM extension_registrations;",
    );

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    extension_id: &str,
) -> Result<ExtensionRegistration, StorageError> {
    let query = sqlx::query_as::<_, ExtensionRegistration>(
        "SELECT extension_id, image, registry_auth, settings, created, modified, status, key_id, additional_roles \
        FROM extension_registrations WHERE extension_id = ?;",
    )
    .bind(extension_id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update(
    conn: &mut SqliteConnection,
    extension_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE extension_registrations SET "#);
    let mut updated_fields_total = 0;

    if let Some(value) = &fields.image {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("image = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.registry_auth {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("registry_auth = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.settings {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("settings = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.status {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.key_id {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("key_id = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    if let Some(value) = &fields.additional_roles {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("additional_roles = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(", ");
    update_query.push("modified = ");
    update_query.push_bind(fields.modified);

    update_query.push(" WHERE extension_id = ");
    update_query.push_bind(extension_id);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

pub async fn delete(conn: &mut SqliteConnection, extension_id: &str) -> Result<(), StorageError> {
    let query = sqlx::query("DELETE FROM extension_registrations WHERE extension_id = ?;")
        .bind(extension_id);

    let sql = query.sql();

    query
        .execute(conn)
        .map_ok(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TestHarness;
    use sqlx::{pool::PoolConnection, Sqlite};

    async fn setup() -> Result<(TestHarness, PoolConnection<Sqlite>), Box<dyn std::error::Error>> {
        let harness = TestHarness::new().await;
        let mut conn = harness.conn().await.unwrap();

        let namespace = crate::storage::namespaces::Namespace {
            id: "some_id".into(),
            name: "some_name".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace).await?;

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

        insert(&mut conn, &registration).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_extension_registrations() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let extension_registrations = list(&mut conn)
            .await
            .expect("Failed to list extension_registrations");

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

    #[tokio::test]
    async fn test_get_extension_registration() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let extension_registration = get(&mut conn, "ext123")
            .await
            .expect("Failed to get extension_registration");

        assert_eq!(extension_registration.settings, "var1=value1,var2=value2");
        assert_eq!(extension_registration.status, "Active");
    }

    #[tokio::test]
    async fn test_update_extension_registration() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields = UpdatableFields {
            image: Some("new_image.png".to_string()),
            registry_auth: Some("new_auth".to_string()),
            settings: Some("new_settings".to_string()),
            status: Some("Active".to_string()),
            key_id: Some("12345".to_string()),
            additional_roles: Some("some_other_role_here".to_string()),
            modified: "".to_string(),
        };

        update(&mut conn, "ext123", fields).await.unwrap();

        let updated = get(&mut conn, "ext123")
            .await
            .expect("Failed to retrieve updated namespace");

        assert_eq!(updated.image, "new_image.png".to_string());
        assert_eq!(updated.status, "Active".to_string());
    }

    #[tokio::test]
    async fn test_delete_extension_registration() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "ext123")
            .await
            .expect("Failed to delete extension_registration");

        assert!(
            get(&mut conn, "ext123").await.is_err(),
            "ExtensionRegistration was not deleted"
        );
    }
}
