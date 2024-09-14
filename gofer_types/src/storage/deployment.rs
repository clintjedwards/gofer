use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
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

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub ended: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub logs: Option<String>,
}
