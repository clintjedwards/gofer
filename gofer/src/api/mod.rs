//! The REST API package for Gofer; manages the API models and handlers.

mod deployments;
mod event_utils;
mod events;
pub mod extensions;
mod external;
mod namespaces;
mod objects;
mod orchestrator;
mod permissioning;
mod pipeline_configs;
mod pipelines;
mod runs;
mod secrets;
mod static_router;
mod subscriptions;
mod system;
pub mod task_executions;
mod tasks;
mod tokens;

use crate::{conf, object_store, scheduler, secret_store, storage};
use anyhow::{Context, Result, anyhow, bail};
use dashmap::DashMap;
use dropshot::{
    ApiDescription, Body, ClientErrorStatusCode, CompressionConfig, ConfigDropshot, ConfigTls,
    DropshotState, EndpointTagPolicy, ErrorStatusCode, HandlerError, HandlerTaskMode, HttpError,
    HttpServer, RequestInfo, ServerBuilder, ServerContext, TagConfig, TagDetails,
    WebsocketConnectionRaw,
};
use futures::Future;
use lazy_regex::regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use std::{net::SocketAddr, pin::Pin, str::FromStr, sync::Arc, sync::atomic};
use strum::{Display, EnumString};
use tokio::signal;
use tokio_tungstenite::WebSocketStream;
use tracing::{error, info, warn};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};

/// GOFER_EOF is a special string marker we include at the end of log files.
/// It denotes that no further logs will be written. This is to provide the functionality for downstream
/// applications to follow log files and not also have to monitor the container for state to know when
/// logs will no longer be printed.
const GOFER_EOF: &str = "GOFER_EOF";

const BUILD_SEMVER: &str = env!("BUILD_SEMVER");
const BUILD_COMMIT: &str = env!("BUILD_COMMIT");

/// These certs are purely for ease of use in development; We embed it into the binary so that it's easy for developers
/// to run everything locally and have as close to an experience as production as possible.
/// These certs are NOT MEANT TO BE USED IN PRODUCTION.
const LOCALHOST_CERT: &[u8] = include_bytes!("./localhost.crt");
const LOCALHOST_KEY: &[u8] = include_bytes!("./localhost.key");

/// A constant for the header that tracks which version of the API a client has requested.
const API_VERSION_HEADER: &str = "gofer-api-version";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ApiVersion {
    V0,
}

impl ApiVersion {
    pub fn to_list() -> [String; 1] {
        ["v0".into()]
    }
}

impl FromStr for ApiVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "v0" => Ok(ApiVersion::V0),
            _ => Err(anyhow::anyhow!("Invalid API version")),
        }
    }
}

fn generate_inject_api_token_role_id(namespace_id: &str, pipeline_id: &str) -> String {
    format!("inject_api_token_{namespace_id}_{pipeline_id}")
}

/// Holds objects that are created and used over the lifetime of a single request.
///
/// This is different from [`dropshot::RequestContext`] since that is automatically created for us but we need some
/// more Gofer specific information.
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    #[allow(dead_code)]
    api_version: ApiVersion,
    auth: permissioning::AuthContext,

    /// Admin tokens skip permission checks entirely.
    admin: bool,

    /// Every grant given to the token across all of its roles.
    grants: permissioning::Grants,
}

impl RequestMetadata {
    pub fn allows(
        &self,
        requirement: &permissioning::Requirement,
        action: &permissioning::Action,
    ) -> bool {
        self.admin || self.grants.allows(requirement, action)
    }
}

#[derive(Debug, Clone)]
pub struct PreflightOptions {
    bypass_auth: bool,
    admin_only: bool,

    /// Allows unauthenticated users to access particular things under the default namespace.
    allow_anonymous: bool,

    /// What the token needs to be granted in order to access this route. Use the ids from the request path as
    /// targets so they can be matched against the token's grants.
    requires: permissioning::Requirement,
    action: permissioning::Action,
}

/// Holds all objects that need to exist for the entire runtime of the API server.
#[derive(Debug)]
pub struct ApiState {
    /// The API configuration read in at init.
    config: conf::api::ApiConfig,

    /// The config file Gofer was started with, if one was given on the command line. Extension reloads reread it.
    config_path: Option<PathBuf>,

    /// An in-memory mapping of currently registered and started extensions. These extensions are registered on startup
    /// and launched as long running containers via the scheduler. Gofer refers to this cache as a way to communicate
    /// quickly with the containers and their potentially changing endpoints.
    extensions: DashMap<String, extensions::Extension>,

    /// Acts as an event bus for the Gofer application. It is used throughout the whole application to give
    /// different parts of the application the ability to listen for and respond to events that might happen in other
    /// parts.
    event_bus: event_utils::EventBus,

    /// An in-memory count of how many runs each pipeline currently has in-progress.
    in_progress_runs: DashMap<String, atomic::AtomicU64>,

    /// Controls if the pipelines are allowed to run globally. If this is set to false the entire Gofer service will
    /// not schedule new runs.
    ignore_pipeline_run_events: atomic::AtomicBool,

    /// `Storage` represents the main backend storage implementation. Gofer stores most of its critical state information
    /// using this storage mechanism.
    storage: storage::Db,

    /// `Scheduler` is the mechanism in which Gofer uses to run its containers(tasks). This is the connection to
    /// the scheduler backend that we're using.
    scheduler: Box<dyn scheduler::Scheduler>,

    /// `Orchestrator` is responsible for launching, monitoring, and recovery of runs that are started.
    orchestrator: orchestrator::Orchestrator,

    /// ObjectStore is the mechanism in which Gofer stores pipeline and run level objects. The implementation here
    /// is meant to act as a basic object store that Gofer's connections can use freely.
    object_store: Box<dyn object_store::ObjectStore>,

    /// SecretStore is the mechanism in which Gofer manages pipeline secrets.
    secret_store: Box<dyn secret_store::SecretStore>,

    /// Short lived, single use codes that let `gofer web` sign the browser in without putting a token in a URL.
    /// Keyed by code. Kept in memory only, since a code that outlives a restart isn't worth keeping.
    web_logins: DashMap<String, tokens::WebLogin>,

    /// Held while an extension is being started, stopped, or swapped so two reloads can't fight over the same
    /// container.
    extension_lock: tokio::sync::Mutex<()>,
}

