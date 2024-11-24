pub mod api;

use async_trait::async_trait;
use dropshot::{
    endpoint, ApiDescription, ConfigDropshot, ConfigTls, DropshotState, HttpError, HttpResponseOk,
    HttpResponseUpdatedNoContent, HttpServer, HttpServerStarter, RequestContext, RequestInfo,
    ServerContext, TypedBody,
};
use futures::Future;
use http::Request;
use hyper::Body;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, pin::Pin};
use std::{env, str::FromStr};
use std::{net::SocketAddr, sync::Arc};
use tracing::{error, info};

/// Represents different extensions failure possibilities. These errors are meant to be consumed by extension authors.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ExtensionError {
    #[error("System environment variable '{0}' missing or empty but required.")]
    RequiredSystemEnvVarMissing(String),

    #[error("Encountered an error while attempting to parse system env vars; {0}")]
    SystemEnvVarError(String),

    #[error("Error encountered during HTTP server startup; {0}")]
    ServerError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Parameter {
    pub key: String,
    pub required: bool,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Documentation {
    /// Each extension has configuration parameters that can be passed in at extension startup. These parameters
    /// should control extension behavior for it's entire lifetime.
    pub config_params: Vec<Parameter>,

    /// Each extension has pipeline subscription parameters that are passed in by a pipeline when it attempts to
    /// subscribe to an extension. This controls how the extension treats that specific pipeline subscription.
    pub pipeline_subscription_params: Vec<Parameter>,

    /// Anything the extension wants to explain to the user. This text is inserted into the documentation a user
    /// can look up about the extension. Supports AsciiDoc.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InfoResponse {
    /// The unique extension identifier
    pub extension_id: String,

    /// Documentation about how to use the extension.
    pub documentation: Documentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DebugResponse {
    pub info: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SubscriptionRequest {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// A unique name created by the pipeline owner to differentiate this pipeline subscription to the extension
    /// from any others to the same extension.
    pub pipeline_subscription_id: String,

    /// Each extension has pipeline subscription parameters that are passed in by a pipeline when it attempts to
    /// subscribe to an extension. This controls how the extension treats that specific pipeline subscription.
    pub pipeline_subscription_params: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UnsubscriptionRequest {
    /// The unique identifier for the target namespace.
    pub namespace_id: String,

    /// The unique identifier for the target pipeline.
    pub pipeline_id: String,

    /// A unique name created by the pipeline owner to differentiate this pipeline subscription to the extension
    /// from any others to the same extension.
    pub pipeline_subscription_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExternalEventRequest {
    /// The headers for the incoming external request.
    pub headers: HashMap<String, String>,

    /// The bytes of the response body for the external request.
    pub body: Vec<u8>,
}

/// The Extension trait serves as a contact point for developers writing extensions. It clearly defines the endpoints
/// that must be implemented by every extension and also takes care of setting up the HTTP service, documentation and
/// more.
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    /// A simple healthcheck endpoint used by Gofer to make sure the extension is still in good health and reachable.
    async fn health(&self) -> Result<(), HttpError>;

    /// Returns information specific to the extension.
    async fn info(&self) -> Result<InfoResponse, HttpError>;

    /// Allows the extension to print any information relevant to it's execution.
    /// This endpoint is freely open so make sure to not include any particularly sensitive information in this
    /// endpoint.
    async fn debug(&self) -> DebugResponse;

    /// Registers a pipeline with said extension to provide the extension's functionality.
    async fn subscribe(&self, request: SubscriptionRequest) -> Result<(), HttpError>;

    /// Allows pipelines to remove their extension subscriptions.
    async fn unsubscribe(&self, request: UnsubscriptionRequest) -> Result<(), HttpError>;

    /// Shutdown tells the extension to cleanup and gracefully shutdown. If a extension
    /// does not shutdown in a time defined by the Gofer API the extension will
    /// instead be Force shutdown(SIGKILL). This is to say that all extensions should
    /// lean toward quick cleanups and shutdowns.
    async fn shutdown(&self);

    /// Gofer supports external requests from third-parties to perform different actions. Upon receiving such a request
    /// Gofer will process the request and return the body via this endpoint.
    async fn external_event(&self, request: ExternalEventRequest) -> Result<(), HttpError>;
}

/// A wrapper for the user's concrete implementation of the Extension trait such that we can add in extra functionality
/// so the user doesn't have to.
struct ExtensionWrapper {
    auth_key: String,
    extension: Box<dyn Extension>,
}

impl ExtensionWrapper {
    /// Checks request authentication.
    fn check_auth(&self, request: &RequestInfo) -> Result<(), HttpError> {
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

        if token != self.auth_key {
            return Err(HttpError::for_client_error(
                None,
                http::StatusCode::UNAUTHORIZED,
                "Unauthorized".into(),
            ));
        }

        Ok(())
    }
}

/// Returns extension health information. Useful for health checks.
#[endpoint(
    method = GET,
    path = "/api/health",
)]
async fn health(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let context = rqctx.context();
    context.extension.health().await?;

    Ok(HttpResponseUpdatedNoContent())
}

/// Returns general metadata about the extension.
#[endpoint(
    method = GET,
    path = "/api/info",
)]
async fn info(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
) -> Result<HttpResponseOk<InfoResponse>, HttpError> {
    let context = rqctx.context();
    context.check_auth(&rqctx.request)?;

    let name = get_env("GOFER_EXTENSION_SYSTEM_ID").unwrap_or("unknown".into());

    let mut resp = context.extension.info().await?;
    resp.extension_id = name;

    Ok(HttpResponseOk(resp))
}

