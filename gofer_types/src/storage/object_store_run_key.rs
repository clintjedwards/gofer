use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct ObjectStoreRunKey {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub run_id: i64,
    pub key: String,
    pub created: String,
}
