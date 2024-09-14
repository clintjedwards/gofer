use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct ExtensionRegistration {
    pub extension_id: String,
    pub image: String,
    pub registry_auth: String,
    pub settings: String,
    pub created: String,
    pub modified: String,
    pub status: String,
    pub key_id: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub image: Option<String>,
    pub registry_auth: Option<String>,
    pub settings: Option<String>,
    pub status: Option<String>,
    pub key_id: Option<String>,
    pub modified: String,
}
