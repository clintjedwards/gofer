use crate::storage::{map_rusqlite_error, Executable, StorageError};
use rusqlite::Row;
use sea_query::{Expr, Iden, Order, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;

#[derive(Clone, Debug, Default)]
pub struct Deployment {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub deployment_id: i64,
    pub start_version: i64,
    pub end_version: i64,
    pub started: String,
    pub ended: String,
    pub state: String,
    pub status: String,
    pub status_reason: String,
    pub logs: String,
}

impl From<&Row<'_>> for Deployment {
    fn from(row: &Row) -> Self {
        Self {
            namespace_id: row.get_unwrap("namespace_id"),
            pipeline_id: row.get_unwrap("pipeline_id"),
            deployment_id: row.get_unwrap("deployment_id"),
            start_version: row.get_unwrap("start_version"),
            end_version: row.get_unwrap("end_version"),
            started: row.get_unwrap("started"),
            ended: row.get_unwrap("ended"),
            state: row.get_unwrap("state"),
            status: row.get_unwrap("status"),
            status_reason: row.get_unwrap("status_reason"),
            logs: row.get_unwrap("logs"),
        }
    }
}

#[derive(Iden)]
enum DeploymentTable {
    Table,
    NamespaceId,
    PipelineId,
    DeploymentId,
    StartVersion,
    EndVersion,
    Started,
    Ended,
    State,
    Status,
    StatusReason,
    Logs,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub ended: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub logs: Option<String>,
}

pub fn insert(conn: &dyn Executable, deployment: &Deployment) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(DeploymentTable::Table)
        .columns([
            DeploymentTable::NamespaceId,
            DeploymentTable::PipelineId,
            DeploymentTable::DeploymentId,
            DeploymentTable::StartVersion,
            DeploymentTable::EndVersion,
            DeploymentTable::Started,
            DeploymentTable::Ended,
            DeploymentTable::State,
            DeploymentTable::Status,
            DeploymentTable::StatusReason,
            DeploymentTable::Logs,
        ])
        .values_panic([
            deployment.namespace_id.clone().into(),
            deployment.pipeline_id.clone().into(),
            deployment.deployment_id.into(),
            deployment.start_version.into(),
            deployment.end_version.into(),
            deployment.started.clone().into(),
            deployment.ended.clone().into(),
            deployment.state.clone().into(),
            deployment.status.clone().into(),
            deployment.status_reason.clone().into(),
            deployment.logs.clone().into(),
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
) -> Result<Vec<Deployment>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            DeploymentTable::NamespaceId,
            DeploymentTable::PipelineId,
            DeploymentTable::DeploymentId,
            DeploymentTable::StartVersion,
            DeploymentTable::EndVersion,
            DeploymentTable::Started,
            DeploymentTable::Ended,
            DeploymentTable::State,
            DeploymentTable::Status,
            DeploymentTable::StatusReason,
            DeploymentTable::Logs,
        ])
        .from(DeploymentTable::Table)
        .and_where(Expr::col(DeploymentTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(DeploymentTable::PipelineId).eq(pipeline_id))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<Deployment> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(Deployment::from(row));
    }

    Ok(objects)
}

pub fn list_running(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Vec<Deployment>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            DeploymentTable::NamespaceId,
            DeploymentTable::PipelineId,
            DeploymentTable::DeploymentId,
            DeploymentTable::StartVersion,
            DeploymentTable::EndVersion,
            DeploymentTable::Started,
            DeploymentTable::Ended,
            DeploymentTable::State,
            DeploymentTable::Status,
            DeploymentTable::StatusReason,
            DeploymentTable::Logs,
        ])
        .from(DeploymentTable::Table)
        .and_where(Expr::col(DeploymentTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(DeploymentTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(DeploymentTable::State).eq("RUNNING"))
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<Deployment> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(Deployment::from(row));
    }

    Ok(objects)
}

