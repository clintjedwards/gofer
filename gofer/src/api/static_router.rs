use crate::api::ApiState;
use dropshot::{Body, HttpError, Path, RequestContext, endpoint};
use hyper::{Response, StatusCode, header};
use rust_embed::RustEmbed;
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

#[derive(RustEmbed)]
#[folder = "public"]
pub struct EmbeddedFrontendFS;

#[derive(RustEmbed)]
#[folder = "docs/book/html"]
struct EmbeddedDocumentationFS;

/// Dropshot deserializes the input path into this Vec.
#[derive(Deserialize, JsonSchema)]
struct AllPath {
    path: Vec<String>,
}

/// Serve files from the specified root path.
#[endpoint {
    method = GET,

    /*
     * Match literally every path including the empty path.
     */
    path = "/docs/{path:.*}",

    /*
     * This isn't an API so we don't want this to appear in the OpenAPI
     * description if we were to generate it.
     */
    unpublished = true,
}]
pub async fn static_documentation_handler(
    _rqctx: RequestContext<Arc<ApiState>>,
    path: Path<AllPath>,
) -> Result<hyper::Response<Body>, HttpError> {
    let path = path.into_inner().path;

    // Turns the path into one that we can actually route.
    let path = path.join("/"); //  css/variables.css

    // If the path is empty redirect the user to the actual index.html page. If this is not done, they end up on a
    // broken /docs page.
    if path.is_empty() {
        let response = Response::builder()
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header("Location", "/docs/index.html")
            .body(Body::empty())
            .unwrap();

        return Ok(response);
    }

    match EmbeddedDocumentationFS::get(&path) {
        Some(content) => {
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("txt");

            let mime_type = mime_guess::from_ext(ext).first_or_text_plain();

            Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime_type.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap())
        }
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from("<h1>404</h1><p>Not Found</p>"))
            .unwrap()),
    }
}

#[derive(Deserialize, JsonSchema)]
struct ManifestPath {
    name: String,
}

/// Serve the manifests for Gofer's default extensions, which are built into Gofer. Operators can read exactly what
/// Gofer runs for them, and `gofer extension manifest <name>` reads them from here.
#[endpoint {
    method = GET,
    path = "/extensions/manifests/{name}",
    unpublished = true,
}]
pub async fn default_manifest_handler(
    _rqctx: RequestContext<Arc<ApiState>>,
    path: Path<ManifestPath>,
) -> Result<Response<Body>, HttpError> {
    let name = path.into_inner().name;
    let name = name.strip_suffix(".toml").unwrap_or(&name);

    match crate::api::extensions::reconcile::default_manifest(name) {
        Some(content) => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/toml")
            .body(Body::from(content))
            .unwrap()),
        None => Err(HttpError::for_not_found(
            None,
            format!("'{name}' isn't one of Gofer's default extensions"),
        )),
    }
}

/// Serve files from the specified root path.
///
/// Dropshot does not allow paths to overlap, see discussions here:
///   * https://github.com/oxidecomputer/omicron/issues/430
///   * https://github.com/oxidecomputer/dropshot/issues/199
///
/// To mitigate this we use a path with a differentiator subpath and to avoid an ugly random character we use a hyphen.
#[endpoint {
    method = GET,

    /*
     * Match literally every path including the empty path.
     */
    path = "/{path:.*}",

    /*
     * This isn't an API so we don't want this to appear in the OpenAPI
     * description if we were to generate it.
     */
    unpublished = true,
}]
pub async fn static_handler(
    _rqctx: RequestContext<Arc<ApiState>>,
    path: Path<AllPath>,
) -> Result<Response<Body>, HttpError> {
    let path = path.into_inner().path;

    // Turns the path into one that we can actually route.
    let mut path = path.join("/"); //  css/variables.css

    if path.is_empty() {
        path = "index.html".into()
    }

    match EmbeddedFrontendFS::get(&path) {
        Some(content) => {
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("txt");

            let mime_type = mime_guess::from_ext(ext).first_or_text_plain();

            Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime_type.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap())
        }
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from("<h1>404</h1><p>Not Found</p>"))
            .unwrap()),
    }
}
