//! The `web/` service: the site's files with their types, byte ranges and a path kept inside the root, on tower-http's `ServeDir`.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::header::{ACCEPT_RANGES, CACHE_CONTROL, CONTENT_TYPE, RANGE};
use axum::http::uri::PathAndQuery;
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use tower_http::services::ServeDir;

/// The types the site's files have; anything else is a byte stream.
fn content_type(path: &str) -> &'static str {
    let path = if path.ends_with('/') { "index.html" } else { path };
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("wasm") => "application/wasm",
        Some("woff2") => "font/woff2",
        Some("pbf") => "application/x-protobuf",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Whether a decoded path has more `..` than the directories above them.
fn climbs_out(path: &str) -> bool {
    let mut depth = 0i32;
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => depth += 1,
        }
    }
    false
}

/// A segment is encoded but for the unreserved characters, so what ServeDir decodes is exactly what was kept.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// The decoded path with its `.` and `..` segments resolved (call it only after `climbs_out` said no, so no `..`
/// is left over), encoded again, with the slash that ended it. This is the path as a browser would resolve it.
fn normalised(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    let mut clean = String::from("/");
    clean.push_str(&parts.iter().map(|p| utf8_percent_encode(p, SEGMENT).to_string()).collect::<Vec<_>>().join("/"));
    let directory = path.ends_with('/') || path.ends_with("/.") || path.ends_with("/..");
    if directory && !parts.is_empty() {
        clean.push('/');
    }
    clean
}

/// The request's address with its path replaced and its query kept. `None` when the result is not an address.
fn with_path(uri: &Uri, path: &str) -> Option<Uri> {
    let path_and_query = match uri.query() {
        Some(q) => format!("{path}?{q}"),
        None => path.to_string(),
    };
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(path_and_query.parse::<PathAndQuery>().ok()?);
    Uri::from_parts(parts).ok()
}

async fn apply(State(cache_control): State<Arc<HeaderValue>>, mut req: Request, next: Next) -> Response {
    let Ok(path) = percent_decode_str(req.uri().path()).decode_utf8().map(|p| p.into_owned()) else {
        return (StatusCode::BAD_REQUEST, "bad request").into_response();
    };
    if climbs_out(&path) {
        return StatusCode::FORBIDDEN.into_response();
    }
    // From here the path stays in the root, so give ServeDir the collapsed one: it never sees a `..` segment
    // (it would answer 404 for `/sub/../map.html`, which is the file `/map.html`).
    let path = normalised(&path);
    match with_path(req.uri(), &path) {
        Some(uri) => *req.uri_mut() = uri,
        None => return (StatusCode::BAD_REQUEST, "bad request").into_response(),
    }
    // A list of ranges is not supported: the whole file is answered.
    if req.headers().get(RANGE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.contains(',')) {
        req.headers_mut().remove(RANGE);
    }
    let mut res = next.run(req).await;
    if res.status().is_success() {
        res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(content_type(&path)));
    }
    let headers = res.headers_mut();
    headers.insert(CACHE_CONTROL, (*cache_control).clone());
    headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    res
}

pub fn router(root: PathBuf, cache_control: String) -> Router {
    let cache_control = Arc::new(HeaderValue::from_str(&cache_control).unwrap_or_else(|_| HeaderValue::from_static("no-store")));
    Router::new().fallback_service(ServeDir::new(root).append_index_html_on_directories(true)).layer(middleware::from_fn_with_state(cache_control, apply))
}