/// Returns inner state information about the extension to aid in debugging.
#[endpoint(
    method = GET,
    path = "/api/debug",
)]
async fn debug(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
) -> Result<HttpResponseOk<DebugResponse>, HttpError> {
    let context = rqctx.context();
    context.check_auth(&rqctx.request)?;

    Ok(HttpResponseOk(context.extension.debug().await))
}

/// Register pipeline with extension.
#[endpoint(
    method = POST,
    path = "/api/subscribe",
)]
async fn subscribe(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
    body: TypedBody<SubscriptionRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let context = rqctx.context();
    context.check_auth(&rqctx.request)?;
    let body = body.into_inner();

    context.extension.subscribe(body).await?;

    Ok(HttpResponseUpdatedNoContent())
}

/// Unregister a pipeline with extension.
#[endpoint(
    method = DELETE,
    path = "/api/subscribe",
)]
async fn unsubscribe(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
    body: TypedBody<UnsubscriptionRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let context = rqctx.context();
    context.check_auth(&rqctx.request)?;
    let body = body.into_inner();

    context.extension.unsubscribe(body).await?;

    Ok(HttpResponseUpdatedNoContent())
}

/// Shutdown tells the extension to cleanup and gracefully shutdown. If a extension
/// does not shutdown in a time defined by the Gofer API the extension will
/// instead be forced to shutdown via SIGKILL.
#[endpoint(
    method = POST,
    path = "/api/shutdown",
)]
async fn shutdown(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let context = rqctx.context();
    context.check_auth(&rqctx.request)?;

    context.extension.shutdown().await;

    Ok(HttpResponseUpdatedNoContent())
}

/// Gofer supports external requests from third-parties to perform different actions. Upon receiving such a request
/// Gofer will process the request and return the body via this endpoint.
#[endpoint(
    method = POST,
    path = "/api/external-event",
)]
async fn external_event(
    rqctx: RequestContext<Arc<ExtensionWrapper>>,
    body: TypedBody<ExternalEventRequest>,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let context = rqctx.context();
    context.check_auth(&rqctx.request)?;
    let body = body.into_inner();

    context.extension.external_event(body).await?;

    Ok(HttpResponseUpdatedNoContent())
}

pub struct SystemConfig {
    /// Secret is the auth key passed by the main Gofer application to prevent other actors from communicating
    /// with the extensions.
    pub secret: String,

    /// Unique identifier for the extension.
    pub id: String,

    /// The log_level this extension should emit. Logs are ingested into the main Gofer application and combined
    /// with the main application logs.
    pub log_level: String,

    /// If use_tls is false tls_key and tls_cert are allowed to be empty.
    pub use_tls: bool,
    pub tls_key: String,
    pub tls_cert: String,

    /// The ip:port combination this extension should attempt to use. This being predictable allows Gofer to correctly
    /// contact the service and tell the scheduler how it should handle networking.
    pub bind_address: String,

    /// The address to contact the main Gofer process.
    pub gofer_host: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        SystemConfig {
            secret: String::new(),
            id: String::new(),
            log_level: "info".into(),
            use_tls: true,
            tls_key: String::new(),
            tls_cert: String::new(),
            bind_address: "0.0.0.0:8082".into(),
            gofer_host: "http://localhost:8080".into(),
        }
    }
}