impl ApiState {
    #[allow(clippy::too_many_arguments)]
    fn new(
        conf: conf::api::ApiConfig,
        config_path: Option<PathBuf>,
        event_bus: event_utils::EventBus,
        ignore_pipeline_run_events: atomic::AtomicBool,
        object_store: Box<dyn object_store::ObjectStore>,
        orchestrator: orchestrator::Orchestrator,
        scheduler: Box<dyn scheduler::Scheduler>,
        secret_store: Box<dyn secret_store::SecretStore>,
        storage: storage::Db,
    ) -> Self {
        Self {
            config: conf.clone(),
            config_path,
            event_bus,
            extensions: DashMap::new(),
            ignore_pipeline_run_events,
            in_progress_runs: DashMap::new(),
            object_store,
            orchestrator,
            scheduler,
            secret_store,
            storage,
            web_logins: DashMap::new(),
            extension_lock: tokio::sync::Mutex::new(()),
        }
    }
}

fn check_version_handler(request: &RequestInfo) -> Result<ApiVersion, HttpError> {
    let version_header = match request.headers().get(API_VERSION_HEADER) {
        Some(version_header) => version_header,
        None => {
            return Err(HttpError::for_bad_request(
                None,
                "Gofer version header missing; `gofer-api-version`".into(),
            ));
        }
    };
    let version_header = version_header.to_str().map_err(|e| {
        HttpError::for_bad_request(
            None,
            format!("Could not parse gofer-api-version header; {:#?}", e),
        )
    })?;

    let version = match ApiVersion::from_str(version_header) {
        Ok(version) => version,
        Err(_) => {
            return Err(HttpError::for_bad_request(
                None,
                format!(
                    "Incorrect Gofer version header; should be one of {:?}",
                    ApiVersion::to_list()
                ),
            ));
        }
    };

    Ok(version)
}

fn init_logger(log_level: &str, pretty: bool) -> Result<()> {
    let level =
        LevelFilter::from_str(log_level).context("could not parse 'log_level' configuration")?;

    let filter = EnvFilter::from_default_env()
        // These directives filter out debug information that is too numerous and we generally don't need during
        // development.
        .add_directive("sqlx=off".parse().expect("Invalid directive"))
        .add_directive("h2=off".parse().expect("Invalid directive"))
        .add_directive("hyper=off".parse().expect("Invalid directive"))
        .add_directive("rustls=off".parse().expect("Invalid directive"))
        .add_directive("bollard=off".parse().expect("Invalid directive"))
        .add_directive("reqwest=off".parse().expect("Invalid directive"))
        .add_directive("tungstenite=off".parse().expect("Invalid directive"))
        .add_directive("dropshot=off".parse().expect("Invalid directive"))
        .add_directive(level.into()); // Accept debug level logs and above for everything else

    if pretty {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .json()
            .init();
    }

    if pretty {
        warn!("pretty logging activated due to config value 'development.pretty_logging'");
    }

    Ok(())
}

/// This is an initialization function for the dropshot type [`dropshot::ApiDescription`]. It allows us to register
/// our routes and configure other things about Gofer's attachment to the OpenAPI spec.
///
/// We keep this in a separate function from the [`init_api`] function such that we can call it from other modules
/// in case we want to generate the OpenAPI spec out of band from the server startup (which is often the case).
fn init_api_description() -> Result<ApiDescription<Arc<ApiState>>> {
    let mut api = ApiDescription::new();
    api = set_tagging_policy(api);
    register_routes(&mut api);

    Ok(api)
}

/// The main initialization function for the Gofer main process. Encompasses all functionality that needs to happen
/// before Gofer can successfully start serving requests.
async fn init_api(
    conf: conf::api::ApiConfig,
    config_path: Option<PathBuf>,
) -> Result<Arc<ApiState>> {
    // First we initialize all the main subsystems.
    let storage = storage::Db::new(&conf.server.storage_path)
        .await
        .context("Could not initialize storage")?;
    let scheduler = scheduler::new(&conf.scheduler)
        .await
        .context("Could not initialize scheduler")?;
    let object_store = object_store::new(&conf.object_store)
        .await
        .context("Could not initialize object store")?;
    let orchestrator = orchestrator::Orchestrator::new(conf.api.global_run_concurrency_limit);
    let secret_store = secret_store::new(&conf.secret_store)
        .await
        .context("Could not initialize secret store")?;
    let event_bus = event_utils::EventBus::new(
        storage.clone(),
        conf.api.event_log_retention,
        conf.api.event_prune_interval,
    );

    // Load our current value for ignore_pipeline_run_events into memory.
    let mut conn = match storage.read_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            bail!(
                "Could not establish connection to database during api initialization: {:#?}",
                e
            );
        }
    };

    let ignore_pipeline_runs = match storage::system::get_system_parameters(&mut conn).await {
        Ok(value) => atomic::AtomicBool::new(value.ignore_pipeline_run_events),
        Err(e) => bail!(
            "Could not get system parameters during api initialization; {:#?}",
            e
        ),
    };

    let api_state = Arc::new(ApiState::new(
        conf.clone(),
        config_path,
        event_bus,
        ignore_pipeline_runs,
        object_store,
        orchestrator,
        scheduler,
        secret_store,
        storage,
    ));

    // Then we perform additional housekeeping.

    namespaces::create_default_namespace(api_state.clone())
        .await
        .context("Could not create default namespace")?;

    permissioning::create_system_roles(api_state.clone())
        .await
        .context("Could not create system roles")?;

    // We attempt to recover any lost runs from a crash before we start the API.
    api_state
        .orchestrator
        .recover_runs(api_state.clone())
        .await?;

    Ok(api_state)
}

/// Starts both the gofer main api and the external events web service.
pub async fn start_web_services(config_path: Option<PathBuf>) -> Result<()> {
    let conf = conf::Configuration::<conf::api::ApiConfig>::load(config_path.clone())
        .context("Could not initialize configuration")?;

    init_logger(&conf.api.log_level, conf.development.pretty_logging)?;

    let api_state = init_api(conf.clone(), config_path)
        .await
        .context("Could not initialize API")?;

    if conf.external_events.enable {
        tokio::spawn(external::start_web_service(conf.clone(), api_state.clone()));
    }

    start_web_service(conf, api_state.clone()).await?;

    // Cleanup
    extensions::stop_extensions(api_state).await;

    Ok(())
}

