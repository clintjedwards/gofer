use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct Namespace {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created: String,
    pub modified: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub modified: String,
}
