use sqlx::FromRow;

#[derive(Clone, Debug, Default, FromRow)]
pub struct Task {
    pub namespace_id: String,
    pub pipeline_id: String,
    pub pipeline_config_version: i64,
    pub task_id: String,
    pub description: String,
    pub image: String,
    pub registry_auth: String,
    pub depends_on: String,
    pub variables: String,
    pub entrypoint: String,
    pub command: String,
    pub inject_api_token: bool,
}