/// Start the main Gofer api web service. Blocks until server finishes.
pub async fn start_web_service(conf: conf::api::ApiConfig, api_state: Arc<ApiState>) -> Result<()> {
    if conf.development.bypass_auth {
        warn!("Bypass auth activated due to config value 'development.bypass_auth'");
    }

    if conf.extensions.use_tls && !conf.extensions.verify_certs {
        warn!("Skipping verification of cert on extensions due to 'extensions.verify_cert'");
    }

    let bind_address = std::net::SocketAddr::from_str(&conf.server.bind_address.clone()).with_context(|| {
        format!(
            "Could not parse url '{}' while trying to bind binary to port; \
    should be in format '<ip>:<port>'; Please be sure to use an ip instead of something like 'localhost', \
    when attempting to bind",
            conf.server.bind_address.clone()
        )
    })?;

    let dropshot_conf = ConfigDropshot {
        bind_address,

        // 500MB to allow for extra large objects, this is overwritten in the per handler endpoint struct for routes
        // that require more than this.
        default_request_body_max_bytes: 524288000,

        // If a client disconnects run the handler to completion still. Eventually we'll want to save resources
        // by allowing the handler to early cancel, but until this is more developed lets just run it to completion.
        default_handler_task_mode: HandlerTaskMode::Detached,

        // Only applies to compressible content types and when the client asks for it via Accept-Encoding.
        compression: CompressionConfig::Gzip,
    };

    let api = init_api_description()?;

    let tls_config = match conf.server.use_tls {
        true => {
            let (tls_cert, tls_key) = load_tls(
                conf.development.use_included_certs,
                conf.server.tls_cert_path,
                conf.server.tls_key_path,
            )?;

            Some(ConfigTls::AsBytes {
                certs: tls_cert,
                key: tls_key,
            })
        }
        false => None,
    };

    let server = ServerBuilder::new(api, api_state.clone(), Some(Arc::new(Middleware)))
        .config(dropshot_conf)
        .tls(tls_config)
        .start()
        .map_err(|error| anyhow!("failed to create server: {}", error))?;

    let shutdown = server.wait_for_shutdown();

    tokio::spawn(wait_for_shutdown_signal(server));

    info!(
        message = "Started Gofer http service",
        host = %bind_address.ip(),
        port = %bind_address.port(),
        tls = conf.server.use_tls,
    );

    // This might cause a race conditions if the containers somehow start up before the API, but this could be trivially
    // solved on either side by either delaying this call a bit or probably in a less brittle fashion by writing some
    // retry logic on the container side.
    // A broken extension shouldn't keep Gofer from starting, so failures here are recorded on the extension itself
    // and shown by `gofer extension list` rather than returned.
    extensions::reconcile::reconcile_on_boot(api_state.clone()).await;

    shutdown
        .await
        .map_err(|error| anyhow!("Server encountered errors while running; {:#?}", error))
}

/// This is called from another binary to write the openAPI spec to a file.
#[allow(dead_code)]
pub fn write_openapi_spec(path: PathBuf) -> Result<()> {
    let api = init_api_description()?;
    let mut file = std::fs::File::create(path)?;
    api.openapi("Gofer", semver::Version::from_str(BUILD_SEMVER).unwrap())
        .write(&mut file)?;

    Ok(())
}

async fn wait_for_shutdown_signal(server: HttpServer<Arc<ApiState>>) {
    listen_for_terminate_signal().await;

    server.close().await.unwrap()
}

async fn listen_for_terminate_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Loads TLS files into memory so we can hand them over in a consistent format regardless of where they come from.
/// Returns (cert, key) as bytes
fn load_tls(
    use_included_certs: bool,
    tls_cert_path: Option<String>,
    tls_key_path: Option<String>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    if use_included_certs {
        warn!(
            "Using included localhost certs due to config value 'development.use_included_certs'"
        );

        return Ok((LOCALHOST_CERT.to_vec(), LOCALHOST_KEY.to_vec()));
    }

    if tls_cert_path.is_none() || tls_key_path.is_none() {
        bail!("Could not load TLS certificates; one or more paths are empty")
    }

    let tls_cert = std::fs::read(tls_cert_path.unwrap()).context(
        "Error occurred while attempting to read TLS \
          cert file from path",
    )?;

    let tls_key = std::fs::read(tls_key_path.unwrap()).context(
        "Error occurred while attempting to read TLS \
          key file from path",
    )?;

    Ok((tls_cert, tls_key))
}

