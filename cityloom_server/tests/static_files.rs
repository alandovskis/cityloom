//! The cases `e2e/tests/serve.spec.ts` checks, against the router itself.
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn site(cache_control: &str) -> (tempfile::TempDir, Router, Vec<u8>) {
    let dir = tempfile::tempdir().unwrap();
    let body: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(dir.path().join("map.html"), b"<link href=\"v/{{build}}/app.css\">").unwrap();
    std::fs::write(dir.path().join("range.bin"), &body).unwrap();
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
    let (status, headers, body) = get(&router, "/range.bin", Some("bytes=10-29")).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(headers[header::CONTENT_RANGE], format!("bytes 10-29/{}", whole.len()));
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    assert_eq!(body, whole[10..30]);
}

#[tokio::test]
async fn answers_an_open_ended_range_and_a_range_of_the_last_bytes() {
    let (_d, router, whole) = site("no-store");
    let (status, _, body) = get(&router, "/range.bin", Some("bytes=100-")).await;
    assert_eq!((status, body), (StatusCode::PARTIAL_CONTENT, whole[100..].to_vec()));
    let (status, _, body) = get(&router, "/range.bin", Some("bytes=-16")).await;
    assert_eq!((status, body), (StatusCode::PARTIAL_CONTENT, whole[whole.len() - 16..].to_vec()));
}

#[tokio::test]
async fn refuses_a_range_past_the_end_and_serves_the_whole_file_without_one() {
    let (_d, router, whole) = site("no-store");
    let (status, headers, body) = get(&router, "/range.bin", None).await;
    assert_eq!((status, body), (StatusCode::OK, whole.clone()));
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    let (status, headers, _) = get(&router, "/range.bin", Some("bytes=99999999-")).await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(headers[header::CONTENT_RANGE], format!("bytes */{}", whole.len()));
}

#[tokio::test]
async fn a_range_list_is_served_whole() {
    let (_d, router, whole) = site("no-store");
    let (status, _, body) = get(&router, "/range.bin", Some("bytes=0-1,5-6")).await;
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

// ---- versioned assets, cache lifetimes and compression ----

fn build_id(page: &[u8]) -> String {
    let text = String::from_utf8(page.to_vec()).unwrap();
    let start = text.find("v/").unwrap() + 2;
    text[start..start + 16].to_string()
}

fn code_site() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("map.html"), "<link href=\"v/{{build}}/app.css\"><script src=\"v/{{build}}/map.js\"></script>").unwrap();
    std::fs::write(root.join("app.css"), "body { color: red; }".repeat(50)).unwrap();
    std::fs::write(root.join("map.js"), "import './pkg/glue.js';".repeat(50)).unwrap();
    std::fs::create_dir_all(root.join("pkg")).unwrap();
    std::fs::write(root.join("pkg/glue.js"), "export default 1;".repeat(50)).unwrap();
    std::fs::write(root.join("pkg/glue_bg.wasm"), b"\0asm\x01\0\0\0".repeat(50)).unwrap();
    std::fs::create_dir_all(root.join("data/metro")).unwrap();
    std::fs::write(root.join("data/metro/index.json"), "{}".repeat(50)).unwrap();
    std::fs::write(root.join("data/metro/0_0.osm.pbf"), vec![7u8; 500]).unwrap();
    std::fs::write(root.join("data/app.js"), "not code").unwrap();
    std::fs::create_dir_all(root.join("glyphs/Font")).unwrap();
    std::fs::write(root.join("glyphs/Font/0-255.pbf"), vec![9u8; 500]).unwrap();
    let router = cityloom_server::static_files::router(root.to_path_buf(), "no-cache".to_string());
    (dir, router)
}

async fn get_with(router: &Router, path: &str, headers: &[(&str, &str)]) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::builder().uri(path);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let res = router.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let (parts, body) = res.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

