use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct PipelineConfig {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub version: i64,
    pub parallelism: i64,
    pub name: String,
    pub description: String,
    pub registered: String,
    pub deprecated: String,
    pub state: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub deprecated: Option<String>,
    pub state: Option<String>,
}
