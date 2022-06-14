use axum::{
    body::{boxed, Full},
    http::header,
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/frontend/public"]
struct EmbeddedFrontendFS;

// An axum compliant method handler that we can use to serve frontend requests from the embedded files
// in the binary.
pub async fn frontend_handler(request: axum::http::Request<axum::body::Body>) -> impl IntoResponse {
    let path = request.uri().path().trim_start_matches('/');
    let file = EmbeddedFrontendFS::get(path);
    match file {
        Some(content) => {
            let payload = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(payload)
                .unwrap()
        }
        // Due to history mode of single page applications we want to redirect to index.html
        // anytime there isn't a path that makes sense.
        None => {
            let file = EmbeddedFrontendFS::get("index.html").unwrap();
            let payload = boxed(Full::from(file.data));
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(payload)
                .unwrap()
        }
    }
}
