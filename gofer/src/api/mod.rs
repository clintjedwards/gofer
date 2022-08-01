mod common_tasks;
mod event_handlers;
mod fmt;
mod gofer_impl;
mod namespaces;
mod pipelines;
mod runs;
mod system;
mod task_runs;
mod triggers;
mod validate;

use crate::{conf, events, frontend, object_store, scheduler, secret_store, storage};
use anyhow::anyhow;
use axum_server::Handle;
use dashmap::DashMap;
use gofer_models::{common_task, event, namespace, trigger};
use gofer_proto::gofer_server::GoferServer;
use http::header::CONTENT_TYPE;
use slog_scope::info;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{ops::Deref, str::FromStr, sync::Arc};
use tokio_util::sync::CancellationToken;
use tonic::transport::{Certificate, ClientTlsConfig, Uri};
use tower::{steer::Steer, ServiceExt};

const BUILD_SEMVER: &str = env!("BUILD_SEMVER");
const BUILD_COMMIT: &str = env!("BUILD_COMMIT");

/// GOFER_EOF is a special string marker we include at the end of log files.
/// It denotes that no further logs will be written. This is to provide the functionality for downstream
/// applications to follow log files and not also have to monitor the container for state to know when
/// logs will no longer be printed.
///
/// If this did not exist, downstream applications would have no idea the difference between a file
/// that was still pending log_lines and a file that was at it's final resting state.
const GOFER_EOF: &str = "GOFER_EOF";

pub fn epoch() -> u64 {
    let current_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    u64::try_from(current_epoch).unwrap()
}

/// Returns a valid TLS configuration for GRPC connections. Most of this is only required to make
/// self-signed cert usage easier. Rustls wont allow IP addresses in the url field and wont allow
/// you to skip client-side issuer verification. So if the user enters 127.0.0.1 we replace
/// it with the domain "localhost" and if the user supplies us with a root cert that trusts the
/// localhost certs we add it to the root certificate trust store.
fn get_tls_client_config(url: &str, ca_cert: Option<String>) -> anyhow::Result<ClientTlsConfig> {
    let uri = Uri::from_str(url)?;
    let mut domain_name = uri
        .host()
        .ok_or_else(|| anyhow!("could not get domain name from uri: {:?}", uri))?;
    if domain_name.eq("127.0.0.1") {
        domain_name = "localhost"
    }

    let mut tls_config = ClientTlsConfig::new().domain_name(domain_name);

    if let Some(ca_cert) = ca_cert {
        tls_config = tls_config.ca_certificate(Certificate::from_pem(ca_cert));
    }

    Ok(tls_config)
}

#[derive(Debug)]
pub struct ApiWrapper(Arc<Api>);

impl Deref for ApiWrapper {
    type Target = Arc<Api>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Api {
    /// Used to cancel downstream threads on shutdown.
    shutdown: CancellationToken,

    /// Various configurations needed by the api
    conf: conf::api::Config,

    /// The main backend storage implementation. Gofer stores most of its critical state information here.
    storage: storage::Db,

    /// The mechanism in which Gofer uses to run individual containers.
    scheduler: Arc<dyn scheduler::Scheduler + Sync + Send>,

    /// The mechanism in which Gofer stores pipeline and run level objects. The implementation is meant to
    /// act as a basic object store.
    object_store: Arc<dyn object_store::Store + Sync + Send>,

    /// The mechanism in which Gofer stores pipeline secrets. It allows users to store secret that can be
    /// interpreted in their pipeline files.
    secret_store: Arc<dyn secret_store::Store + Sync + Send>,

    /// Used throughout the whole application in order to allow functions to wait on state changes in Gofer.
    event_bus: Arc<events::EventBus>,

    /// An in-memory map of currently registered and started triggers.
    /// This is necessary due to triggers being based on containers and their state needing to be constantly
    /// updated and maintained.
    triggers: DashMap<String, trigger::Trigger>,

    /// An in-memory map of currently registered common_tasks. These common_tasks are registered on startup
    /// and launched as requested in the user's pipeline run. Gofer refers to this cache as a way
    /// to quickly look up which container is needed to be launched.
    common_tasks: DashMap<String, common_task::CommonTask>,
}

impl Api {
    /// Create a new instance of API with all services started.
    pub async fn start(conf: conf::api::Config) {
        let shutdown = CancellationToken::new();
        let storage = storage::Db::new(&conf.server.storage_path).await.unwrap();
        let scheduler = scheduler::init_scheduler(&conf.scheduler).await.unwrap();
        let object_store = object_store::init_object_store(&conf.object_store)
            .await
            .unwrap();
        let secret_store = secret_store::init_secret_store(&conf.secret_store)
            .await
            .unwrap();
        let event_bus = Arc::new(events::EventBus::new(
            storage.clone(),
            conf.general.event_retention,
            conf.general.event_prune_interval,
        ));

        let api = Api {
            shutdown,
            conf,
            storage,
            scheduler,
            object_store,
            secret_store,
            event_bus,
            triggers: DashMap::new(),
            common_tasks: DashMap::new(),
        };

        let api = Arc::new(api);

        api.create_default_namespace().await.unwrap();
        api.clone().start_triggers().await.unwrap();

        // Launch a thread that waits for ctrl-c and runs cleanup.
        let server_handle = axum_server::Handle::new();
        let server_cancel_handle = server_handle.clone();
        let handle_api = api.clone();
        tokio::spawn(async move { handle_api.handle_shutdown(server_cancel_handle).await });

        api.start_service(server_handle).await;
    }

