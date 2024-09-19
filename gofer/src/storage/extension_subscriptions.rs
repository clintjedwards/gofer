use crate::storage::{map_sqlx_error, StorageError};
use futures::TryFutureExt;
use sqlx::{Execute, FromRow, QueryBuilder, Sqlite, SqliteConnection};

#[derive(Clone, Debug, Default, FromRow)]
pub struct ExtensionSubscription {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub extension_id: String,
    pub extension_subscription_id: String,
    pub settings: String,
    pub status: String,
    pub status_reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub settings: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
}

pub async fn insert(
    conn: &mut SqliteConnection,
    subscription: &ExtensionSubscription,
) -> Result<(), StorageError> {
    let query = sqlx::query(
        "INSERT INTO extension_subscriptions (namespace_id, pipeline_id, extension_id, extension_subscription_id, \
            settings, status, status_reason) VALUES (?, ?, ?, ?, ?, ?, ?);"
    )
    .bind(&subscription.namespace_id)
    .bind(&subscription.pipeline_id)
    .bind(&subscription.extension_id)
    .bind(&subscription.extension_subscription_id)
    .bind(&subscription.settings)
    .bind(&subscription.status)
    .bind(&subscription.status_reason);

    let sql = query.sql();

    query
        .execute(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await?;

    Ok(())
}

pub async fn list_by_pipeline(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<ExtensionSubscription>, StorageError> {
    let query = sqlx::query_as::<_, ExtensionSubscription>("SELECT namespace_id, pipeline_id, extension_id, \
    extension_subscription_id, settings, status, status_reason FROM extension_subscriptions WHERE namespace_id = ? \
    AND pipeline_id = ?;").bind(namespace_id).bind(pipeline_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn list_by_extension(
    conn: &mut SqliteConnection,
    extension_id: &str,
) -> Result<Vec<ExtensionSubscription>, StorageError> {
    let query = sqlx::query_as::<_, ExtensionSubscription>("SELECT namespace_id, pipeline_id, extension_id, \
    extension_subscription_id, settings, status, status_reason FROM extension_subscriptions WHERE extension_id= ?;").
    bind(extension_id);

    let sql = query.sql();

    query
        .fetch_all(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn get(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    extension_subscription_id: &str,
) -> Result<ExtensionSubscription, StorageError> {
    let query = sqlx::query_as::<_, ExtensionSubscription>(
        "SELECT namespace_id, pipeline_id, extension_id, extension_subscription_id, settings, status, status_reason \
        FROM extension_subscriptions WHERE namespace_id = ? AND pipeline_id = ? AND extension_id = ? AND \
        extension_subscription_id = ?;",
    )
    .bind(namespace_id)
    .bind(pipeline_id)
    .bind(extension_id)
    .bind(extension_subscription_id);

    let sql = query.sql();

    query
        .fetch_one(conn)
        .map_err(|e| map_sqlx_error(e, sql))
        .await
}

pub async fn update(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    extension_subscription_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut update_query: QueryBuilder<Sqlite> =
        QueryBuilder::new(r#"UPDATE extension_subscriptions SET "#);
    let mut updated_fields_total = 0;

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

    if let Some(value) = &fields.status_reason {
        if updated_fields_total > 0 {
            update_query.push(", ");
        }
        update_query.push("status_reason = ");
        update_query.push_bind(value);
        updated_fields_total += 1;
    }

    // If no fields were updated, return an error
    if updated_fields_total == 0 {
        return Err(StorageError::NoFieldsUpdated);
    }

    update_query.push(" WHERE namespace_id = ");
    update_query.push_bind(namespace_id);
    update_query.push(" AND pipeline_id = ");
    update_query.push_bind(pipeline_id);
    update_query.push(" AND extension_id = ");
    update_query.push_bind(extension_id);
    update_query.push(" AND extension_subscription_id = ");
    update_query.push_bind(extension_subscription_id);
    update_query.push(";");

    let update_query = update_query.build();

    let sql = update_query.sql();

    update_query
        .execute(conn)
        .await
        .map(|_| ())
        .map_err(|e| map_sqlx_error(e, sql))
}

pub async fn delete(
    conn: &mut SqliteConnection,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    extension_subscription_id: &str,
) -> Result<(), StorageError> {
    let query = sqlx::query("DELETE FROM extension_subscriptions WHERE namespace_id = ? AND pipeline_id = ? AND extension_id = ? AND extension_subscription_id = ?;")
        .bind(namespace_id).bind(pipeline_id).bind(extension_id).bind(extension_subscription_id);

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

        let registration = crate::storage::extension_registrations::ExtensionRegistration {
            extension_id: "ext123".to_string(),
            image: "http://example.com/image.png".to_string(),
            registry_auth: "auth_token".to_string(),
            settings: "var1=value1,var2=value2".to_string(),
            created: "2023-04-15T12:34:56".to_string(),
            modified: String::new(),
            status: "Active".to_string(),
            additional_roles: "some_role".to_string(),
            key_id: "key456".to_string(),
        };

        crate::storage::extension_registrations::insert(&mut conn, &registration).await?;

        let namespace = crate::storage::namespaces::Namespace {
            id: "namespace1".into(),
            name: "some_name".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace).await?;

        let pipeline_metadata = crate::storage::pipeline_metadata::PipelineMetadata {
            namespace_id: "namespace1".into(),
            pipeline_id: "pipeline1".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::pipeline_metadata::insert(&mut conn, &pipeline_metadata).await?;

        let subscription = ExtensionSubscription {
            namespace_id: "namespace1".to_string(),
            pipeline_id: "pipeline1".to_string(),
            extension_id: "ext123".to_string(),
            extension_subscription_id: "sub123".to_string(),
            settings: "var1=value1,var2=value2".to_string(),
            status: "Active".to_string(),
            status_reason: "Initial setup".to_string(),
        };

        insert(&mut conn, &subscription).await?;

        Ok((harness, conn))
    }

    #[tokio::test]
    async fn test_list_extension_subscriptions() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let subscriptions = list_by_pipeline(&mut conn, "namespace1", "pipeline1")
            .await
            .expect("Failed to list extension subscriptions");

        assert!(
            !subscriptions.is_empty(),
            "No extension subscriptions returned"
        );

        let some_subscription = subscriptions
            .iter()
            .find(|s| s.extension_id == "ext123")
            .expect("Subscription not found");

        assert_eq!(some_subscription.settings, "var1=value1,var2=value2");
        assert_eq!(some_subscription.status, "Active");
    }

    #[tokio::test]
    async fn test_get_extension_subscription() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let subscription = get(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
            .await
            .expect("Failed to get extension subscription");

        assert_eq!(subscription.settings, "var1=value1,var2=value2");
        assert_eq!(subscription.status, "Active");
    }

    #[tokio::test]
    async fn test_update_extension_subscription() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        let fields = UpdatableFields {
            settings: Some("new_variables".to_string()),
            status: Some("Updated".to_string()),
            status_reason: Some("Manual Update".to_string()),
        };

        update(
            &mut conn,
            "namespace1",
            "pipeline1",
            "ext123",
            "sub123",
            fields,
        )
        .await
        .unwrap();

        let updated = get(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
            .await
            .expect("Failed to retrieve updated subscription");

        assert_eq!(updated.settings, "new_variables");
        assert_eq!(updated.status, "Updated");
        assert_eq!(updated.status_reason, "Manual Update");
    }

    #[tokio::test]
    async fn test_delete_extension_subscription() {
        let (_harness, mut conn) = setup().await.expect("Failed to set up DB");

        delete(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
            .await
            .expect("Failed to delete extension subscription");

        assert!(
            get(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
                .await
                .is_err(),
            "Extension subscription was not deleted"
        );
    }
}
