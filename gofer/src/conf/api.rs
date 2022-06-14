use serde::Deserialize;

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub general: General,
    pub server: Server,
    pub scheduler: Scheduler,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct General {
    /// Turns on humanized debug messages, extra debug logging for the webserver and other
    /// convenient features for development. Usually turned on along side LogLevel=debug.
    pub dev_mode: bool,
    pub log_level: String,
    pub encryption_key: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Server {
    pub url: String,
    pub storage_path: String,
    pub tls_cert: String,
    pub tls_key: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Scheduler {
    pub docker: Option<DockerScheduler>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct DockerScheduler {
    pub prune: bool,
    pub prune_interval: u64, // in seconds
}