    pub async fn handle_shutdown(&self, server_handle: Handle) {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
            }
        }

        // Send graceful stop to all triggers.
        self.stop_all_triggers().await;

        // Send cancel to all downstream threads.
        self.shutdown.cancel();

        // Shutdown the GRPC/HTTP service.
        server_handle.graceful_shutdown(Some(tokio::time::Duration::from_secs(
            self.conf.server.shutdown_timeout,
        )));
    }

    /// Gofer starts with a default namespace that all users have access to.
    async fn create_default_namespace(&self) -> Result<(), storage::StorageError> {
        const DEFAULT_NAMESPACE_ID: &str = "default";
        const DEFAULT_NAMESPACE_NAME: &str = "Default";
        const DEFAULT_NAMESPACE_DESCRIPTION: &str =
            "The default namespace when no other namespace is specified.";

        let default_namespace = namespace::Namespace::new(
            DEFAULT_NAMESPACE_ID,
            DEFAULT_NAMESPACE_NAME,
            DEFAULT_NAMESPACE_DESCRIPTION,
        );

        let mut conn = self.storage.conn().await?;

        if let Err(e) = storage::namespaces::insert(&mut conn, &default_namespace).await {
            match e {
                storage::StorageError::Exists => return Ok(()),
                _ => return Err(e),
            }
        };

        self.event_bus
            .publish(event::Kind::CreatedNamespace {
                namespace_id: DEFAULT_NAMESPACE_ID.to_string(),
            })
            .await;

        Ok(())
    }

    /// Start a TLS enabled, multiplexed, grpc/http server. Blocks until receives a ctrl-c.
    async fn start_service(self: Arc<Self>, handle: Handle) {
        let config = self.conf.clone();
        let cert = config.server.tls_cert.clone().into_bytes();
        let key = config.server.tls_key.clone().into_bytes();

        let http = axum::Router::new()
            .route("/*path", axum::routing::any(frontend::frontend_handler))
            .map_err(tower::BoxError::from)
            .boxed_clone();

        let grpc = tonic::transport::Server::builder()
            .add_service(GoferServer::new(ApiWrapper(self)))
            .into_service()
            .map_response(|r| r.map(axum::body::boxed))
            .boxed_clone();

        let http_grpc = Steer::new(
            vec![http, grpc],
            |req: &'_ http::Request<hyper::Body>, _svcs: &'_ [_]| {
                if req.headers().get(CONTENT_TYPE).map(|v| v.as_bytes())
                    != Some(b"application/grpc")
                {
                    0
                } else {
                    1
                }
            },
        );

        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem(cert, key)
            .await
            .expect("could not configure TLS");

        let tcp_settings = axum_server::AddrIncomingConfig::new()
            .tcp_keepalive(Some(std::time::Duration::from_secs(15)))
            .build();

        info!("Started multiplexed, TLS enabled, grpc/http service"; "url" => config.server.url.clone());

        axum_server::bind_rustls(config.server.url.parse().unwrap(), tls_config)
            .handle(handle)
            .addr_incoming_config(tcp_settings)
            .serve(tower::make::Shared::new(http_grpc))
            .await
            .expect("server exited unexpectedly");

        info!("Gracefully shutdown grpc/http service");
    }
}