#[tokio::test]
async fn a_page_names_the_build_and_every_file_it_names_is_served_for_a_year() {
    let (_d, router) = code_site();
    let (status, headers, page) = get(&router, "/map.html", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache", "the page that names the build is revalidated");
    let id = build_id(&page);
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(page, format!("<link href=\"v/{id}/app.css\"><script src=\"v/{id}/map.js\"></script>").into_bytes());
    for (file, kind) in [("app.css", "text/css"), ("map.js", "text/javascript"), ("pkg/glue.js", "text/javascript"), ("pkg/glue_bg.wasm", "application/wasm")] {
        let (status, headers, body) = get(&router, &format!("/v/{id}/{file}"), None).await;
        assert_eq!(status, StatusCode::OK, "{file}");
        assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=31536000, immutable", "{file}");
        assert!(headers[header::CONTENT_TYPE].to_str().unwrap().starts_with(kind), "{file}");
        assert_eq!(body, std::fs::read(_d.path().join(file)).unwrap(), "{file}");
    }
}

#[tokio::test]
async fn another_build_is_not_found_rather_than_served_as_the_current_one() {
    let (_d, router) = code_site();
    let id = build_id(&get(&router, "/map.html", None).await.2);
    for path in [
        "/v/0000000000000000/app.css".to_string(),
        format!("/v/{id}/data/app.js"),
        format!("/v/{id}/data/metro/0_0.osm.pbf"),
        format!("/v/{id}/map.html"),
        format!("/v/{id}"),
    ] {
        let (status, headers, _) = get(&router, &path, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store", "{path}: a refusal is never kept");
    }
}

#[tokio::test]
async fn a_change_to_any_versioned_file_changes_the_build_and_retires_the_old_urls() {
    let (d, router) = code_site();
    let before = build_id(&get(&router, "/map.html", None).await.2);
    assert_eq!(build_id(&get(&router, "/map.html", None).await.2), before, "the same files, the same build");
    // The glue and the module are what must stay together: a new module is a new build.
    std::fs::write(d.path().join("pkg/glue_bg.wasm"), b"\0asm\x01\0\0\0 changed").unwrap();
    let after = build_id(&get(&router, "/map.html", None).await.2);
    assert_ne!(after, before);
    assert_eq!(get(&router, &format!("/v/{before}/pkg/glue.js"), None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&router, &format!("/v/{after}/pkg/glue.js"), None).await.0, StatusCode::OK);
    // Data is not part of the build.
    std::fs::write(d.path().join("data/metro/0_0.osm.pbf"), vec![1u8; 10]).unwrap();
    assert_eq!(build_id(&get(&router, "/map.html", None).await.2), after);
}

#[tokio::test]
async fn a_page_is_always_answered_whole_and_revalidated_by_its_own_tag() {
    let (_d, router) = code_site();
    let (_, headers, page) = get(&router, "/map.html", None).await;
    let etag = headers[header::ETAG].to_str().unwrap().to_string();
    assert_eq!(headers[header::CONTENT_LENGTH], page.len().to_string());
    assert!(headers.get(header::LAST_MODIFIED).is_none(), "the file's date does not say when the build changed");
    // A range of the file is not a range of the page.
    let (status, _, ranged) = get(&router, "/map.html", Some("bytes=0-3")).await;
    assert_eq!((status, ranged), (StatusCode::OK, page));
    let (status, _, body) = get_with(&router, "/map.html", &[("if-none-match", &etag)]).await;
    assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0));
    // A tag of an older build, or a date, does not make a 304.
    assert_eq!(get_with(&router, "/map.html", &[("if-none-match", "\"old\"")]).await.0, StatusCode::OK);
    assert_eq!(get_with(&router, "/map.html", &[("if-modified-since", "Sun, 01 Jan 2090 00:00:00 GMT")]).await.0, StatusCode::OK);
    // And the tag follows the build.
    std::fs::write(_d.path().join("app.css"), "changed").unwrap();
    assert_eq!(get_with(&router, "/map.html", &[("if-none-match", &etag)]).await.0, StatusCode::OK);
}

