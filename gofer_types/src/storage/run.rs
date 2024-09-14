use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct Run {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub pipeline_config_version: i64,
    pub run_id: i64,
    pub started: String,
    pub ended: String,
    pub state: String,
    pub status: String,
    pub status_reason: String,
    pub initiator: String,
    pub variables: String,
    pub token_id: Option<String>,
    pub store_objects_expired: bool,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub ended: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub variables: Option<String>,
    pub store_objects_expired: Option<bool>,
}
