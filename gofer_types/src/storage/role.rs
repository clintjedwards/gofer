use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct Role {
    pub id: String,
    pub description: String,
    pub permissions: String,
    pub system_role: bool,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub description: Option<String>,
    pub permissions: Option<String>,
}
