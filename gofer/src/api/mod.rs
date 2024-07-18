//! The REST API package for Gofer; manages the API models and handlers.

mod deployments;
mod event_utils;
mod events;
pub mod extensions;
mod external;
mod namespaces;
mod objects;
mod pipeline_configs;
mod pipelines;
mod run_utils;
mod runs;
mod secrets;
mod static_router;
mod subscriptions;
mod system;
pub mod task_executions;
mod tasks;
mod tokens;

use crate::{conf, object_store, scheduler, secret_store, storage};
use anyhow::{anyhow, bail, Context, Result};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use dashmap::DashMap;
use dropshot::{
    ApiDescription, ConfigDropshot, ConfigTls, DropshotState, EndpointTagPolicy, HandlerTaskMode,
    HttpError, HttpServer, HttpServerStarter, RequestInfo, ServerContext, TagConfig, TagDetails,
};
use futures::Future;
use http::Request;
use http::StatusCode;
use hyper::upgrade::Upgraded;
use hyper::Body;
use lazy_regex::regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use std::{net::SocketAddr, pin::Pin, str::FromStr, sync::atomic, sync::Arc};
use strum::{Display, EnumString};
use tokio::signal;
use tokio_tungstenite::WebSocketStream;
use tracing::{error, info, warn};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};

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

/// Contains information about the auth token sent with a request.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The namespaces this key is allowed to access. The wildcard character '*' means all namespaces.
    pub allowed_namespaces: HashSet<String>,

    /// The unique identifier for the api token the current user is using.
    pub key_id: String,

    /// What type of api key the current request is made with.
    pub key_type: tokens::TokenType,
}

impl AuthContext {
    /// Verify that the namespace you're currently working within is allowed for that specific token.
    fn verify_namespace_match(&self, namespace: &str) -> Result<(), HttpError> {
        // If the allowed namspaces are anything then just let them through.
        if self.allowed_namespaces.contains(".*") {
            return Ok(());
        };

        for allowed_namespace in &self.allowed_namespaces {
            let re = match regex::Regex::new(allowed_namespace) {
                Ok(re) => re,
                Err(_) => continue,
            };

            if re.is_match(namespace) {
                return Ok(());
            }
        }

        Err(HttpError::for_client_error(
            None,
            StatusCode::UNAUTHORIZED,
            "Unauthorized; Token has incorrect namespaces for targeted namespace".into(),
        ))
    }
}

/// Holds objects that are created and used over the lifetime of a single request.
///
/// This is different from [`dropshot::RequestContext`] since that is automatically created for us but we need some
/// more Gofer specific information.
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    #[allow(dead_code)]
    api_version: ApiVersion,
    #[allow(dead_code)]
    auth: AuthContext,
}

#[derive(Debug, Clone)]
struct PreflightOptions {
    bypass_auth: bool,
    check_namespace: Option<String>,
    management_only: bool,
}

/// Holds all objects that need to exist for the entire runtime of the API server.
#[derive(Debug)]
pub struct ApiState {
    /// The API configuration read in at init.
    config: conf::api::ApiConfig,

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

    /// `Scheduler` is the mechanism in which Gofer uses to run its containers(tasks).
    scheduler: Box<dyn scheduler::Scheduler>,

    /// ObjectStore is the mechanism in which Gofer stores pipeline and run level objects. The implementation here
    /// is meant to act as a basic object store that Gofer's connections can use freely.
    object_store: Box<dyn object_store::ObjectStore>,

    /// SecretStore is the mechanism in which Gofer manages pipeline secrets.
    secret_store: Box<dyn secret_store::SecretStore>,
}

impl ApiState {
    fn new(
        conf: conf::api::ApiConfig,
        storage: storage::Db,
        scheduler: Box<dyn scheduler::Scheduler>,
        event_bus: event_utils::EventBus,
        object_store: Box<dyn object_store::ObjectStore>,
        secret_store: Box<dyn secret_store::SecretStore>,
    ) -> Self {
        Self {
            config: conf.clone(),
            extensions: DashMap::new(),
            event_bus,
            in_progress_runs: DashMap::new(),
            ignore_pipeline_run_events: atomic::AtomicBool::new(
                conf.api.ignore_pipeline_run_events,
            ),
            storage,
            scheduler,
            object_store,
            secret_store,
        }
    }

