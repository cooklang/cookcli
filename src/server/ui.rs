use crate::server::AppState;
use axum::{
    body::Body,
    http::{StatusCode, Uri},
    response::Response,
    Router,
};
use rust_embed::RustEmbed;
use std::sync::Arc;

pub fn ui() -> Router<Arc<AppState>> {
    Router::new().fallback(static_ui)
}

#[derive(RustEmbed)]
#[folder = "ui/public/"]
struct Assets;

async fn static_ui(uri: Uri) -> impl axum::response::IntoResponse {
    use axum::response::IntoResponse;

    const INDEX_HTML: &str = "index.html";

    fn index_html() -> impl axum::response::IntoResponse {
        Assets::get(INDEX_HTML)
            .map(|content| {
                let body = Body::from(content.data);
                Response::builder()
                    .header(axum::http::header::CONTENT_TYPE, "text/html")
                    .body(body)
                    .unwrap()
            })
            .ok_or(StatusCode::NOT_FOUND)
    }

    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return Ok(index_html().into_response());
    }

    match Assets::get(path) {
        Some(content) => {
            let body = Body::from(content.data);
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Ok(Response::builder()
                .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap())
        }
        None => {
            if path.contains('.') {
                return Err(StatusCode::NOT_FOUND);
            }
            Ok(index_html().into_response())
        }
    }
}