impl SystemConfig {
    pub fn from_env() -> Result<SystemConfig, Box<dyn Error>> {
        let mut config = SystemConfig::default();
        config.secret = get_env("GOFER_EXTENSION_SYSTEM_SECRET").unwrap_or(config.secret);
        config.id = get_env("GOFER_EXTENSION_SYSTEM_ID").unwrap_or(config.id);
        config.log_level = get_env("GOFER_EXTENSION_SYSTEM_LOG_LEVEL").unwrap_or(config.log_level);
        config.tls_key = get_env("GOFER_EXTENSION_SYSTEM_TLS_KEY").unwrap_or(config.tls_key);
        config.tls_cert = get_env("GOFER_EXTENSION_SYSTEM_TLS_CERT").unwrap_or(config.tls_cert);
        config.use_tls = get_env("GOFER_EXTENSION_SYSTEM_USE_TLS")
            .unwrap_or(config.use_tls.to_string())
            .parse()?;
        config.bind_address =
            get_env("GOFER_EXTENSION_SYSTEM_BIND_ADDRESS").unwrap_or(config.bind_address);
        config.gofer_host =
            get_env("GOFER_EXTENSION_SYSTEM_GOFER_HOST").unwrap_or(config.gofer_host);

        Ok(config)
    }

    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.secret.is_empty() {
            return Err("Env var 'secret' required but missing".into());
        }
        if self.id.is_empty() {
            return Err("Env var 'id' required but missing".into());
        }

        if self.use_tls {
            if self.tls_cert.is_empty() {
                return Err("Env var 'tls_cert' required but missing".into());
            }
            if self.tls_key.is_empty() {
                return Err("Env var 'tls_key' required but missing".into());
            }
        }

        Ok(())
    }
}

/// Does a more thorough evaluation of env vars by making sure not only they are set, but also making sure they're
/// not an empty string.
fn get_env(key: &str) -> Option<String> {
    if let Ok(value) = env::var(key) {
        if !value.is_empty() {
            return Some(value);
        }
    }

    None
}

fn init_api() -> ApiDescription<Arc<ExtensionWrapper>> {
    let mut api = ApiDescription::new();
    api.register(health).unwrap();
    api.register(info).unwrap();
    api.register(debug).unwrap();
    api.register(subscribe).unwrap();
    api.register(unsubscribe).unwrap();
    api.register(shutdown).unwrap();
    api.register(external_event).unwrap();

    api
}

pub fn write_openapi_spec(path: std::path::PathBuf) -> Result<(), Box<dyn Error>> {
    let api = init_api();
    let mut file = std::fs::File::create(path)?;
    api.openapi("Gofer Extension", "0.0.0").write(&mut file)?;

    Ok(())
}

pub async fn run(ext: Box<dyn Extension>) -> Result<(), Box<dyn Error>> {
    let config =
        SystemConfig::from_env().map_err(|e| ExtensionError::SystemEnvVarError(e.to_string()))?;
    config
        .validate()
        .map_err(|e| ExtensionError::RequiredSystemEnvVarMissing(e.to_string()))?;

    let addr = std::net::SocketAddr::from_str(&config.bind_address).map_err(|e| ExtensionError::ServerError(
        format!(
            "Could not parse url '{}' while trying to bind binary to port; \
    should be in format '<ip>:<port>'; Please be sure to use an ip instead of something like 'localhost', \
    when attempting to bind; {:#?}",
            &config.bind_address, e
        )
    ))?;

    let extension = ExtensionWrapper {
        auth_key: config.secret,
        extension: ext,
    };

    let dropshot_conf = ConfigDropshot {
        bind_address: addr,
        ..Default::default()
    };

    let api = init_api();

    let server = if !config.use_tls {
        HttpServerStarter::new(
            &dropshot_conf,
            api,
            Some(Arc::new(Middleware)),
            Arc::new(extension),
        )
        .map_err(|error| {
            ExtensionError::ServerError(format!("Could not start HTTP server; {:#?}", error))
        })?
        .start()
    } else {
        let tls_config = Some(ConfigTls::AsBytes {
            certs: config.tls_cert.into_bytes(),
            key: config.tls_key.into_bytes(),
        });

        HttpServerStarter::new_with_tls(
            &dropshot_conf,
            api,
            Some(Arc::new(Middleware)),
            Arc::new(extension),
            tls_config,
        )
        .map_err(|error| {
            ExtensionError::ServerError(format!("Could not start HTTPS server; {:#?}", error))
        })?
        .start()
    };
    let shutdown_handle = server.wait_for_shutdown();

    tokio::spawn(wait_for_shutdown_signal(server));

    info!(
        message = "started Gofer external event http service",
        "host" = %addr.ip(),
        "port" = %addr.port(),
    );

    shutdown_handle.await?;

    Ok(())
}

async fn wait_for_shutdown_signal(server: HttpServer<Arc<ExtensionWrapper>>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    server.close().await.unwrap()
}

/// Convenience function for grabbing the extension specific config value from the environment.
/// Gofer passes in these values into the environment when the extension first starts.
pub fn get_config_from_env(config: &str) -> Option<String> {
    let key = format!("GOFER_EXTENSION_CONFIG_{}", config.to_uppercase());
    env::var(key).ok()
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