    /// Resolves request specific context for handlers. This is used to perform auth checks and generally other
    /// actions that should happen before a route runs it's handler.
    ///
    /// **Should be called at the start of every handler**, regardless of if that handler needs auth or req_context.
    ///
    /// We specifically use a struct here so that the reader can easily verify which options are in which state.
    /// The different options here map to different actions that are checked per call.
    async fn run_preflight(
        &self,
        request: &RequestInfo,
        options: PreflightOptions,
    ) -> Result<RequestMetadata, HttpError> {
        let mut bypass_auth = options.bypass_auth;

        if self.config.development.bypass_auth {
            bypass_auth = self.config.development.bypass_auth
        }

        // This is somewhat dangerous since we just assume the user is global admin, but since you cannot auth to
        // a different endpoint from this point on I think it's okay.
        let auth_ctx = if bypass_auth {
            AuthContext {
                // allow any namespace
                allowed_namespaces: HashSet::from(["*".into()]),
                key_id: "dev".into(),
                key_type: tokens::TokenType::Management,
            }
        } else {
            self.get_auth_context(request).await?
        };
        let api_version = check_version_handler(request)?;

        if let Some(namespace) = options.check_namespace {
            auth_ctx.verify_namespace_match(&namespace)?;
        }

        if options.management_only && (auth_ctx.key_type != tokens::TokenType::Management) {
            return Err(HttpError::for_client_error(
                None,
                StatusCode::UNAUTHORIZED,
                "Route requires management level token".into(),
            ));
        }

        Ok(RequestMetadata {
            auth: auth_ctx,
            api_version,
        })
    }

    /// Checks request authentication and returns valid auth information.
    async fn get_auth_context(&self, request: &RequestInfo) -> Result<AuthContext, HttpError> {
        let auth_header =
            request
                .headers()
                .get("Authorization")
                .ok_or(HttpError::for_bad_request(
                    None,
                    "Authorization header not found but required".into(),
                ))?;

        let auth_header = auth_header.to_str().map_err(|e| {
            HttpError::for_bad_request(
                None,
                format!("Could not parse Authorization header; {:#?}", e),
            )
        })?;
        if !auth_header.starts_with("Bearer ") {
            return Err(HttpError::for_bad_request(
                None,
                "Authorization header malformed; should start with 'Bearer'".into(),
            ));
        }

        let token = auth_header.strip_prefix("Bearer ").unwrap();

        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let mut conn = match self.storage.conn().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(crate::http_error!(
                    "Could not open connection to database",
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                    "None".into(),
                    Some(e.into())
                ));
            }
        };

        let storage_token = match storage::tokens::get_by_hash(&mut conn, &hash).await {
            Ok(token) => token,
            Err(e) => match e {
                storage::StorageError::NotFound => {
                    return Err(HttpError::for_client_error(
                        None,
                        StatusCode::UNAUTHORIZED,
                        "Unauthorized".into(),
                    ));
                }
                _ => {
                    return Err(crate::http_error!(
                        "Could not query database",
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        "None".into(),
                        Some(e.into())
                    ));
                }
            },
        };

        let token = tokens::Token::try_from(storage_token).map_err(|e| {
            crate::http_error!(
                "Could not parse token object from database",
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "None".into(),
                Some(e.into())
            )
        })?;

        if token.disabled {
            return Err(HttpError::for_client_error(
                None,
                http::StatusCode::UNAUTHORIZED,
                "Token disabled".into(),
            ));
        }

        if epoch_milli() > token.expires {
            return Err(HttpError::for_client_error(
                None,
                http::StatusCode::UNAUTHORIZED,
                "Token expired".into(),
            ));
        }

        Ok(AuthContext {
            allowed_namespaces: token.namespaces,
            key_id: token.id,
            key_type: token.token_type,
        })
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
async fn init_api(conf: conf::api::ApiConfig) -> Result<Arc<ApiState>> {
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
    let secret_store = secret_store::new(&conf.secret_store)
        .await
        .context("Could not initialize secret store")?;
    let event_bus = event_utils::EventBus::new(
        storage.clone(),
        conf.api.event_log_retention,
        conf.api.event_prune_interval,
    );

    let api_state = Arc::new(ApiState::new(
        conf.clone(),
        storage,
        scheduler,
        event_bus,
        object_store,
        secret_store,
    ));

    // Then we perform additional housekeeping.

    extensions::install_std_extensions(api_state.clone())
        .await
        .context("Could not register standard extensions")?;

    extensions::start_extensions(api_state.clone())
        .await
        .context("Could not start extensions")?;

    namespaces::create_default_namespace(api_state.clone())
        .await
        .context("Could not create default namespace")?;

    if let Err(e) = subscriptions::restore_extension_subscriptions(&api_state).await {
        // We report the error but continue with startup. This avoids the issue of a chicken
        // and an egg problem where an extension might have been removed, but the subscription
        // hasn't. And as such if we failed we wouldn't be able to remove the subscription in
        // question. Instead we print a big error in the logs and inform user that they should
        // fix the subscription in question and restart.
        error!(error= %e, "Could not properly restore extension subscriptions; Gofer will continue with startup \
        but you should fix this error before continuing. Normally this means that some extension has been removed \
        but there were previous subscribers that weren't properly removed. Deleting those subscriber will allow \
        for a successful start");
    }

    Ok(api_state)
}

