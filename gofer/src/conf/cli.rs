use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Clone, econf::LoadEnv)]
pub struct Config {
    pub dev_mode: bool,
    pub namespace: Option<String>,
    pub server: String,
}
