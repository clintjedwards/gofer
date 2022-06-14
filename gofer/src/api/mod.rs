mod gofer_impl;
mod namespaces;
mod pipelines;
mod runs;
mod system;
mod triggers;
mod validate;

use crate::{conf, events, frontend, scheduler, storage};
use anyhow::anyhow;
use dashmap::DashMap;
use gofer_proto::gofer_server::GoferServer;
use http::header::CONTENT_TYPE;
use slog_scope::info;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::transport::{Certificate, ClientTlsConfig, Uri};
use tower::{steer::Steer, ServiceExt};

const BUILD_SEMVER: &str = env!("BUILD_SEMVER");
const BUILD_COMMIT: &str = env!("BUILD_COMMIT");

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

pub struct Api {
    /// Various configurations needed by the api
    conf: conf::api::Config,

    /// The main backend storage implementation. Gofer stores most of its critical state information here.
    storage: storage::Db,

    /// The mechanism in which Gofer uses to run individual containers.
    scheduler: Box<dyn scheduler::Scheduler + Sync + Send>,

    /// Used throughout the whole application in order to allow functions to wait on state changes in Gofer.
    event_bus: events::EventBus,

    /// An in-memory map of currently registered and started triggers.
    /// This is necessary due to triggers being based on containers and their state needing to be constantly
    /// updated and maintained.
    triggers: DashMap<String, gofer_models::Trigger>,

    /// An in-memory map of currently registered notifiers. These notifiers are registered on startup
    /// and launched as needed at the end of a user's pipeline run. Gofer refers to this cache as a way
    /// to quickly look up which container is needed to be launched.
    notifiers: DashMap<String, gofer_models::Notifier>,
}

impl Api {
    /// Create a new instance of API with all services started.
    pub async fn start(conf: conf::api::Config) {
        let storage = storage::Db::new(&conf.server.storage_path).await.unwrap();
        let scheduler = scheduler::init_scheduler(&conf.scheduler).await.unwrap();
        let event_bus = events::EventBus::new(
            storage.clone(),
            conf.general.event_retention,
            conf.general.event_prune_interval,
        );

        let api = Api {
            conf,
            storage,
            scheduler,
            event_bus,
            triggers: DashMap::new(),
            notifiers: DashMap::new(),
        };

        api.create_default_namespace().await.unwrap();
        api.start_triggers().await.unwrap();
        api.start_service().await;
    }

    /// Gofer starts with a default namespace that all users have access to.
    async fn create_default_namespace(&self) -> Result<(), storage::StorageError> {
        const DEFAULT_NAMESPACE_ID: &str = "default";
        const DEFAULT_NAMESPACE_NAME: &str = "Default";
        const DEFAULT_NAMESPACE_DESCRIPTION: &str =
            "The default namespace when no other namespace is specified.";

        let default_namespace = gofer_models::Namespace::new(
            DEFAULT_NAMESPACE_ID,
            DEFAULT_NAMESPACE_NAME,
            DEFAULT_NAMESPACE_DESCRIPTION,
        );

        match self.storage.create_namespace(&default_namespace).await {
            Ok(_) => {
                self.event_bus
                    .publish(gofer_models::EventKind::CreatedNamespace {
                        namespace_id: DEFAULT_NAMESPACE_ID.to_string(),
                    })
                    .await;
                Ok(())
            }
            Err(e) => match e {
                storage::StorageError::Exists => Ok(()),
                _ => Err(e),
            },
        }
    }

    /// Start a TLS enabled, multiplexed, grpc/http server.
    async fn start_service(self) {
        let config = self.conf.clone();
        let cert = config.server.tls_cert.clone().into_bytes();
        let key = config.server.tls_key.clone().into_bytes();

        let http = axum::Router::new()
            .route("/*path", axum::routing::any(frontend::frontend_handler))
            .map_err(tower::BoxError::from)
            .boxed_clone();

        let grpc = tonic::transport::Server::builder()
            .add_service(GoferServer::new(self))
            .into_service()
            .map_response(|r| r.map(axum::body::boxed))
            .boxed_clone();

        let http_grpc = Steer::new(
            vec![http, grpc],
            |req: &http::Request<hyper::Body>, _svcs: &[_]| {
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
            .addr_incoming_config(tcp_settings)
            .serve(tower::make::Shared::new(http_grpc))
            .await
            .expect("server exited unexpectedly");
    }
}
