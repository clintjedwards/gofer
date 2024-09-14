use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct ObjectStorePipelineKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub key: String,
    pub created: String,
}
