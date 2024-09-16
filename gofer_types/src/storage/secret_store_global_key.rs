use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct SecretStoreGlobalKey {
    pub key: String,
    pub namespaces: String,
    pub created: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub namespaces: Option<String>,
}
