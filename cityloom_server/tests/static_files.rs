//! The cases `e2e/tests/serve.spec.ts` checks, against the router itself.
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn site(cache_control: &str) -> (tempfile::TempDir, Router, Vec<u8>) {
    let dir = tempfile::tempdir().unwrap();
    let body: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(dir.path().join("map.html"), &body).unwrap();
    std::fs::write(dir.path().join("index.html"), b"home").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/index.html"), b"sub home").unwrap();
    for (name, bytes) in
        [("a.json", &b"{}"[..]), ("a.wasm", b"\0asm"), ("a.pmtiles", b"PM"), ("a.pbf", b"pb"), ("a.mjs", b"x"), ("a.woff2", b"w"), ("a.unknown", b"?")]
    {
        std::fs::write(dir.path().join(name), bytes).unwrap();
    }
    let router = cityloom_server::static_files::router(dir.path().to_path_buf(), cache_control.to_string());
    (dir, router, body)
}

async fn get(router: &Router, path: &str, range: Option<&str>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::builder().uri(path);
    if let Some(r) = range {
        req = req.header(header::RANGE, r);
    }
    let res = router.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let (parts, body) = res.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

#[tokio::test]
async fn answers_a_range_with_just_those_bytes_and_says_where_they_lie() {
    let (_d, router, whole) = site("no-store");
    let (status, headers, body) = get(&router, "/map.html", Some("bytes=10-29")).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(headers[header::CONTENT_RANGE], format!("bytes 10-29/{}", whole.len()));
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    assert_eq!(body, whole[10..30]);
}

#[tokio::test]
async fn answers_an_open_ended_range_and_a_range_of_the_last_bytes() {
    let (_d, router, whole) = site("no-store");
    let (status, _, body) = get(&router, "/map.html", Some("bytes=100-")).await;
    assert_eq!((status, body), (StatusCode::PARTIAL_CONTENT, whole[100..].to_vec()));
    let (status, _, body) = get(&router, "/map.html", Some("bytes=-16")).await;
    assert_eq!((status, body), (StatusCode::PARTIAL_CONTENT, whole[whole.len() - 16..].to_vec()));
}

#[tokio::test]
async fn refuses_a_range_past_the_end_and_serves_the_whole_file_without_one() {
    let (_d, router, whole) = site("no-store");
    let (status, headers, body) = get(&router, "/map.html", None).await;
    assert_eq!((status, body), (StatusCode::OK, whole.clone()));
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    let (status, headers, _) = get(&router, "/map.html", Some("bytes=99999999-")).await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(headers[header::CONTENT_RANGE], format!("bytes */{}", whole.len()));
}

#[tokio::test]
async fn a_range_list_is_served_whole() {
    let (_d, router, whole) = site("no-store");
    let (status, _, body) = get(&router, "/map.html", Some("bytes=0-1,5-6")).await;
    assert_eq!((status, body), (StatusCode::OK, whole));
}

#[tokio::test]
async fn refuses_a_malformed_escape_and_keeps_serving() {
    let (_d, router, _) = site("no-store");
    assert_eq!(get(&router, "/%E0%A4%A", None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(get(&router, "/map.html", None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn refuses_a_path_that_leaves_the_root() {
    let (_d, router, _) = site("no-store");
    assert_eq!(get(&router, "/%2e%2e/%2e%2e/etc/passwd", None).await.0, StatusCode::FORBIDDEN);
    // A `..` that stays inside the root is collapsed, as a browser would, and served: ServeDir must never see it.
    assert_eq!(get(&router, "/sub/%2E%2E/map.html", None).await.0, StatusCode::OK);
    assert_eq!(get(&router, "/sub/%2e%2e/sub/", None).await.2, b"sub home");
    // An encoded slash is a separator once decoded.
    assert_eq!(get(&router, "/sub%2f..%2fmap.html", None).await.0, StatusCode::OK);
    assert_eq!(get(&router, "/sub/%2e%2e/map.html?x=1", None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn a_path_ending_in_a_slash_serves_its_index() {
    let (_d, router, _) = site("no-store");
    assert_eq!(get(&router, "/", None).await.2, b"home");
    assert_eq!(get(&router, "/sub/", None).await.2, b"sub home");
}

#[tokio::test]
async fn a_missing_file_is_404() {
    let (_d, router, _) = site("no-store");
    assert_eq!(get(&router, "/nope.html", None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn serves_each_extension_with_its_content_type() {
    let (_d, router, _) = site("no-store");
    for (path, expected) in [
        ("/map.html", "text/html; charset=utf-8"),
        ("/a.json", "application/json; charset=utf-8"),
        ("/a.wasm", "application/wasm"),
        ("/a.pmtiles", "application/octet-stream"),
        ("/a.pbf", "application/x-protobuf"),
        ("/a.mjs", "text/javascript; charset=utf-8"),
        ("/a.woff2", "font/woff2"),
        ("/a.unknown", "application/octet-stream"),
    ] {
        assert_eq!(get(&router, path, None).await.1[header::CONTENT_TYPE], expected, "{path}");
    }
}

#[tokio::test]
async fn cache_control_is_the_one_given() {
    let (_d, router, _) = site("no-cache");
    assert_eq!(get(&router, "/map.html", None).await.1[header::CACHE_CONTROL], "no-cache");
}
