use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct PipelineMetadata {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub state: String,
    pub created: String,
    pub modified: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub state: Option<String>,
    pub modified: String,
}