/// Registers the handlers into the API harness. Can panic.
///
/// It's better to use unwrap here for two reasons. The first is that we fail fast and early when a handler is incorrect
/// in some way. The second is that since the underlying error returned by the register function is simply a string
/// it can be hard to know which route caused said error without unwrapping it on the spot.
fn register_routes(api: &mut ApiDescription<Arc<ApiState>>) {
    /* /api/namespaces */
    api.register(namespaces::list_namespaces).unwrap();
    api.register(namespaces::create_namespace).unwrap();

    /* /api/namespaces/{id} */
    api.register(namespaces::get_namespace).unwrap();
    api.register(namespaces::delete_namespace).unwrap();
    api.register(namespaces::update_namespace).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines */
    api.register(pipelines::list_pipelines).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id} */
    api.register(pipelines::get_pipeline).unwrap();
    api.register(pipelines::update_pipeline).unwrap();
    api.register(pipelines::delete_pipeline).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs */
    api.register(pipeline_configs::list_configs).unwrap();
    api.register(pipeline_configs::register_config).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/configs/{version} */
    api.register(pipeline_configs::get_config).unwrap();
    api.register(pipeline_configs::deploy_config).unwrap();
    api.register(pipeline_configs::delete_config).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments */
    api.register(deployments::list_deployments).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/deployments/{deployment_id} */
    api.register(deployments::get_deployment).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs */
    api.register(runs::list_runs).unwrap();
    api.register(runs::start_run).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id} */
    api.register(runs::get_run).unwrap();
    api.register(runs::cancel_run).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks */
    api.register(task_executions::list_task_executions).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id} */
    api.register(task_executions::get_task_execution).unwrap();
    api.register(task_executions::cancel_task_execution)
        .unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/logs */
    api.register(task_executions::get_logs).unwrap();
    api.register(task_executions::delete_logs).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/tasks/{task_id}/attach */
    api.register(task_executions::attach_task_execution)
        .unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects */
    api.register(objects::list_pipeline_objects).unwrap();
    api.register(objects::put_pipeline_object).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/objects/{key} */
    api.register(objects::get_pipeline_object).unwrap();
    api.register(objects::delete_pipeline_object).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects */
    api.register(objects::list_run_objects).unwrap();
    api.register(objects::put_run_object).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/runs/{run_id}/objects/{key} */
    api.register(objects::get_run_object).unwrap();
    api.register(objects::delete_run_object).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions */
    api.register(subscriptions::list_subscriptions).unwrap();
    api.register(subscriptions::create_subscription).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/subscriptions/{extension_id}/{subscription_id} */
    api.register(subscriptions::get_subscription).unwrap();
    api.register(subscriptions::update_subscription).unwrap();
    api.register(subscriptions::delete_subscription).unwrap();

    /* /api/tokens */
    api.register(tokens::list_tokens).unwrap();
    api.register(tokens::create_token).unwrap();

    /* /api/tokens/{id} */
    api.register(tokens::get_token_by_id).unwrap();
    api.register(tokens::update_token).unwrap();
    api.register(tokens::delete_token).unwrap();

    /* /api/tokens/bootstrap */
    api.register(tokens::create_bootstrap_token).unwrap();

    /* /api/tokens/whoami */
    api.register(tokens::whoami).unwrap();
    api.register(tokens::create_web_login).unwrap();
    api.register(tokens::exchange_web_login).unwrap();

    /* /api/extensions */
    api.register(extensions::list_extensions).unwrap();

    /* /api/extensions/plan */
    api.register(extensions::reconcile::plan_extensions)
        .unwrap();

    /* /api/extensions/{extension_id} */
    api.register(extensions::get_extension).unwrap();
    api.register(extensions::purge_extension).unwrap();

    /* /api/extensions/{extension_id}/apply */
    api.register(extensions::reconcile::apply_extension)
        .unwrap();

    /* /api/extensions/{extension_id}/revert */
    api.register(extensions::reconcile::revert_extension)
        .unwrap();

    /* /api/extensions/{extension_id}/logs */
    api.register(extensions::get_extension_logs).unwrap();

    /* /api/extensions/{extension_id}/debug */
    api.register(extensions::get_extension_debug_info).unwrap();

    /* /api/extensions/{extension_id}/objects */
    api.register(objects::list_extension_objects).unwrap();
    api.register(objects::put_extension_object).unwrap();

    /* /api/extensions/{extension_id}/objects/{key} */
    api.register(objects::get_extension_object).unwrap();
    api.register(objects::delete_extension_object).unwrap();

    /* /api/extensions/{extension_id}/subscriptions */
    api.register(extensions::list_extension_subscriptions)
        .unwrap();

    /* /api/secrets/global */
    api.register(secrets::list_global_secrets).unwrap();
    api.register(secrets::put_global_secret).unwrap();

    /* /api/secrets/global/{key} */
    api.register(secrets::get_global_secret).unwrap();
    api.register(secrets::delete_global_secret).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets */
    api.register(secrets::list_pipeline_secrets).unwrap();
    api.register(secrets::put_pipeline_secret).unwrap();

    /* /api/namespaces/{namespace_id}/pipelines/{pipeline_id}/secrets/{key} */
    api.register(secrets::get_pipeline_secret).unwrap();
    api.register(secrets::delete_pipeline_secret).unwrap();

    /* /api/events */
    api.register(events::stream_events).unwrap();

    /* /api/events/{event_id} */
    api.register(events::get_event).unwrap();
    api.register(events::delete_event).unwrap();

    /* /api/system */
    api.register(system::get_system_preferences).unwrap();
    api.register(system::update_system_preferences).unwrap();

    /* /api/system/metadata */
    api.register(system::get_system_metadata).unwrap();

    /* /api/roles */
    api.register(permissioning::list_roles).unwrap();
    api.register(permissioning::create_role).unwrap();

    /* /api/roles/{role_id} */
    api.register(permissioning::get_role).unwrap();
    api.register(permissioning::delete_role).unwrap();
    api.register(permissioning::update_role).unwrap();

    // /docs/*
    api.register(static_router::static_documentation_handler)
        .unwrap();

    // /extensions/manifests/{name}
    api.register(static_router::default_manifest_handler)
        .unwrap();

    // /
    api.register(static_router::static_handler).unwrap();
}

