use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct SecretStorePipelineKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub key: String,
    pub created: String,
}