/// Starts both the gofer main api and the external events web service.
pub async fn start_web_services() -> Result<()> {
    let conf = conf::Configuration::<conf::api::ApiConfig>::load(None)
        .context("Could not initialize configuration")?;

    init_logger(&conf.api.log_level, conf.development.pretty_logging)?;

    let api_state = init_api(conf.clone())
        .await
        .context("Could not initialize API")?;

    tokio::spawn(external::start_web_service(conf.clone(), api_state.clone()));

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
            &conf.server.bind_address.clone()
        )
    })?;

    let dropshot_conf = ConfigDropshot {
        bind_address,
        request_body_max_bytes: 524288000, // 500MB to allow for extra large objects.

        // If a client disconnects run the handler to completion still. Eventually we'll want to save resources
        // by allowing the handler to early cancel, but until this is more developed lets just run it to completion.
        default_handler_task_mode: HandlerTaskMode::Detached,
    };

    let api = init_api_description()?;

    let server = if !conf.server.use_tls {
        HttpServerStarter::new(&dropshot_conf, api, Some(Arc::new(Middleware)), api_state)
            .map_err(|error| anyhow!("failed to create server: {}", error))?
            .start()
    } else {
        let (tls_cert, tls_key) = load_tls(
            conf.server.use_tls,
            conf.server.tls_cert_path,
            conf.server.tls_key_path,
        )?;

        let tls_config = Some(ConfigTls::AsBytes {
            certs: tls_cert,
            key: tls_key,
        });

        HttpServerStarter::new_with_tls(
            &dropshot_conf,
            api,
            Some(Arc::new(Middleware)),
            api_state,
            tls_config,
        )
        .map_err(|error| anyhow!("failed to create server: {}", error))?
        .start()
    };
    let shutdown = server.wait_for_shutdown();

    tokio::spawn(wait_for_shutdown_signal(server));

    info!(
        message = "Started Gofer http service",
        host = %bind_address.ip(),
        port = %bind_address.port(),
        tls = conf.server.use_tls,
    );

    shutdown
        .await
        .map_err(|error| anyhow!("Server encountered errors while running; {:#?}", error))
}

/// This is called from another binary to write the openAPI spec to a file.
#[allow(dead_code)]
pub fn write_openapi_spec(path: PathBuf) -> Result<()> {
    let api = init_api_description()?;
    let mut file = std::fs::File::create(path)?;
    api.openapi("Gofer", BUILD_SEMVER).write(&mut file)?;

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

    /* /api/extensions */
    api.register(extensions::list_extensions).unwrap();
    api.register(extensions::install_extension).unwrap();

    /* /api/extensions/{extension_id} */
    api.register(extensions::get_extension).unwrap();
    api.register(extensions::update_extension).unwrap();
    api.register(extensions::uninstall_extension).unwrap();

    /* /api/extensions/{extension_id}/logs */
    api.register(extensions::get_extension_logs).unwrap();

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

    /* /api/system/metadata */
    api.register(system::get_metadata).unwrap();

    // /docs/*
    api.register(static_router::static_documentation_handler)
        .unwrap();

    // /
    api.register(static_router::static_handler).unwrap();
}