/// Config OpenAPI tagging policies and description
fn set_tagging_policy(api: ApiDescription<Arc<ApiState>>) -> ApiDescription<Arc<ApiState>> {
    api.tag_config(TagConfig {
        allow_other_tags: false,
        policy: EndpointTagPolicy::ExactlyOne,
        tags: vec![
            (
                "Configs".to_string(),
                TagDetails {
                    description: Some("Pipeline configs are versioned configurations for a particular pipeline.".into()),
                    ..Default::default()
                },
            ),
            (
                "Deployments".to_string(),
                TagDetails {
                    description: Some("A deployment represents a transition between pipeline versions".into()),
                    ..Default::default()
                },
            ),
            (
                "Extensions".to_string(),
                TagDetails {
                    description: Some("An extension is a way to give pipelines more functionality. This might include \
                    automatically running your pipeline or printing the results of a run to Slack or more. Pipelines \
                    can subscribe to one or more extensions (usually with some individual configuration) and those \
                    extensions perform actions on behalf of the pipeline.".into()),
                    ..Default::default()
                },
            ),
            (
                "Events".to_string(),
                TagDetails {
                    description: Some("Gofer emits events for actions that happen within it's purview. You can use \
                    the event api to get a list of all events or request specific events.".into()),
                    ..Default::default()
                },
            ),
            (
                "Namespaces".to_string(),
                TagDetails {
                    description: Some("A namespace represents a grouping of pipelines. Normally it is used to divide \
                    teams or logically different sections of workloads. It is the highest level unit as it sits above \
                    pipelines in the hierarchy of Gofer".into()),
                    ..Default::default()
                },
            ),
            (
                "Pipelines".to_string(),
                TagDetails {
                    description: Some("A pipeline is a graph of containers that accomplish some goal. Pipelines are \
                    created via a Pipeline configuration file and can be set to be run automatically via attached \
                    extensions".into()),
                    ..Default::default()
                },
            ),
            (
                "Permissions".to_string(),
                TagDetails {
                    description: Some("Gofer has an RBAC system which can be utilized to give different tokens/users
                        permissions.".into()),
                    ..Default::default()
                },
            ),
            (
                "Runs".to_string(),
                TagDetails {
                    description: Some("A run is a specific execution of a pipeline at a specific point in time. A run \
                    is made up of multiple tasks that all execute according to their dependency on each other.".into()),
                    ..Default::default()
                },
            ),
            (
                "Tasks".to_string(),
                TagDetails {
                    description: Some("A task is the lowest unit of execution for a pipeline. A task execution is the \
                    tracking of a task, which is to say a task execution is simply the tracking of the container that \
                    is in the act of being executed.".into()),
                    ..Default::default()
                },
            ),
            (
                "Secrets".to_string(),
                TagDetails {
                    description: Some("Gofer allows user to enter secrets on both a global and pipeline scope. This \
                    is useful for workloads that need access to secret values and want a quick, convenient way to \
                    access those secrets. Global secrets are managed by admins and can grant pipelines access to secrets
                    shared amongst many namespaces. Pipeline secrets on the other hand are only accessible from within
                    that specific pipeline".into()),
                    ..Default::default()
                },
            ),
            (
                "Objects".to_string(),
                TagDetails {
                    description: Some("The object store is a temporary key-vale storage mechanism for pipelines and \
                    runs. It allows the user to cache objects for the lifetime of multiple runs or for the lifetime of \
                    a single run.\
                    There are two separate types of objects, each useful for its own use case. Visit the documentation
                    for more details on the associated lifetimes of pipeline specific and run specific objects".into()),
                    ..Default::default()
                },
            ),
            (
                "Subscriptions".to_string(),
                TagDetails {
                    description: Some("A subscription represents a pipeline's subscription to a extension.".into()),
                    ..Default::default()
                },
            ),
            (
                "System".to_string(),
                TagDetails {
                    description: Some("Routes focused on meta-information for the Gofer service".into()),
                    ..Default::default()
                },
            ),
            (
                "Tokens".to_string(),
                TagDetails {
                    description: Some("Gofer API Token".to_string()),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect(),
    })
}

/// Identifiers are used as the primary key in most of gofer's resources.
/// They're defined by the user and therefore should have some sane bounds.
/// For all ids we'll want the following:
/// * 32 > characters < 3
/// * Only alphanumeric characters or hyphens
///
/// We don't allow underscores to conform with common practices for url safe strings.
pub fn is_valid_identifier(id: &str) -> Result<()> {
    let alphanumeric_w_hyphen = regex!("^[a-zA-Z0-9-]*$");

    if id.len() > 32 {
        bail!("length cannot be greater than 32");
    }

    if id.len() < 3 {
        bail!("length cannot be less than 3");
    }

    if !alphanumeric_w_hyphen.is_match(id) {
        bail!("can only be made up of alphanumeric and hyphen characters");
    }

    Ok(())
}

/// Returns a 404 if the namespace doesn't exist.
///
/// Routes that work on a resource's children call these first so that a mistyped parent gets a clear "doesn't
/// exist" instead of an empty list or a foreign key failure turned into a 500.
pub async fn ensure_namespace_exists(
    conn: &mut sqlx::SqliteConnection,
    request_id: &str,
    namespace_id: &str,
) -> Result<(), HttpError> {
    match storage::namespaces::get(conn, namespace_id).await {
        Ok(_) => Ok(()),
        Err(storage::StorageError::NotFound) => Err(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::NOT_FOUND,
            format!("namespace '{namespace_id}' does not exist"),
        )),
        Err(e) => Err(crate::http_error!(
            "Could not get namespace from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            request_id.to_string(),
            Some(e.into())
        )),
    }
}

/// Returns a 404 naming whichever of the namespace or pipeline doesn't exist.
pub async fn ensure_pipeline_exists(
    conn: &mut sqlx::SqliteConnection,
    request_id: &str,
    namespace_id: &str,
    pipeline_id: &str,
) -> Result<(), HttpError> {
    match storage::pipeline_metadata::get(conn, namespace_id, pipeline_id).await {
        Ok(_) => Ok(()),
        Err(storage::StorageError::NotFound) => {
            // Only worth the extra query when something is missing, so the user knows which part they mistyped.
            ensure_namespace_exists(conn, request_id, namespace_id).await?;
            Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::NOT_FOUND,
                format!("pipeline '{pipeline_id}' does not exist in namespace '{namespace_id}'"),
            ))
        }
        Err(e) => Err(crate::http_error!(
            "Could not get pipeline from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            request_id.to_string(),
            Some(e.into())
        )),
    }
}

/// Returns a 404 naming whichever of the namespace, pipeline, or run doesn't exist.
pub async fn ensure_run_exists(
    conn: &mut sqlx::SqliteConnection,
    request_id: &str,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: u64,
) -> Result<(), HttpError> {
    let Ok(storage_run_id) = i64::try_from(run_id) else {
        return Err(HttpError::for_bad_request(
            None,
            format!("run id '{run_id}' is too large"),
        ));
    };

    match storage::runs::get(conn, namespace_id, pipeline_id, storage_run_id).await {
        Ok(_) => Ok(()),
        Err(storage::StorageError::NotFound) => {
            ensure_pipeline_exists(conn, request_id, namespace_id, pipeline_id).await?;
            Err(HttpError::for_client_error(
                None,
                ClientErrorStatusCode::NOT_FOUND,
                format!("run '{run_id}' does not exist for pipeline '{pipeline_id}'"),
            ))
        }
        Err(e) => Err(crate::http_error!(
            "Could not get run from database",
            hyper::StatusCode::INTERNAL_SERVER_ERROR,
            request_id.to_string(),
            Some(e.into())
        )),
    }
}

/// Authentication information for container registries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct RegistryAuth {
    pub user: String,
    pub pass: String,
}

impl From<gofer_sdk::config::RegistryAuth> for RegistryAuth {
    fn from(value: gofer_sdk::config::RegistryAuth) -> Self {
        RegistryAuth {
            user: value.user,
            pass: value.pass,
        }
    }
}

/// Stands in for credentials in API responses. Whoever set a credential already has it, and anyone else with read
/// access to the object (a teammate's token, a custom role) shouldn't be able to pull it back out. Admins can still
/// see real values by asking for them; see [`include_secrets`].
///
/// This is applied when building responses rather than with serde attributes because these same types are
/// serialized into the database, where the real values have to survive.
pub const REDACTED: &str = "[redacted]";

/// Query args for routes that redact credentials. Admins can ask for the real values when they need to debug,
/// mirroring `include_secret` on the secret store routes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IncludeSecretQueryArgs {
    /// Return credentials in plaintext instead of redacted. Admin only.
    pub include_secret: Option<bool>,
}

/// Whether a response should carry real credentials. It has to be asked for explicitly so a routine listing never
/// puts a key on someone's screen, and asking without being an admin is an error rather than a silent redaction so
/// the caller knows why they aren't seeing values.
pub fn include_secrets(
    req_metadata: &RequestMetadata,
    query: &IncludeSecretQueryArgs,
) -> Result<bool, HttpError> {
    if !query.include_secret.unwrap_or_default() {
        return Ok(false);
    }

    if !req_metadata.admin {
        return Err(HttpError::for_client_error(
            None,
            ClientErrorStatusCode::FORBIDDEN,
            "'include_secret' requires an admin token".into(),
        ));
    }

    Ok(true)
}

impl RegistryAuth {
    pub fn redacted(self) -> Self {
        RegistryAuth {
            user: self.user,
            pass: REDACTED.into(),
        }
    }
}

#[derive(
    Debug, Clone, Display, Default, PartialEq, EnumString, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum VariableSource {
    #[default]
    Unknown,

    /// From the user's own pipeline configuration.
    PipelineConfig,

    /// From the Gofer API executor itself.
    System,

    /// Injected at the beginning of a particular run.
    RunOptions,

    /// Injected by a subscribed extension.
    Extension,
}

/// A variable is a key value pair that is used either at a run or task level.
/// The variable is inserted as an environment variable to an eventual task execution.
/// It can be owned by different parts of the system which control where the potentially
/// sensitive variables might show up.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct Variable {
    pub key: String,
    pub value: String,
    pub source: VariableSource,
}

