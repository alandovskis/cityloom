//! The `web/` service: the site's files with their types, byte ranges and a path kept inside the root, on tower-http's `ServeDir`.
//!
//! **Versioned assets.** The code the pages run (scripts, styles, the WebAssembly modules and their glue, the
//! vendored libraries, the logo) is served under `/v/<build>/…`, where `<build>` is a hash of the content of all of
//! it. A page names the build it was made for by writing `{{build}}` in its markup, which the server fills in when it
//! serves the page; everything the page's scripts then import, and the wasm file the glue fetches, resolves under
//! the same prefix, so a page cannot pair a script with a module of another build. The prefix is only answered for the
//! current build (any other is a 404, never other content), so these URLs never change meaning and are cached for a
//! year. The pages themselves are never cached without revalidation: they are what names the build.
//!
//! **Compression.** Files that were compressed at build time (`scripts/precompress.mjs`: `.br`, `.zst` and `.gz`
//! beside the wasm, the glue and the vendored libraries) are sent as they are; anything else that is text is
//! compressed on the fly, quickly.
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{
    ACCEPT_ENCODING, ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, IF_MATCH, IF_MODIFIED_SINCE, IF_NONE_MATCH, IF_RANGE,
    IF_UNMODIFIED_SINCE, LAST_MODIFIED, RANGE, VARY,
};
use axum::http::uri::PathAndQuery;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use sha2::{Digest, Sha256};
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::services::ServeDir;

/// What a versioned URL is cached as: its name is its content, so it never changes.
pub const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// The tile index: a stale copy only delays an area's refresh, so it may be used for minutes (five fresh, ten more while it is revalidated).
const INDEX: &str = "public, max-age=300, stale-while-revalidate=600";
/// Glyph ranges and font licences: a stale one is a cosmetic difference at worst.
const GLYPHS: &str = "public, max-age=86400, stale-while-revalidate=604800";
/// What a page says where it names the build its assets belong to.
const BUILD_TOKEN: &str = "{{build}}";
const VERSIONED: &str = "/v/";

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

/// Whether a file is part of the versioned code: what the pages load and run, not the data (tiles, glyphs) or the
/// pages themselves.
fn is_versioned(path: &str) -> bool {
    let top = path.trim_start_matches('/').split('/').next().unwrap_or("");
    if matches!(top, "data" | "glyphs" | "licenses" | "node_modules") {
        return false;
    }
    matches!(Path::new(path).extension().and_then(|e| e.to_str()), Some("js" | "mjs" | "css" | "svg" | "wasm" | "woff2"))
}

/// Whether a path is a page: the server fills in its `{{build}}`.
fn is_page(path: &str) -> bool {
    path.ends_with('/') || path.ends_with(".html")
}

/// Every versioned file under `dir`, as `(path from the root, size, modified)`, in order.
fn versioned_files(root: &Path, dir: &Path, out: &mut Vec<(String, u64, Option<SystemTime>)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        let Ok(rel) = path.strip_prefix(root) else { continue };
        let rel = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
        if meta.is_dir() {
            if is_versioned(&format!("{rel}/x.js")) {
                versioned_files(root, &path, out);
            }
        } else if is_versioned(&rel) {
            out.push((rel, meta.len(), meta.modified().ok()));
        }
    }
}

/// The build's name: the first 16 hex digits of a hash over the names and content of the versioned files. It is
/// computed again only when one of them changed in size or time, so a page costs a walk of the tree and no hashing.
struct Build {
    root: PathBuf,
    state: Mutex<(Vec<(String, u64, Option<SystemTime>)>, String)>,
}

impl Build {
    fn new(root: PathBuf) -> Build {
        Build { root, state: Mutex::new((Vec::new(), String::new())) }
    }

    fn id(&self) -> String {
        let mut files = Vec::new();
        versioned_files(&self.root, &self.root, &mut files);
        files.sort();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.1.is_empty() || state.0 != files {
            let mut hash = Sha256::new();
            for (path, ..) in &files {
                let bytes = std::fs::read(self.root.join(path.trim_start_matches('/'))).unwrap_or_default();
                hash.update((path.len() as u64).to_le_bytes());
                hash.update(path.as_bytes());
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(&bytes);
            }
            *state = (files, hex::encode(hash.finalize())[..16].to_string());
        }
        state.1.clone()
    }
}