pub fn get(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    deployment_id: i64,
) -> Result<Deployment, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            DeploymentTable::NamespaceId,
            DeploymentTable::PipelineId,
            DeploymentTable::DeploymentId,
            DeploymentTable::StartVersion,
            DeploymentTable::EndVersion,
            DeploymentTable::Started,
            DeploymentTable::Ended,
            DeploymentTable::State,
            DeploymentTable::Status,
            DeploymentTable::StatusReason,
            DeploymentTable::Logs,
        ])
        .from(DeploymentTable::Table)
        .and_where(Expr::col(DeploymentTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(DeploymentTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(DeploymentTable::DeploymentId).eq(deployment_id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Deployment::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn get_latest(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<Deployment, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            DeploymentTable::NamespaceId,
            DeploymentTable::PipelineId,
            DeploymentTable::DeploymentId,
            DeploymentTable::StartVersion,
            DeploymentTable::EndVersion,
            DeploymentTable::Started,
            DeploymentTable::Ended,
            DeploymentTable::State,
            DeploymentTable::Status,
            DeploymentTable::StatusReason,
            DeploymentTable::Logs,
        ])
        .from(DeploymentTable::Table)
        .and_where(Expr::col(DeploymentTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(DeploymentTable::PipelineId).eq(pipeline_id))
        .order_by(DeploymentTable::DeploymentId, Order::Desc)
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Deployment::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    deployment_id: i64,
    fields: UpdatableFields,
) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(DeploymentTable::Table);

    if let Some(value) = fields.ended {
        query.value(DeploymentTable::Ended, value.into());
    }

    if let Some(value) = fields.state {
        query.value(DeploymentTable::State, value.into());
    }

    if let Some(value) = fields.status {
        query.value(DeploymentTable::Status, value.into());
    }

    if let Some(value) = fields.status_reason {
        query.value(DeploymentTable::StatusReason, value.into());
    }

    if let Some(value) = fields.logs {
        query.value(DeploymentTable::Logs, value.into());
    }

    if query.is_empty_values() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query
        .and_where(Expr::col(DeploymentTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(DeploymentTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(DeploymentTable::DeploymentId).eq(deployment_id));

    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

// For the time being there is no need to delete a deployment and normally a deployment should not be deleted.
// But we might make an admin route that allows this.
#[allow(dead_code)]
pub fn delete(
    conn: &dyn Executable,
    namespace_id: &str,
    pipeline_id: &str,
    deployment_id: i64,
) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(DeploymentTable::Table)
        .and_where(Expr::col(DeploymentTable::NamespaceId).eq(namespace_id))
        .and_where(Expr::col(DeploymentTable::PipelineId).eq(pipeline_id))
        .and_where(Expr::col(DeploymentTable::DeploymentId).eq(deployment_id))
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
            name: "some_pipeline_id".into(),
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

        let new_pipeline_config = crate::storage::pipeline_configs::PipelineConfig {
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

        crate::storage::pipeline_configs::insert(&mut conn, &new_pipeline_config)
            .expect("Failed to insert pipeline_config");

        let deployment = Deployment {
            namespace_id: "some_id".to_string(),
            pipeline_id: "some_pipeline_id".to_string(),
            deployment_id: 1,
            start_version: 1,
            end_version: 2,
            started: "2021-01-01T00:00:00Z".to_string(),
            ended: "2021-01-01T01:00:00Z".to_string(),
            state: "Deploymenting".to_string(),
            status: "Active".to_string(),
            status_reason: "No issues".to_string(),
            logs: "some_logs".into(),
        };

        insert(&mut conn, &deployment)?;

        Ok((harness, conn))
    }

    fn test_list_deployments() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let deployments =
            list(&mut conn, "some_id", "some_pipeline_id").expect("Failed to list deployments");

        assert!(!deployments.is_empty(), "No deployments returned");

        let some_deployment = deployments
            .iter()
            .find(|n| n.deployment_id == 1)
            .expect("Deployment not found");
        assert_eq!(some_deployment.pipeline_id, "some_pipeline_id");
        assert_eq!(some_deployment.state, "Deploymenting");
    }

    fn test_get_deployment() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let deployment =
            get(&mut conn, "some_id", "some_pipeline_id", 1).expect("Failed to get deployment");

        assert_eq!(deployment.pipeline_id, "some_pipeline_id");
        assert_eq!(deployment.state, "Deploymenting");
    }

    fn test_update_deployment() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            ended: Some("2021-01-01T02:00:00Z".to_string()),
            state: Some("Failed".to_string()),
            status: Some("Error".to_string()),
            status_reason: Some("Encountered an error".to_string()),
            logs: None,
        };

        update(
            &mut conn,
            "some_id",
            "some_pipeline_id",
            1,
            fields_to_update,
        )
        .expect("Failed to update deployment");

        let updated_deployment = get(&mut conn, "some_id", "some_pipeline_id", 1)
            .expect("Failed to retrieve updated deployment");

        assert_eq!(updated_deployment.state, "Failed");
        assert_eq!(updated_deployment.status, "Error");
    }

    fn test_delete_deployment() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id", "some_pipeline_id", 1).expect("Failed to delete deployment");

        assert!(
            get(&mut conn, "some_id", "some_pipeline_id", 1).is_err(),
            "Deployment was not deleted"
        );
    }
}