/// Convenience function for the composite key for the in_progress_run mapping in [`ApiState`].
fn in_progress_runs_key(namespace_id: &str, pipeline_id: &str) -> String {
    format!("{}_{}", namespace_id, pipeline_id)
}

/// Return the current epoch time in milliseconds.
pub fn epoch_milli() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Gofer allows users to enter special interpolation strings such that
/// special functionality is substituted when Gofer reads these strings
/// in a user's pipeline configuration.
#[derive(Debug, Display, EnumString, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum(ascii_case_insensitive)]
pub enum InterpolationKind {
    Unknown,

    /// pipeline_secret{{\<key\>]}}
    PipelineSecret,

    /// global_secret{{\<key\>]}}
    GlobalSecret,

    /// run_object{{\<key\>}}
    RunObject,

    /// pipeline_object{{\<key\>}}
    PipelineObject,
}

/// Looks up the value of a global secret.
///
/// Pipelines pass the namespace they're running in, and the secret has to allow that namespace. Extensions pass
/// `None` since they don't belong to a namespace; that skips the check, which is fine because only admins can
/// install extensions and only admins can manage global secrets.
pub async fn fetch_global_secret(
    storage: &storage::Db,
    secret_store: &dyn secret_store::SecretStore,
    key: &str,
    namespace_id: Option<&str>,
) -> Result<String> {
    let mut conn = storage
        .read_conn()
        .await
        .map_err(|e| anyhow!("Could not establish a connection to the database; {:#?}", e))?;

    let key_metadata = match storage::secret_store_global_keys::get(&mut conn, key).await {
        Ok(val) => val,
        Err(storage::StorageError::NotFound) => bail!("Could not find global secret '{key}'"),
        Err(e) => bail!("Could not retrieve global secret '{key}'; {:#?}", e),
    };

    let key_metadata: secrets::Secret = key_metadata
        .try_into()
        .map_err(|e| anyhow!("Could not parse global secret '{key}'; {:#?}", e))?;

    if let Some(namespace_id) = namespace_id
        && !key_metadata.is_allowed_namespace(namespace_id)
    {
        bail!(
            "Global secret {} cannot be used in this current namespace. Valid namespaces: {:#?}",
            key_metadata.key,
            key_metadata.namespaces
        )
    }

    let value = match secret_store
        .get(&secrets::global_secret_store_key(&key_metadata.key))
        .await
    {
        Ok(val) => val,
        Err(secret_store::SecretStoreError::NotFound) => {
            bail!("Could not find global secret '{}'", key_metadata.key)
        }
        Err(e) => bail!("Could not retrieve global secret: {:#?}", e),
    };

    Ok(String::from_utf8_lossy(&value.0).to_string())
}

/// Gofer allows users to use secrets and objects from it's built-in sources. To facilitate this the user
/// simply includes a special string in into special places within the Gofer pipeline manifest(for now this is only
/// the "variables" field within a pipeline's tasks or a run). These special strings are decoded here.
///
/// Takes in a map of mixed plaintext and raw secret/store strings and populates it with
/// the fetched strings for each type.
///
/// The 'run_id' is optional here since we mainly use interpolate_vars in two separate contexts. The first context
/// is when we process a new run, in which case there might be some run specific vars that need to be interpolated.
/// The second is during pipeline subscriptions in which case you might want to pass a secret, but we aren't in the
/// context of a run and don't require it.
///
/// This takes the stores it reads from instead of [`ApiState`] so it can be tested without a scheduler.
pub async fn interpolate_vars(
    storage: &storage::Db,
    secret_store: &dyn secret_store::SecretStore,
    object_store: &dyn object_store::ObjectStore,
    namespace_id: &str,
    pipeline_id: &str,
    run_id: Option<u64>,
    variables: &Vec<Variable>,
) -> Result<Vec<Variable>> {
    let mut variable_list = vec![];

    for variable in variables {
        // If its not an interpolated var we simply just add it to the vars and move on to the next one.
        let (interpolation_kind, value) = match parse_interpolation_syntax(&variable.value) {
            Some((k, v)) => (k, v),
            None => {
                variable_list.push(variable.to_owned());
                continue;
            }
        };

        match interpolation_kind {
            InterpolationKind::Unknown => {
                bail!("Encountered error during variable interpolation; Interpolation kind unknown")
            }
            InterpolationKind::PipelineSecret => {
                let value = match secret_store
                    .get(&secrets::pipeline_secret_store_key(
                        namespace_id,
                        pipeline_id,
                        &value,
                    ))
                    .await
                {
                    Ok(val) => String::from_utf8_lossy(&val.0).to_string(),
                    Err(e) => match e {
                        secret_store::SecretStoreError::NotFound => {
                            bail!("Could not find pipeline secret '{}'", value);
                        }
                        _ => {
                            bail!(
                                "Encountered error while attempting to retrieve pipeline during interpolation {:#?}",
                                e
                            );
                        }
                    },
                };

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value,
                    source: variable.source.clone(),
                });
            }
            InterpolationKind::GlobalSecret => {
                let value = fetch_global_secret(storage, secret_store, &value, Some(namespace_id)).await?;

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value,
                    source: variable.source.clone(),
                });
            }
            InterpolationKind::PipelineObject => {
                let retrieved_value = match object_store
                    .get(&objects::pipeline_object_store_key(
                        namespace_id,
                        pipeline_id,
                        &value,
                    ))
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        if e == object_store::ObjectStoreError::NotFound {
                            bail!("Could not find pipeline object '{}'", value)
                        };

                        bail!("Could not retrieve pipeline object: {:#?}", e)
                    }
                };

                // We attempt to stringify the object to insert it into the environment variables.
                let stringified_object = String::from_utf8_lossy(&retrieved_value);

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value: stringified_object.to_string(),
                    source: variable.source.clone(),
                });
            }
            InterpolationKind::RunObject => {
                if run_id.is_none() {
                    continue;
                }

                let retrieved_value = match object_store
                    .get(&objects::run_object_store_key(
                        namespace_id,
                        pipeline_id,
                        run_id.unwrap(),
                        &value,
                    ))
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        if e == object_store::ObjectStoreError::NotFound {
                            bail!("Could not find run object '{}'", value)
                        };

                        bail!("Could not retrieve run object: {:#?}", e)
                    }
                };

                // We attempt to stringify the object to insert it into the environment variables.
                let stringified_object = String::from_utf8_lossy(&retrieved_value);

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value: stringified_object.to_string(),
                    source: variable.source.clone(),
                });
            }
        };
    }

    Ok(variable_list)
}