/// Config OpenAPI tagging policies and description
fn set_tagging_policy(api: ApiDescription<Arc<ApiState>>) -> ApiDescription<Arc<ApiState>> {
    api.tag_config(TagConfig {
        allow_other_tags: false,
        endpoint_tag_policy: EndpointTagPolicy::ExactlyOne,
        tag_definitions: vec![
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
        bail!("can only be made up of alphanumeric and underscore characters");
    }

    Ok(())
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
#[derive(Debug, Display, EnumString, Deserialize, Serialize)]
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
pub async fn interpolate_vars(
    api_state: &ApiState,
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
            InterpolationKind::Unknown => todo!(),
            InterpolationKind::PipelineSecret => {
                let value = match api_state
                    .secret_store
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
                            bail!("Could not find pipeline secret '{}'", &value);
                        }
                        _ => {
                            bail!("Encountered error while attempting to retrieve pipeline during interpolation {:#?}", e);
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
                let mut conn = match api_state.storage.conn().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        bail!("Could not establish a connection to the database during interpolation; {:#?}", e);
                    }
                };

                let retrieved_key_metadata = match storage::secret_store_global_keys::get(
                    &mut conn, &value,
                )
                .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        bail!("Encountered error while attempting to retrieve global secret during interpolation: {:#?}", e)
                    }
                };

                let key_metadata: secrets::Secret = match retrieved_key_metadata.try_into() {
                    Ok(secret) => secret,
                    Err(e) => {
                        bail!(
                                "Could not serialize retrieved global secret during interpolation: {:#?}", e
                            );
                    }
                };

                if !key_metadata.is_allowed_namespace(namespace_id) {
                    bail!("Global secret {} cannot be used in this current namespace. Valid namespaces: {:#?}",
                        key_metadata.key, key_metadata.namespaces)
                }

                let retrieved_value = match api_state
                    .secret_store
                    .get(&secrets::global_secret_store_key(&key_metadata.key))
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        if e == secret_store::SecretStoreError::NotFound {
                            bail!("Could not find global secret {}", &key_metadata.key)
                        };

                        bail!("Could not retrieve global secret: {:#?}", e)
                    }
                };

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value: String::from_utf8_lossy(&retrieved_value.0).to_string(),
                    source: variable.source.clone(),
                });
            }
            InterpolationKind::PipelineObject => {
                let retrieved_value = match api_state
                    .object_store
                    .get(&objects::pipeline_object_store_key(
                        namespace_id,
                        pipeline_id,
                        &variable.key.clone(),
                    ))
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        if e == object_store::ObjectStoreError::NotFound {
                            bail!("Could not find pipeline object {}", &variable.key.clone(),)
                        };

                        bail!("Could not retrieve pipeline object: {:#?}", e)
                    }
                };
                // We base64 encode the bytes so the user can handle them when they are injected into the environment.
                let base64_string = BASE64_STANDARD.encode(retrieved_value.0);

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value: base64_string,
                    source: variable.source.clone(),
                });
            }
            InterpolationKind::RunObject => {
                if run_id.is_none() {
                    continue;
                }

                let retrieved_value = match api_state
                    .object_store
                    .get(&objects::run_object_store_key(
                        namespace_id,
                        pipeline_id,
                        run_id.unwrap(),
                        &variable.key.clone(),
                    ))
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        if e == object_store::ObjectStoreError::NotFound {
                            bail!("Could not find run object {}", &variable.key.clone(),)
                        };

                        bail!("Could not retrieve run object: {:#?}", e)
                    }
                };
                // We base64 encode the bytes so the user can handle them when they are injected into the environment.
                let base64_string = BASE64_STANDARD.encode(retrieved_value.0);

                variable_list.push(Variable {
                    key: variable.key.clone(),
                    value: base64_string,
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
    let mut raw_input = raw_input.trim();

    let bracket_index = match raw_input.find("{{") {
        Some(index) => index,
        None => return None, // The input did not have the correct syntax so it must not be a string for interpolation.
    };

    let interpolation_name_str = &raw_input[..bracket_index];
    let interpolation_kind = match InterpolationKind::from_str(interpolation_name_str) {
        Ok(kind) => kind,
        Err(_) => return None,
    };

    let interpolation_prefix = format!("{}{{", interpolation_kind.to_string().to_lowercase());
    let interpolation_suffix = "}}";
    if raw_input.starts_with(&interpolation_prefix) && raw_input.ends_with(interpolation_suffix) {
        raw_input = raw_input.strip_prefix(&interpolation_prefix).unwrap();
        raw_input = raw_input.strip_suffix(interpolation_suffix).unwrap();
        return Some((interpolation_kind, raw_input.trim().to_string()));
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
        request: Request<Body>,
        request_id: String,
        remote_addr: SocketAddr,
        next: fn(
            Arc<DropshotState<C>>,
            Request<Body>,
            String,
            SocketAddr,
        )
            -> Pin<Box<dyn Future<Output = Result<hyper::Response<Body>, HttpError>> + Send>>,
    ) -> Result<http::Response<Body>, HttpError> {
        let start_time = std::time::Instant::now();

        let method = request.method().as_str().to_string();
        let uri = request.uri().to_string();

        let response = next(server.clone(), request, request_id.clone(), remote_addr).await;

        match &response {
            Ok(response) => {
                info!(
                    remote_addr = %remote_addr,
                    req_id = request_id,
                    method = method,
                    uri = uri,
                    response_code = response.status().as_str(),
                    latency = format_duration(start_time.elapsed()),
                    "request completed"
                );
            }
            Err(_) => {}
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
    code: http::StatusCode,
    request_id: String,
    context: HashMap<String, String>,
    err: Option<Box<dyn std::error::Error>>,
) -> HttpError {
    if let Some(ref e) = err {
        error!(message = message, request_id, error = %e, context = ?context);
    } else {
        error!(message = message, request_id, context = ?context);
    }

    HttpError {
        status_code: code,
        error_code: None,
        external_message: format!("{}: {}", code.canonical_reason().unwrap(), message),
        internal_message: message,
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
    ($message:expr, $code:expr, $req_id:expr, $error:expr $(, $key:ident = $value:expr)*) => {{
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
    mut conn: WebSocketStream<Upgraded>,
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