#[tokio::test]
async fn data_is_cached_as_long_as_it_is_safe_to() {
    let (_d, router) = code_site();
    let cache = |path: &'static str| {
        let router = router.clone();
        async move { get(&router, path, None).await.1[header::CACHE_CONTROL].to_str().unwrap().to_string() }
    };
    // The index names the tiles' checksums: a few minutes stale only delays a refresh.
    assert_eq!(cache("/data/metro/index.json").await, "public, max-age=300, stale-while-revalidate=600");
    // A tile is not named by its content, and a cached old one would be kept under a new checksum: always revalidated.
    assert_eq!(cache("/data/metro/0_0.osm.pbf").await, "no-cache");
    assert_eq!(cache("/glyphs/Font/0-255.pbf").await, "public, max-age=86400, stale-while-revalidate=604800");
    assert_eq!(cache("/pkg/glue.js").await, "no-cache", "the unversioned name of a script is not kept");
    assert_eq!(cache("/nothing.js").await, "no-cache");
}

#[tokio::test]
async fn text_and_wasm_are_compressed_with_the_best_encoding_the_client_takes() {
    let (_d, router) = code_site();
    for (accept, encoding) in [("br, gzip", "br"), ("gzip, deflate", "gzip")] {
        for path in ["/app.css", "/map.html", "/pkg/glue_bg.wasm", "/data/metro/index.json"] {
            let (_, headers, body) = get_with(&router, path, &[("accept-encoding", accept)]).await;
            assert_eq!(headers[header::CONTENT_ENCODING], encoding, "{path} for {accept}");
            assert!(body.len() < 200, "{path} is smaller than its 800 bytes");
            assert!(headers[header::VARY].to_str().unwrap().to_lowercase().contains("accept-encoding"), "{path}");
        }
    }
    let (_, headers, _) = get(&router, "/app.css", None).await;
    assert!(headers.get(header::CONTENT_ENCODING).is_none(), "a client that asks for nothing gets the file as it is");
}

#[tokio::test]
async fn what_is_already_compressed_or_read_in_ranges_is_not_compressed_again() {
    let (_d, router) = code_site();
    for path in ["/data/metro/0_0.osm.pbf", "/glyphs/Font/0-255.pbf"] {
        let (_, headers, body) = get_with(&router, path, &[("accept-encoding", "br, gzip")]).await;
        assert!(headers.get(header::CONTENT_ENCODING).is_none(), "{path}");
        assert_eq!(body.len(), 500);
    }
}

#[tokio::test]
async fn a_file_compressed_at_build_time_is_sent_as_it_is() {
    let (d, router) = code_site();
    std::fs::write(d.path().join("pkg/glue_bg.wasm.br"), b"BROTLI-BYTES").unwrap();
    let (status, headers, body) = get_with(&router, "/pkg/glue_bg.wasm", &[("accept-encoding", "br, gzip")]).await;
    assert_eq!((status, body), (StatusCode::OK, b"BROTLI-BYTES".to_vec()));
    assert_eq!(headers[header::CONTENT_ENCODING], "br");
    assert_eq!(headers[header::CONTENT_TYPE], "application/wasm");
    // The versioned name is the same file, and the build does not count the compressed copies.
    let id = build_id(&get(&router, "/map.html", None).await.2);
    let (_, headers, body) = get_with(&router, &format!("/v/{id}/pkg/glue_bg.wasm"), &[("accept-encoding", "br")]).await;
    assert_eq!((headers[header::CONTENT_ENCODING].to_str().unwrap(), body), ("br", b"BROTLI-BYTES".to_vec()));
    assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=31536000, immutable");
}