/// Checks a string for the existence of an interpolation format. ex: "pipeline_secret{{ example }}".
/// If an interpolation was found we return Some, if not(the string was just a plain string) we return None.
///
/// Within the Some type is the kind of interpolation that was found and secondly the value found within.
///
/// Currently the supported interpolation syntaxes are:
///   - `pipeline_secret{{ example }}` for inserting from the pipeline secret store.
///   - `global_secret{{ example }}` for inserting from the global secret store.
///   - `pipeline_object{{ example }}` for inserting from the pipeline object store.
///   - `run_object{{ example }}` for inserting from the run object store.
pub fn parse_interpolation_syntax(raw_input: &str) -> Option<(InterpolationKind, String)> {
    let raw_input = raw_input.trim();

    let bracket_index = raw_input.find("{{")?;

    let interpolation_name_str = &raw_input[..bracket_index];
    let interpolation_kind = match InterpolationKind::from_str(interpolation_name_str) {
        Ok(kind) => kind,
        Err(_) => return None,
    };

    let kind_str = interpolation_kind.to_string().to_lowercase();
    let interpolation_prefix = format!("{}{{{{", kind_str); // emits e.g. "pipeline_secret{{"
    let interpolation_suffix = "}}";

    // Check prefix in a case-insensitive way
    if raw_input.len() > interpolation_prefix.len() + 2
        && raw_input[..interpolation_prefix.len()].to_lowercase() == interpolation_prefix
        && raw_input.ends_with(interpolation_suffix)
    {
        let content = &raw_input[interpolation_prefix.len()..raw_input.len() - 2];
        return Some((interpolation_kind, content.trim().to_string()));
    }

    None
}