/// The codings of `accept-encoding` that have no stale copy: a `.br` or `.gz` older than the file beside it was made
/// from another build of it, and ServeDir would send it all the same. Without the coding, the file is sent whole and
/// the compression layer compresses it as it goes.
async fn drop_stale_codings(root: &Path, path: &str, headers: &mut HeaderMap) {
    let Some(accept) = headers.get(ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).map(str::to_owned) else { return };
    let file = root.join(path.trim_start_matches('/'));
    let Ok(source) = tokio::fs::metadata(&file).await else { return };
    let Ok(source) = source.modified() else { return };
    let mut kept = Vec::new();
    for token in accept.split(',') {
        let suffix = match token.split(';').next().unwrap_or("").trim().to_ascii_lowercase().as_str() {
            "br" => Some("br"),
            "gzip" => Some("gz"),
            _ => None,
        };
        let stale = match suffix {
            Some(suffix) => {
                let mut copy = file.clone().into_os_string();
                copy.push(format!(".{suffix}"));
                tokio::fs::metadata(copy).await.ok().and_then(|m| m.modified().ok()).is_some_and(|copy| copy < source)
            }
            None => false,
        };
        if !stale {
            kept.push(token.trim());
        }
    }
    match HeaderValue::from_str(&kept.join(", ")) {
        Ok(value) if !kept.is_empty() => headers.insert(ACCEPT_ENCODING, value),
        _ => headers.remove(ACCEPT_ENCODING),
    };
}

struct Site {
    root: PathBuf,
    cache_control: HeaderValue,
    build: Build,
}

/// The path a versioned URL stands for (`/v/<build>/app.css` is `/app.css`), `Err` for a build that is not the
/// current one or a file that is not versioned code.
fn unversioned(path: &str, build: &str) -> Result<String, ()> {
    let rest = &path[VERSIONED.len()..];
    let (id, file) = rest.split_once('/').ok_or(())?;
    let file = format!("/{file}");
    if id == build && is_versioned(&file) { Ok(file) } else { Err(()) }
}

/// How a file that is not versioned is cached, where that differs from what the server is told to send.
fn cache_for(path: &str) -> Option<&'static str> {
    match path {
        "/data/metro/index.json" => Some(INDEX),
        p if p.starts_with("/glyphs/") || p.starts_with("/fonts/") => Some(GLYPHS),
        _ => None,
    }
}

/// The request's validators and range are not for a page: it is made from the file and the build together.
fn drop_conditions(headers: &mut HeaderMap) {
    for name in [RANGE, IF_RANGE, IF_MODIFIED_SINCE, IF_UNMODIFIED_SINCE, IF_NONE_MATCH, IF_MATCH] {
        headers.remove(name);
    }
}

/// Whether an `If-None-Match` list names the tag, by weak comparison (RFC 9110 13.1.2): `W/` is ignored, `*` is any.
fn matches_tag(list: &str, etag: &str) -> bool {
    let opaque = |t: &str| t.trim().trim_start_matches("W/").to_string();
    let wanted = opaque(etag);
    list.split(',').any(|t| t.trim() == "*" || opaque(t) == wanted)
}

/// A page with its build filled in, as a whole answer with a validator of its own.
async fn fill_in(res: Response, build: &str, if_none_match: Option<HeaderValue>) -> Response {
    let (mut parts, body) = res.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, 8 * 1024 * 1024).await else { return StatusCode::INTERNAL_SERVER_ERROR.into_response() };
    let bytes = match std::str::from_utf8(&bytes) {
        Ok(text) => text.replace(BUILD_TOKEN, build).into_bytes(),
        Err(_) => bytes.to_vec(),
    };
    // Weak: the same page is sent in several encodings, and a proxy may re-encode it.
    let etag = format!("W/\"{}\"", &hex::encode(Sha256::digest(&bytes))[..16]);
    for name in [CONTENT_LENGTH, LAST_MODIFIED, CONTENT_RANGE] {
        parts.headers.remove(name);
    }
    parts.headers.insert(ETAG, HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("\"\"")));
    if if_none_match.as_ref().and_then(|v| v.to_str().ok()).is_some_and(|v| matches_tag(v, &etag)) {
        parts.status = StatusCode::NOT_MODIFIED;
        return Response::from_parts(parts, Body::empty());
    }
    parts.status = StatusCode::OK;
    parts.headers.insert(CONTENT_LENGTH, HeaderValue::from(bytes.len()));
    Response::from_parts(parts, Body::from(bytes))
}

