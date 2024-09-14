use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct ObjectStoreExtensionKey {
    pub extension_id: String,
    pub key: String,
    pub created: String,
}
