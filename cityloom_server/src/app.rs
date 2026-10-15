//! The whole router: the account routes (or their 503), then the static site.
use axum::Router;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::any;

use crate::config::Config;
use crate::static_files;

async fn unavailable() -> Response {
    let mut res = (StatusCode::SERVICE_UNAVAILABLE, r#"{"error":"account-unavailable"}"#).into_response();
    res.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

pub fn router(config: &Config) -> Router {
    Router::new()
        .route("/auth/{*rest}", any(unavailable))
        .route("/api/{*rest}", any(unavailable))
        .fallback_service(static_files::router(config.web_root.clone(), config.cache_control.clone()))
}
