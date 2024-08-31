use crate::conf::ConfigType;
use crate::{object_store, scheduler, secret_store};
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_API_CONFIG: &str = include_str!("./default_api_config.toml");

#[derive(Deserialize, Default, Debug, Clone)]
pub struct ApiConfig {
    pub api: Api,
    pub development: Development,
    pub extensions: Extensions,
    pub external_events: ExternalEvents,
    pub scheduler: Scheduler,
    pub server: Server,
    pub object_store: ObjectStore,
    pub secret_store: SecretStore,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Api {
    /// The limit automatically imposed if the pipeline does not define a limit. 0 is unlimited.
    pub run_parallelism_limit: u64,

    /// How many total versions of an individual pipeline to keep. The oldest version of a pipeline over this limit
    /// gets deleted. 0 means don't delete versions.
    pub pipeline_version_retention: u64,

    /// Controls how long Gofer will hold onto events before discarding them (in seconds).
    /// This is important factor in disk space and memory footprint.
    ///
    /// Example: Rough math on a 5,000 pipeline Gofer instance with a full 6 months of retention
    ///  puts the memory and storage footprint at about 9 GB.
    pub event_log_retention: u64,

    /// How often the background process for pruning events should run (in seconds).
    pub event_prune_interval: u64,

    /// The entire service's log level including extensions.
    pub log_level: String,

    /// The total amount of runs before logs of the oldest run will be deleted.
    pub task_execution_log_retention: u64,

    /// Directory to store task execution log files.
    pub task_execution_logs_dir: String,

    /// Time in seconds the scheduler will wait for a normal user container(not-trigger containers)
    /// to stop. When the timeout is reached the container will be forcefully terminated.
    /// You can you use a timeout of 0 to convey that no timeout should be specified and the
    /// scheduler should instantly kill all containers.
    pub task_execution_stop_timeout: u64,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Development {
    /// Tells the logging package to use human readable output.
    pub pretty_logging: bool,

    /// Turns off authentication.
    pub bypass_auth: bool,

    /// Automatically loads localhost certs for development.
    pub use_included_certs: bool,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Extensions {
    /// Gofer attempts to automatically install known good, default extensions defined as the "standard extensions".
    pub install_std_extensions: bool,

    /// The time the scheduler will wait for an extension container to stop. After this period Gofer will attempt to
    /// force stop the container.
    pub stop_timeout: u64,

    /// These are the paths to the certificate pieces the server will pass to each extension such that it can use.
    pub use_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,

    /// When attempting to communicate from Gofer to an extension verify the cert is correct and known.
    pub verify_certs: bool,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct ExternalEvents {
    pub enable: bool,
    pub bind_address: String,
    pub use_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Scheduler {
    pub engine: scheduler::Engine,
    pub docker: Option<scheduler::docker::Config>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Server {
    /// The URL that users use to interact with Gofer. Should be the full uri to the root. Ex. http://example.org
    pub url: String,

    /// The bind address the server will listen on. Ex: 0.0.0.0:8080
    pub bind_address: String,

    /// URL for the Gofer API that can be contacted by extensions. This is important due to extensions likely being
    /// part of a local network and as such they need a different address than the default 'url' address.
    ///
    /// For example, development for Gofer is done locally and that requires us to set this address to the 'docker host'
    /// address such that when extensions make a request they make it through the proper network stack.
    pub extension_address: String,

    /// Path to Gofer's database.
    pub storage_path: String,

    /// The total amount of results the database will attempt to pass back when a limit is not explicitly given.
    pub storage_results_limit: u64,

    /// These are the paths to the certificate pieces the server needs to use for TLS.
    pub use_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct ObjectStore {
    /// The ObjectStore engine used by the backend.
    pub engine: object_store::Engine,

    /// Pipeline Objects last forever but are limited in number. This is the total amount of items that can be stored
    /// per pipeline before gofer starts deleting objects operating in a ring buffer fashion.
    pub pipeline_object_limit: u64,

    /// Objects stored at the run level are unlimited in number, but only last for a certain number of runs.
    /// The number below controls how many runs until the run objects for the oldest run will be deleted.
    /// Ex. an object stored on run number #5 with an expiry of 2(only the last two runs keep their objects) will be
    /// deleted on run #7 regardless of run health.
    pub run_object_expiry: u64,

    pub sqlite: Option<object_store::sqlite::Config>,
}

#[derive(Deserialize, Default, Debug, Clone)]
pub struct SecretStore {
    /// The SecretStore engine used by the backend.
    pub engine: secret_store::Engine,
    pub sqlite: Option<secret_store::sqlite::Config>,
}

impl ConfigType for ApiConfig {
    fn default_config() -> &'static str {
        DEFAULT_API_CONFIG
    }

    fn config_paths() -> Vec<std::path::PathBuf> {
        vec![PathBuf::from("/etc/gofer/gofer_web.toml")]
    }

    fn env_prefix() -> &'static str {
        "GOFER_WEB_"
    }
}