async fn apply(State(site): State<Arc<Site>>, mut req: Request, next: Next) -> Response {
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
    let mut versioned = false;
    let build = site.build.id();
    let mut path = path;
    if path.starts_with(VERSIONED) {
        let Ok(file) = unversioned(&path, &build) else {
            let mut res = (StatusCode::NOT_FOUND, "this page was made for another build of the site: reload it").into_response();
            res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            return res;
        };
        match with_path(req.uri(), &file) {
            Some(uri) => *req.uri_mut() = uri,
            None => return (StatusCode::BAD_REQUEST, "bad request").into_response(),
        }
        path = file;
        versioned = true;
    }
    let page = is_page(&path);
    let if_none_match = req.headers().get(IF_NONE_MATCH).cloned();
    // A HEAD on a page is answered from the page a GET would get, so its length and tag are that page's.
    let head = page && req.method() == Method::HEAD;
    if page {
        drop_conditions(req.headers_mut());
        if head {
            *req.method_mut() = Method::GET;
        }
    }
    drop_stale_codings(&site.root, &path, req.headers_mut()).await;
    // A list of ranges is not supported: the whole file is answered.
    if req.headers().get(RANGE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.contains(',')) {
        req.headers_mut().remove(RANGE);
    }
    let mut res = next.run(req).await;
    let found = res.status().is_success();
    // A 304 stands for the 200 it validates, and replaces the headers of the stored one: the same policy.
    let kept = found || res.status() == StatusCode::NOT_MODIFIED;
    if found {
        res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(content_type(&path)));
    }
    if found && page {
        res = fill_in(res, &build, if_none_match).await;
        if head {
            *res.body_mut() = Body::empty();
        }
    }
    let cache = match (kept && versioned, cache_for(&path)) {
        (true, _) => HeaderValue::from_static(IMMUTABLE),
        (false, Some(own)) if kept => HeaderValue::from_static(own),
        _ => site.cache_control.clone(),
    };
    let headers = res.headers_mut();
    headers.insert(CACHE_CONTROL, cache);
    headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    // ServeDir already says it when it can choose a precompressed copy.
    if !headers.get_all(VARY).iter().any(|v| v.to_str().is_ok_and(|v| v.to_ascii_lowercase().contains("accept-encoding"))) {
        headers.append(VARY, HeaderValue::from_static("accept-encoding"));
    }
    res
}

/// What is worth compressing on the fly: text and the WebAssembly modules. The tiles, the glyphs and the basemap are
/// already compressed, or do not compress, and are read in ranges.
fn compressible(status: StatusCode, headers: &HeaderMap) -> bool {
    if status != StatusCode::OK {
        return false;
    }
    let kind = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    ["text/", "application/json", "application/wasm", "image/svg+xml"].iter().any(|k| kind.starts_with(k))
}

pub fn router(root: PathBuf, cache_control: String) -> Router {
    let cache_control = HeaderValue::from_str(&cache_control).unwrap_or_else(|_| HeaderValue::from_static("no-store"));
    let site = Arc::new(Site { root: root.clone(), cache_control, build: Build::new(root.clone()) });
    let files = ServeDir::new(root).append_index_html_on_directories(true).precompressed_br().precompressed_gzip();
    let compression = CompressionLayer::new()
        .quality(CompressionLevel::Fastest)
        .compress_when(|status: StatusCode, _, headers: &HeaderMap, _: &axum::http::Extensions| compressible(status, headers));
    Router::new().fallback_service(files).layer(middleware::from_fn_with_state(site, apply)).layer(compression)
}