/// The pages of the site as they are in the repository name their code only by versioned URLs, and each such URL is
/// answered. (`pkg/` and `vendor/` are built, so their files are checked only where a build has put them.)
#[tokio::test]
async fn every_page_of_the_site_names_existing_versioned_assets() {
    let web = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web");
    let router = cityloom_server::static_files::router(web.clone(), "no-cache".to_string());
    let mut checked = 0;
    for entry in std::fs::read_dir(&web).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "html")) {
        let name = entry.file_name().to_string_lossy().to_string();
        let (status, _, page) = get(&router, &format!("/{name}"), None).await;
        assert_eq!(status, StatusCode::OK, "{name}");
        let page = String::from_utf8(page).unwrap();
        assert!(!page.contains("{{"), "{name} has a token the server did not fill in");
        let mut rest = page.as_str();
        while let Some(at) = rest.find(['"', '\'']) {
            rest = &rest[at + 1..];
            let end = rest.find(['"', '\'']).unwrap_or(rest.len());
            let value = &rest[..end];
            rest = &rest[end..];
            let target = value.split('#').next().unwrap();
            let is_code = [".js", ".mjs", ".css", ".wasm"].iter().any(|x| target.ends_with(x)) || value.contains(".svg#");
            if !is_code || target.contains(':') || target.contains(' ') || target.starts_with('<') {
                continue;
            }
            // Icons that are not part of the build are named by their own names; every other file of the build is versioned.
            let source = target.rsplit('/').next().unwrap();
            if target.starts_with("favicon") || source.starts_with("logo-") {
                continue;
            }
            assert!(target.starts_with("v/") && target.len() > 19, "{name} names {target} without its build");
            let built = target.contains("/pkg/") || target.contains("/vendor/");
            let file = web.join(&target[19..]);
            if built && !file.exists() {
                continue;
            }
            assert!(file.exists(), "{name} names {target}, which is not in web/");
            let (status, headers, _) = get(&router, &format!("/{target}"), None).await;
            assert_eq!(status, StatusCode::OK, "{name}: {target}");
            assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=31536000, immutable", "{name}: {target}");
            checked += 1;
        }
    }
    assert!(checked >= 12, "the pages' scripts and styles were found ({checked})");
}

/// A 304 replaces the headers of the stored response, so it must carry the policy the 200 carried.
#[tokio::test]
async fn a_304_keeps_the_cache_policy_of_its_200() {
    let (_d, router) = code_site();
    let id = build_id(&get(&router, "/map.html", None).await.2);
    for (path, policy) in [
        (format!("/v/{id}/app.css"), "public, max-age=31536000, immutable"),
        ("/data/metro/index.json".to_string(), "public, max-age=300, stale-while-revalidate=600"),
        ("/glyphs/Font/0-255.pbf".to_string(), "public, max-age=86400, stale-while-revalidate=604800"),
    ] {
        let (status, headers, _) = get(&router, &path, None).await;
        assert_eq!((status, headers[header::CACHE_CONTROL].to_str().unwrap()), (StatusCode::OK, policy), "{path}");
        let etag = headers[header::ETAG].to_str().unwrap().to_string();
        let (status, headers, body) = get_with(&router, &path, &[("if-none-match", &etag)]).await;
        assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0), "{path}");
        assert_eq!(headers[header::CACHE_CONTROL], policy, "{path}: the 304 says what the 200 said");
        assert_eq!(headers[header::ETAG].to_str().unwrap(), etag, "{path}");
        let modified = get(&router, &path, None).await.1[header::LAST_MODIFIED].to_str().unwrap().to_string();
        let (status, headers, _) = get_with(&router, &path, &[("if-modified-since", &modified)]).await;
        assert_eq!((status, headers[header::CACHE_CONTROL].to_str().unwrap()), (StatusCode::NOT_MODIFIED, policy), "{path}: by date");
    }
}

