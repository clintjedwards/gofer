use crate::storage::{map_rusqlite_error, StorageError};
use rusqlite::{Connection, Row};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct ExtensionSubscription {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub extension_id: String,
    pub extension_subscription_id: String,
    pub settings: String,
    pub status: String,
    pub status_reason: String,
}

impl From<&Row<'_>> for ExtensionSubscription {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            extension_id: row.get_unwrap("extension_id"),
            extension_subscription_id: row.get_unwrap("extension_subscription_id"),
            settings: row.get_unwrap("settings"),
            status: row.get_unwrap("status"),
            status_reason: row.get_unwrap("status_reason"),
        }
    }
}

#[derive(Iden)]
enum ExtensionSubscriptionTable {
    Table,
    NamespaceId,
    PipelineId,
    ExtensionId,
    ExtensionSubscriptionId,
    Settings,
    Status,
    StatusReason,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub settings: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
}

pub fn insert(conn: &Connection, subscription: &ExtensionSubscription) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(ExtensionSubscriptionTable::Table)
        .columns([
            ExtensionSubscriptionTable::NamespaceId,
            ExtensionSubscriptionTable::PipelineId,
            ExtensionSubscriptionTable::ExtensionId,
            ExtensionSubscriptionTable::ExtensionSubscriptionId,
            ExtensionSubscriptionTable::Settings,
            ExtensionSubscriptionTable::Status,
            ExtensionSubscriptionTable::StatusReason,
        ])
        .values_panic([
            subscription.namespace_id.clone().into(),
            subscription.pipeline_id.clone().into(),
            subscription.extension_id.clone().into(),
            subscription.extension_subscription_id.clone().into(),
            subscription.settings.clone().into(),
            subscription.status.clone().into(),
            subscription.status_reason.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list_by_pipeline(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<ExtensionSubscription>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ExtensionSubscriptionTable::NamespaceId,
            ExtensionSubscriptionTable::PipelineId,
            ExtensionSubscriptionTable::ExtensionId,
            ExtensionSubscriptionTable::ExtensionSubscriptionId,
            ExtensionSubscriptionTable::Settings,
            ExtensionSubscriptionTable::Status,
            ExtensionSubscriptionTable::StatusReason,
        ])
        .from(ExtensionSubscriptionTable::Table)
        .and_where(Expr::col(ExtensionSubscriptionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::PipelineId).eq(pipeline_id))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<ExtensionSubscription> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(ExtensionSubscription::from(row));
    }

    Ok(objects)
}

pub fn list_by_extension(
    conn: &Connection,
    extension_id: &str,
) -> Result<Vec<ExtensionSubscription>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ExtensionSubscriptionTable::NamespaceId,
            ExtensionSubscriptionTable::PipelineId,
            ExtensionSubscriptionTable::ExtensionId,
            ExtensionSubscriptionTable::ExtensionSubscriptionId,
            ExtensionSubscriptionTable::Settings,
            ExtensionSubscriptionTable::Status,
            ExtensionSubscriptionTable::StatusReason,
        ])
        .from(ExtensionSubscriptionTable::Table)
        .and_where(Expr::col(ExtensionSubscriptionTable::ExtensionId).eq(extension_id))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<ExtensionSubscription> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(ExtensionSubscription::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    extension_subscription_id: &str,
) -> Result<ExtensionSubscription, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            ExtensionSubscriptionTable::NamespaceId,
            ExtensionSubscriptionTable::PipelineId,
            ExtensionSubscriptionTable::ExtensionId,
            ExtensionSubscriptionTable::ExtensionSubscriptionId,
            ExtensionSubscriptionTable::Settings,
            ExtensionSubscriptionTable::Status,
            ExtensionSubscriptionTable::StatusReason,
        ])
        .from(ExtensionSubscriptionTable::Table)
        .and_where(Expr::col(ExtensionSubscriptionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::ExtensionId).eq(extension_id))
        .and_where(
            Expr::col(ExtensionSubscriptionTable::ExtensionSubscriptionId)
                .eq(extension_subscription_id),
        )
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(ExtensionSubscription::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    extension_subscription_id: &str,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(ExtensionSubscriptionTable::Table);

    if let Some(value) = fields.settings {
        query.value(ExtensionSubscriptionTable::Settings, value);
    }

    if let Some(value) = fields.status {
        query.value(ExtensionSubscriptionTable::Status, value);
    }

    if let Some(value) = fields.status_reason {
        query.value(ExtensionSubscriptionTable::StatusReason, value);
    }

    if query.get_values().is_empty() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query
        .and_where(Expr::col(ExtensionSubscriptionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::ExtensionId).eq(extension_id))
        .and_where(
            Expr::col(ExtensionSubscriptionTable::ExtensionSubscriptionId)
                .eq(extension_subscription_id),
        );

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(
    conn: &Connection,
    namespace_id: &str,
    pipeline_id: &str,
    extension_id: &str,
    extension_subscription_id: &str,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(ExtensionSubscriptionTable::Table)
        .and_where(Expr::col(ExtensionSubscriptionTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(ExtensionSubscriptionTable::ExtensionId).eq(extension_id))
        .and_where(
            Expr::col(ExtensionSubscriptionTable::ExtensionSubscriptionId)
                .eq(extension_subscription_id),
        )
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

        crate::storage::extension_registrations::insert(&mut conn, &registration)?;

        let namespace = crate::storage::namespaces::Namespace {
            id: "namespace1".into(),
            name: "some_name".into(),
            description: "some_description".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::namespaces::insert(&mut conn, &namespace)?;

        let pipeline_metadata = crate::storage::pipeline_metadata::PipelineMetadata {
            namespace_id: "namespace1".into(),
            pipeline_id: "pipeline1".into(),
            state: "some_state".into(),
            created: "some_time".into(),
            modified: "some_time_mod".into(),
        };

        crate::storage::pipeline_metadata::insert(&mut conn, &pipeline_metadata)?;

        let subscription = ExtensionSubscription {
            namespace_id: "namespace1".to_string(),
            pipeline_id: "pipeline1".to_string(),
            extension_id: "ext123".to_string(),
            extension_subscription_id: "sub123".to_string(),
            settings: "var1=value1,var2=value2".to_string(),
            status: "Active".to_string(),
            status_reason: "Initial setup".to_string(),
        };

        insert(&mut conn, &subscription)?;

        Ok((harness, conn))
    }

    fn test_list_extension_subscriptions() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let subscriptions = list_by_pipeline(&mut conn, "namespace1", "pipeline1")
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

    fn test_get_extension_subscription() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let subscription = get(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
            .expect("Failed to get extension subscription");

        assert_eq!(subscription.settings, "var1=value1,var2=value2");
        assert_eq!(subscription.status, "Active");
    }

    fn test_update_extension_subscription() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

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
        .unwrap();

        let updated = get(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
            .expect("Failed to retrieve updated subscription");

        assert_eq!(updated.settings, "new_variables");
        assert_eq!(updated.status, "Updated");
        assert_eq!(updated.status_reason, "Manual Update");
    }

    fn test_delete_extension_subscription() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "namespace1", "pipeline1", "ext123", "sub123")
            .expect("Failed to delete extension subscription");

        assert!(
            get(&mut conn, "namespace1", "pipeline1", "ext123", "sub123").is_err(),
            "Extension subscription was not deleted"
        );
    }
}
