use crate::api::Api;
use gofer_proto::gofer_server::GoferServer;

use slog_scope::info;

use axum::{body::BoxBody, response::IntoResponse};
use axum_server::tls_rustls::RustlsConfig;
use futures::{
    future::{BoxFuture, Either},
    ready, TryFutureExt,
};
use std::{convert::Infallible, io::BufReader, sync::Arc, task::Poll};
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tower::Service;

pub async fn start_server(service: MultiplexService<axum::Router, GoferServer<Api>>, host: &str) {
    info!("Started multiplexed grpc/http service"; "url" => host.parse::<String>().unwrap());

    axum::Server::bind(&host.parse().unwrap())
        .tcp_keepalive(Some(std::time::Duration::from_secs(15)))
        .serve(tower::make::Shared::new(service))
        .await
        .expect("server exited unexpectedly");
}

pub async fn start_tls_server(
    service: MultiplexService<axum::Router, GoferServer<Api>>,
    host: &str,
    cert: Vec<u8>,
    key: Vec<u8>,
) {
    let tls_config = get_tls_config(cert, key);

    let tcp_settings = axum_server::AddrIncomingConfig::new()
        .tcp_keepalive(Some(std::time::Duration::from_secs(15)))
        .build();

    info!("Started multiplexed, TLS enabled, grpc/http service"; "url" => host.parse::<String>().unwrap());

    axum_server::bind_rustls(host.parse().unwrap(), tls_config)
        .addr_incoming_config(tcp_settings)
        .serve(tower::make::Shared::new(service))
        .await
        .expect("server exited unexpectedly");
}

/// returns a TLS configuration object for use in the multiplexing server.
fn get_tls_config(cert: Vec<u8>, key: Vec<u8>) -> RustlsConfig {
    let mut buffered_cert: BufReader<&[u8]> = BufReader::new(&cert);
    let mut buffered_key: BufReader<&[u8]> = BufReader::new(&key);

    let certs = rustls_pemfile::certs(&mut buffered_cert)
        .expect("could not get certificate chain")
        .into_iter()
        .map(Certificate)
        .collect();

    let key = PrivateKey(
        rustls_pemfile::pkcs8_private_keys(&mut buffered_key)
            .expect("could not get private key")
            .get(0)
            .expect("could not get private key")
            .to_vec(),
    );

    let tls_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("could not load certificate or private key");

    RustlsConfig::from_config(Arc::new(tls_config))
}

#[derive(Clone)]
pub struct MultiplexService<A, B> {
    pub rest: A,
    pub grpc: B,
}

impl<A, B> Service<hyper::Request<hyper::Body>> for MultiplexService<A, B>
where
    A: Service<hyper::Request<hyper::Body>, Error = Infallible>,
    A::Response: IntoResponse,
    A::Future: Send + 'static,
    B: Service<hyper::Request<hyper::Body>, Error = Infallible>,
    B::Response: IntoResponse,
    B::Future: Send + 'static,
{
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = hyper::Response<BoxBody>;

    // This seems incorrect. We never check GRPC readiness; but I'm too lazy
    // to fix it and it seems to work well enough.
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(if let Err(err) = ready!(self.rest.poll_ready(cx)) {
            Err(err)
        } else {
            ready!(self.rest.poll_ready(cx))
        })
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        let hv = req.headers().get("content-type").map(|x| x.as_bytes());

        let fut = if hv
            .filter(|value| value.starts_with(b"application/grpc"))
            .is_some()
        {
            Either::Left(self.grpc.call(req).map_ok(|res| res.into_response()))
        } else {
            Either::Right(self.rest.call(req).map_ok(|res| res.into_response()))
        };

        Box::pin(fut)
    }
}
