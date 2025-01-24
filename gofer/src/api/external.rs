use crate::api::{extensions, load_tls, wait_for_shutdown_signal, ApiState, Middleware};
use crate::conf;
use anyhow::{anyhow, Context, Result};
use dropshot::{
    endpoint, ApiDescription, ConfigDropshot, ConfigTls, HandlerTaskMode, HttpError,
    HttpResponseUpdatedNoContent, Path, RequestContext, ServerBuilder, UntypedBody,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use tracing::{error, info};

pub async fn start_web_service(conf: conf::api::ApiConfig, api_state: Arc<ApiState>) -> Result<()> {
    let bind_address = std::net::SocketAddr::from_str(&conf.external_events.bind_address.clone()).with_context(|| {
        format!(
            "Could not parse url '{}' while trying to bind binary to port; \
    should be in format '<ip>:<port>'; Please be sure to use an ip instead of something like 'localhost', \
    when attempting to bind",
            &conf.server.bind_address.clone()
        )
    })?;

    let dropshot_conf = ConfigDropshot {
        bind_address,
        default_request_body_max_bytes: 524288000, // 500MB to allow for extra large objects.

        // If a client disconnects run the handler to completion still. Eventually we'll want to save resources
        // by allowing the handler to early cancel, but until this is more developed lets just run it to completion.
        default_handler_task_mode: HandlerTaskMode::Detached,
    };

    let mut api = ApiDescription::new();

    /* /api/external/{extension_id} */
    api.register(external_event_handler).unwrap();

    let tls_config = match conf.server.use_tls {
        true => {
            let (tls_cert, tls_key) = load_tls(
                conf.external_events.use_tls,
                conf.external_events.tls_cert_path,
                conf.external_events.tls_key_path,
            )?;

            Some(ConfigTls::AsBytes {
                certs: tls_cert,
                key: tls_key,
            })
        }
        false => None,
    };

    let server = ServerBuilder::new(api, api_state, Some(Arc::new(Middleware)))
        .config(dropshot_conf)
        .tls(tls_config)
        .start()
        .map_err(|error| anyhow!("failed to create server: {}", error))?;

    let shutdown = server.wait_for_shutdown();

    tokio::spawn(wait_for_shutdown_signal(server));

    info!(
        message = "Started Gofer external http service",
        host = %bind_address.ip(),
        port = %bind_address.port(),
        tls = conf.server.use_tls,
    );

    shutdown
        .await
        .map_err(|error| anyhow!("Server encountered errors while running; {:#?}", error))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExternalEventPathArgs {
    /// The unique identifier for the target extension.
    pub extension_id: String,
}

/// Create a new external event.
///
/// The data here will be passed to the targeted extension.
///
/// This route is only accessible for admin tokens.
#[endpoint(
    method = POST,
    path = "/api/external/{extension_id}",
    tags = ["ExternalEvents"],
)]
pub async fn external_event_handler(
    rqctx: RequestContext<Arc<ApiState>>,
    path: Path<ExternalEventPathArgs>,
    body: UntypedBody,
) -> Result<HttpResponseUpdatedNoContent, HttpError> {
    let api_state = rqctx.context();
    let path = path.into_inner();

    let extension = match api_state.extensions.get(&path.extension_id) {
        Some(extension) => extension.value().clone(),
        None => {
            return Err(HttpError::for_bad_request(
                None,
                format!("extension_id '{}' not found", &path.extension_id,),
            ));
        }
    };

    let client = extensions::new_extension_client(&extension.url, &extension.secret, api_state.config.extensions.verify_certs
    ).map_err(|err| {
        error!(error = %err, extension_id = &path.extension_id, "Could not send external event to extension");

        HttpError::for_internal_error("Could not send external event to extension".into())
    })?;

    let headers = rqctx.request.headers();

    let mut headers_map = std::collections::HashMap::new();

    for (key, value) in headers.iter() {
        let header_key = key.as_str().to_string();

        let header_value = match value.to_str() {
            Ok(value) => value,
            Err(_) => continue,
        };

        headers_map.insert(header_key, header_value.to_string());
    }

    if let Err(err) = client
        .external_event(&gofer_sdk::extension::api::types::ExternalEventRequest {
            headers: headers_map,
            body: body.as_bytes().to_vec(),
        })
        .await
    {
        error!(error = %err, extension_id = &path.extension_id, "Could not send external event to extension");

        return Err(HttpError::for_internal_error(
            "Could not send external event to extension".into(),
        ));
    }

    Ok(HttpResponseUpdatedNoContent())
}