// Function to truncate a string to fit within a specified byte limit
fn truncate_to_utf8_bytes(s: &str, max_bytes: usize) -> String {
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.as_millis();
    let micros = duration.as_micros();

    if secs > 0 {
        format!("{}s", secs)
    } else if millis > 0 {
        format!("{}ms", millis)
    } else if micros > 0 {
        format!("{}μs", micros)
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

#[derive(Debug)]
struct Middleware;

#[async_trait::async_trait]
impl<C: ServerContext> dropshot::Middleware<C> for Middleware {
    async fn handle(
        &self,
        server: Arc<DropshotState<C>>,
        request: hyper::Request<hyper::body::Incoming>,
        request_id: String,
        remote_addr: SocketAddr,
        next: fn(
            Arc<DropshotState<C>>,
            hyper::Request<hyper::body::Incoming>,
            String,
            SocketAddr,
        ) -> Pin<
            Box<dyn Future<Output = Result<hyper::Response<Body>, HandlerError>> + Send>,
        >,
    ) -> Result<hyper::Response<Body>, HandlerError> {
        let start_time = std::time::Instant::now();

        let method = request.method().as_str().to_string();
        let uri = request.uri().to_string();

        // If we're behind a reverse proxy we want the "X-Forwarded-For" header since that gives us the caller's external
        // ip. If not just log whatever the default remote address is.
        let remote_ip = match request.headers().get("X-Forwarded-For") {
            Some(value) => value
                .to_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| remote_addr.to_string()),
            None => remote_addr.to_string(),
        };

        let response = next(server.clone(), request, request_id.clone(), remote_addr).await;

        if let Ok(response) = &response {
            info!(
                remote_addr = remote_ip,
                req_id = request_id,
                method = method,
                uri = uri,
                response_code = response.status().as_str(),
                latency = format_duration(start_time.elapsed()),
                "request completed"
            );
        }

        response
    }
}

/// Returns an HttpError while logging pertinent information, meant to be used as a general error handler for route
/// handlers.
///
/// * Message given is provided to the user as the error message.
/// * Error and Context are passed to the logger for more information internally.
fn _http_error(
    message: String,
    code: hyper::StatusCode,
    request_id: String,
    context: HashMap<String, String>,
    err: Option<Box<dyn std::error::Error>>,
) -> HttpError {
    // We log the error first.
    if let Some(ref e) = err {
        error!(message = message, request_id, error = %e, context = ?context);
    } else {
        error!(message = message, request_id, context = ?context);
    }

    HttpError {
        status_code: ErrorStatusCode::from_status(code).unwrap(),
        error_code: None,
        external_message: format!("{}: {}", code.canonical_reason().unwrap(), message),
        internal_message: message,
        headers: None,
    }
}

/// Returns an HttpError while logging pertinent information, meant to be used as a general error handler for route
/// handlers.
///
/// Wraps the underlying concrete function [`_http_error`] (which you can use to see what parameters the macro requires)
///
/// * Message given is provided to the user as the error message.
/// * Error and Context are passed to the logger for more information internally.
#[macro_export]
macro_rules! http_error {
    ($message:expr_2021, $code:expr_2021, $req_id:expr_2021, $error:expr_2021 $(, $key:ident = $value:expr_2021)*) => {{
        #[allow(unused_mut)]
        let mut context = std::collections::HashMap::new();
        $(
            context.insert(stringify!($key).to_string(), $value.to_string());
        )*

        $crate::api::_http_error(
            $message.to_string(),
            $code,
            $req_id,
            context,
            $error
        )
    }};
}

async fn websocket_error(
    message: &str,
    code: CloseCode,
    request_id: String,
    mut conn: WebSocketStream<WebsocketConnectionRaw>,
    err: Option<String>,
) -> String {
    if let Some(ref e) = err {
        error!(message = message, request_id, error = %e);
    }

    let _ = conn
        .close(Some(CloseFrame {
            code,
            reason: truncate_to_utf8_bytes(message, 123).into(), // Control frames can only be 125 bytes long (-2 for code)
        }))
        .await;

    message.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pipeline_secret() {
        let input = "pipeline_secret{{ example_key }}";
        let expected = Some((InterpolationKind::PipelineSecret, "example_key".to_string()));
        assert_eq!(parse_interpolation_syntax(input), expected);
    }

    #[test]
    fn test_valid_global_secret_with_whitespace() {
        let input = " global_secret{{ another_value }} ";
        let expected = Some((InterpolationKind::GlobalSecret, "another_value".to_string()));
        assert_eq!(parse_interpolation_syntax(input), expected);
    }

    #[test]
    fn test_valid_pipeline_object_nested_whitespace() {
        let input = "pipeline_object{{  nested   }}";
        let expected = Some((InterpolationKind::PipelineObject, "nested".to_string()));
        assert_eq!(parse_interpolation_syntax(input), expected);
    }

    #[test]
    fn test_valid_run_object() {
        let input = "run_object{{run123}}";
        let expected = Some((InterpolationKind::RunObject, "run123".to_string()));
        assert_eq!(parse_interpolation_syntax(input), expected);
    }

    #[test]
    fn test_invalid_missing_braces() {
        let input = "pipeline_secret example";
        assert_eq!(parse_interpolation_syntax(input), None);
    }

    #[test]
    fn test_invalid_unknown_prefix() {
        let input = "foobar{{ some_value }}";
        assert_eq!(parse_interpolation_syntax(input), None);
    }

    #[test]
    fn test_invalid_unclosed_braces() {
        let input = "pipeline_secret{{ value }";
        assert_eq!(parse_interpolation_syntax(input), None);
    }

    #[test]
    fn test_invalid_malformed_string() {
        let input = "pipeline_secret value }}";
        assert_eq!(parse_interpolation_syntax(input), None);
    }

    #[test]
    fn test_plain_string() {
        let input = "just_a_normal_string";
        assert_eq!(parse_interpolation_syntax(input), None);
    }

    #[test]
    fn test_case_insensitive_prefix() {
        let input = "PIPELINE_SECRET{{ uppercase_key }}";
        let expected = Some((
            InterpolationKind::PipelineSecret,
            "uppercase_key".to_string(),
        ));
        assert_eq!(parse_interpolation_syntax(input), expected);
    }

    fn variable(key: &str, value: &str) -> Variable {
        Variable {
            key: key.into(),
            value: value.into(),
            source: VariableSource::PipelineConfig,
        }
    }

    // Users pick their variable names and their secret/object keys separately, so each value has to be looked up
    // by the key inside the braces and never by the variable's name. The names here purposely don't match the keys.
    #[tokio::test]
    async fn test_interpolate_vars_looks_up_keys_inside_braces() {
        use crate::object_store::ObjectStore;
        use crate::secret_store::SecretStore;

        let storage = storage::tests::TestHarness::new().await;
        let secret_store = secret_store::sqlite::tests::TestHarness::new().await;
        let object_store = object_store::filesystem::tests::TestHarness::new().await;

        secret_store
            .put(
                &secrets::pipeline_secret_store_key("default", "simple", "deploy_key"),
                b"pipeline secret value".to_vec(),
                false,
            )
            .await
            .unwrap();

        let global_secret = secrets::Secret::new("slack_token", vec!["default".into()]);
        let mut conn = storage.write_conn().await.unwrap();
        storage::secret_store_global_keys::insert(&mut conn, &global_secret.try_into().unwrap())
            .await
            .unwrap();
        drop(conn);
        secret_store
            .put(
                &secrets::global_secret_store_key("slack_token"),
                b"global secret value".to_vec(),
                false,
            )
            .await
            .unwrap();

        object_store
            .put(
                &objects::pipeline_object_store_key("default", "simple", "logs_header"),
                bytes::Bytes::from("pipeline object value"),
                false,
            )
            .await
            .unwrap();
        object_store
            .put(
                &objects::run_object_store_key("default", "simple", 1, "build_number"),
                bytes::Bytes::from("run object value"),
                false,
            )
            .await
            .unwrap();

        let variables = vec![
            variable("PLAIN", "plain value"),
            variable("DEPLOY_KEY", "pipeline_secret{{deploy_key}}"),
            variable("SLACK", "global_secret{{slack_token}}"),
            variable("HEADER", "pipeline_object{{logs_header}}"),
            variable("BUILD", "run_object{{build_number}}"),
        ];

        let result = interpolate_vars(
            &storage,
            &secret_store.db,
            &object_store.db,
            "default",
            "simple",
            Some(1),
            &variables,
        )
        .await
        .unwrap();

        let result: HashMap<String, String> =
            result.into_iter().map(|v| (v.key, v.value)).collect();

        assert_eq!(result["PLAIN"], "plain value");
        assert_eq!(result["DEPLOY_KEY"], "pipeline secret value");
        assert_eq!(result["SLACK"], "global secret value");
        assert_eq!(result["HEADER"], "pipeline object value");
        assert_eq!(result["BUILD"], "run object value");
    }

    #[tokio::test]
    async fn test_interpolate_vars_missing_key_names_the_key() {
        let storage = storage::tests::TestHarness::new().await;
        let secret_store = secret_store::sqlite::tests::TestHarness::new().await;
        let object_store = object_store::filesystem::tests::TestHarness::new().await;

        let err = interpolate_vars(
            &storage,
            &secret_store.db,
            &object_store.db,
            "default",
            "simple",
            Some(1),
            &vec![variable("HEADER", "pipeline_object{{logs_header}}")],
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("logs_header"), "{err}");
    }

    #[tokio::test]
    async fn test_interpolate_vars_global_secret_wrong_namespace() {
        use crate::secret_store::SecretStore;

        let storage = storage::tests::TestHarness::new().await;
        let secret_store = secret_store::sqlite::tests::TestHarness::new().await;
        let object_store = object_store::filesystem::tests::TestHarness::new().await;

        let global_secret = secrets::Secret::new("slack_token", vec!["ops".into()]);
        let mut conn = storage.write_conn().await.unwrap();
        storage::secret_store_global_keys::insert(&mut conn, &global_secret.try_into().unwrap())
            .await
            .unwrap();
        drop(conn);
        secret_store
            .put(
                &secrets::global_secret_store_key("slack_token"),
                b"global secret value".to_vec(),
                false,
            )
            .await
            .unwrap();

        let result = interpolate_vars(
            &storage,
            &secret_store.db,
            &object_store.db,
            "default",
            "simple",
            Some(1),
            &vec![variable("SLACK", "global_secret{{slack_token}}")],
        )
        .await;

        assert!(result.is_err());
    }
}
