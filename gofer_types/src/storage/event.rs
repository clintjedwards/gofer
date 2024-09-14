use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct Event {
    pub id: String,
    pub kind: String,
    pub details: String,
    pub emitted: String,
}
