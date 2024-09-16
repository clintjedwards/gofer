use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct Token {
    pub id: String,
    pub hash: String,
    pub created: String,
    pub metadata: String,
    pub expires: String,
    pub disabled: bool,
    pub roles: String,
    pub user: String,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub disabled: Option<bool>,
}
