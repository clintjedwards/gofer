use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct ExtensionSubscription {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub extension_id: String,
    pub extension_subscription_id: String,
    pub settings: String,
    pub status: String,
    pub status_reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct UpdatableFields {
    pub settings: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
}