#[tokio::test]
async fn a_page_is_revalidated_by_a_weak_comparison_of_its_tag_whatever_the_list() {
    let (_d, router) = code_site();
    let (_, headers, _) = get(&router, "/map.html", None).await;
    let etag = headers[header::ETAG].to_str().unwrap().to_string();
    let opaque = etag.trim_start_matches("W/").to_string();
    // A proxy that re-encodes the body weakens the tag it passes on, and the browser sends that one back.
    for sent in [format!("W/{opaque}"), opaque.clone(), format!("\"other\", W/{opaque}"), format!("W/\"other\" ,{opaque}"), "*".to_string()] {
        let (status, _, body) = get_with(&router, "/map.html", &[("if-none-match", &sent)]).await;
        assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0), "{sent}");
    }
    assert_eq!(get_with(&router, "/map.html", &[("if-none-match", "\"other\", W/\"another\"")]).await.0, StatusCode::OK);
}

#[tokio::test]
async fn a_304_for_a_page_is_not_marked_as_compressed_and_says_vary_once() {
    let (_d, router) = code_site();
    let etag = get(&router, "/map.html", None).await.1[header::ETAG].to_str().unwrap().to_string();
    let (status, headers, _) = get_with(&router, "/map.html", &[("if-none-match", &etag), ("accept-encoding", "br")]).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(headers.get(header::CONTENT_ENCODING).is_none(), "an empty body has no encoding");
    for path in ["/map.html", "/app.css", "/pkg/glue_bg.wasm"] {
        let (_, headers, _) = get_with(&router, path, &[("accept-encoding", "br")]).await;
        let varies = headers
            .get_all(header::VARY)
            .iter()
            .flat_map(|v| v.to_str().unwrap().split(','))
            .filter(|t| t.trim().eq_ignore_ascii_case("accept-encoding"))
            .count();
        assert_eq!(varies, 1, "{path}");
    }
}

#[tokio::test]
async fn a_head_on_a_page_says_what_the_get_says() {
    let (_d, router) = code_site();
    let (_, get_headers, page) = get(&router, "/map.html", None).await;
    let res = router.clone().oneshot(Request::builder().method("HEAD").uri("/map.html").body(Body::empty()).unwrap()).await.unwrap();
    let (parts, body) = res.into_parts();
    assert_eq!(parts.status, StatusCode::OK);
    assert!(body.collect().await.unwrap().to_bytes().is_empty());
    assert_eq!(parts.headers[header::ETAG], get_headers[header::ETAG]);
    assert_eq!(parts.headers[header::CONTENT_LENGTH], page.len().to_string());
    assert_eq!(parts.headers[header::CONTENT_TYPE], get_headers[header::CONTENT_TYPE]);
    assert_eq!(parts.headers[header::CACHE_CONTROL], "no-cache");
}

#[tokio::test]
async fn a_path_that_climbs_out_from_under_the_build_is_refused_or_kept_inside() {
    let (_d, router) = code_site();
    let id = build_id(&get(&router, "/map.html", None).await.2);
    // Out of the root: refused, before the prefix is looked at.
    for path in [format!("/v/{id}/%2e%2e/%2e%2e/%2e%2e/etc/passwd"), format!("/v/{id}/../../../etc/passwd")] {
        assert_eq!(get(&router, &path, None).await.0, StatusCode::FORBIDDEN, "{path}");
    }
    // Inside the root: collapsed first, as a browser would, so the answer is that of the path it comes to (a 404 here)
    // and never a file under the build's name: nothing is kept for a year on a name it was not asked for.
    assert_eq!(get(&router, &format!("/v/{id}/../../etc/passwd"), None).await.0, StatusCode::NOT_FOUND);
    for path in [format!("/v/{id}/%2e%2e/%2e%2e/map.html"), format!("/v/{id}/pkg/../../data/app.js")] {
        let (_, headers, _) = get(&router, &path, None).await;
        assert_ne!(headers[header::CACHE_CONTROL], "public, max-age=31536000, immutable", "{path}");
    }
    // A collapsed path that stays under the build is the file it names.
    assert_eq!(get(&router, &format!("/v/{id}/pkg/../app.css"), None).await.0, StatusCode::OK);
}
