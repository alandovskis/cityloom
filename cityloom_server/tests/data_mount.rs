//! `/data/metro/*` and `/data/basemap/*` from an unpacked area, with the rules `static_files` gives every file.
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use cityloom_prepare::archive::{self, PackInputs};
use cityloom_prepare::metro_index::Grid;
use cityloom_server::config::Config;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// A site root with its own `data/` (which must not be used for the two mounted prefixes) and an area directory.
fn site(cache_control: &str, mounted: bool) -> (tempfile::TempDir, Router) {
    let tmp = tempfile::tempdir().unwrap();
    let (root, area) = (tmp.path().join("web"), tmp.path().join("area"));
    for dir in [root.join("data/metro"), root.join("data/basemap"), root.join("data/other"), area.join("metro"), area.join("basemap")] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(root.join("index.html"), "home").unwrap();
    std::fs::write(root.join("data/metro/index.json"), r#"{"from":"web"}"#).unwrap();
    std::fs::write(root.join("data/metro/9_9.osm.pbf"), b"web tile").unwrap();
    std::fs::write(root.join("data/other/a.json"), r#"{"other":true}"#).unwrap();
    std::fs::write(area.join("metro/index.json"), r#"{"from":"area"}"#).unwrap();
    std::fs::write(area.join("metro/0_0.osm.pbf"), b"area tile 0_0").unwrap();
    std::fs::write(area.join("basemap/montreal.pmtiles"), (0..=255u8).collect::<Vec<_>>()).unwrap();
    std::fs::write(area.join(".index-sha256"), "secret-ish").unwrap();
    let router = cityloom_server::static_files::router_with_data(root, mounted.then_some(area), cache_control.to_string());
    (tmp, router)
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

fn header_of(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers.get(name).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default()
}

#[tokio::test]
async fn the_two_directories_come_from_the_area_and_not_from_web() {
    let (_tmp, router) = site("no-cache", true);
    let (status, _, body) = get(&router, "/data/metro/index.json", None).await;
    assert_eq!((status, body), (StatusCode::OK, br#"{"from":"area"}"#.to_vec()));
    let (status, headers, body) = get(&router, "/data/metro/0_0.osm.pbf", None).await;
    assert_eq!((status, body), (StatusCode::OK, b"area tile 0_0".to_vec()));
    assert_eq!(header_of(&headers, "content-type"), "application/x-protobuf");
    let (status, _, body) = get(&router, "/data/basemap/montreal.pmtiles", None).await;
    assert_eq!((status, body.len()), (StatusCode::OK, 256));
}

#[tokio::test]
async fn index_json_keeps_its_cache_rule_and_a_tile_the_sites() {
    let (_tmp, router) = site("no-cache", true);
    let (_, headers, _) = get(&router, "/data/metro/index.json", None).await;
    assert_eq!(header_of(&headers, "cache-control"), "public, max-age=300, stale-while-revalidate=600");
    assert_eq!(header_of(&headers, "content-type"), "application/json; charset=utf-8");
    for path in ["/data/metro/index.json", "/data/metro/0_0.osm.pbf", "/data/basemap/montreal.pmtiles"] {
        let (_, headers, _) = get(&router, path, None).await;
        assert_eq!(header_of(&headers, "accept-ranges"), "bytes", "{path}");
        assert!(header_of(&headers, "vary").to_ascii_lowercase().contains("accept-encoding"), "{path}");
        if !path.ends_with("index.json") {
            assert_eq!(header_of(&headers, "cache-control"), "no-cache", "{path}");
        }
    }
}

#[tokio::test]
async fn the_basemap_is_served_in_ranges() {
    let (_tmp, router) = site("no-cache", true);
    let path = "/data/basemap/montreal.pmtiles";
    let (status, headers, body) = get(&router, path, Some("bytes=10-29")).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(header_of(&headers, "content-range"), "bytes 10-29/256");
    assert_eq!(body, (10..30u8).collect::<Vec<_>>());
    let (status, _, body) = get(&router, path, Some("bytes=-16")).await;
    assert_eq!((status, body), (StatusCode::PARTIAL_CONTENT, (240..=255u8).collect::<Vec<_>>()));
    let (status, ..) = get(&router, path, Some("bytes=999-")).await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
}

#[tokio::test]
async fn a_file_missing_from_the_area_is_a_404_not_web_data() {
    let (_tmp, router) = site("no-cache", true);
    assert_eq!(get(&router, "/data/metro/9_9.osm.pbf", None).await.0, StatusCode::NOT_FOUND);
    let (_tmp, router) = site("no-cache", false);
    let (status, _, body) = get(&router, "/data/metro/9_9.osm.pbf", None).await;
    assert_eq!((status, body), (StatusCode::OK, b"web tile".to_vec()));
}

#[tokio::test]
async fn other_data_paths_and_the_site_are_unchanged() {
    let (_tmp, router) = site("no-cache", true);
    let (status, _, body) = get(&router, "/data/other/a.json", None).await;
    assert_eq!((status, body), (StatusCode::OK, br#"{"other":true}"#.to_vec()));
    let (status, _, body) = get(&router, "/", None).await;
    assert_eq!((status, body), (StatusCode::OK, b"home".to_vec()));
    for path in ["/data/metro", "/data/metro/"] {
        let (status, _, body) = get(&router, path, None).await;
        assert!(!status.is_success() || body.is_empty(), "{path}");
        assert!(!String::from_utf8_lossy(&body).contains("secret-ish"), "{path}");
    }
}

#[tokio::test]
async fn a_path_leaving_the_area_is_refused() {
    let (_tmp, router) = site("no-cache", true);
    for (path, expected) in [
        ("/data/metro/../../area/.index-sha256", StatusCode::NOT_FOUND),
        ("/data/metro/%2e%2e/%2e%2e/%2e%2e/etc/passwd", StatusCode::FORBIDDEN),
        ("/data/metro/../../../etc/passwd", StatusCode::FORBIDDEN),
        ("/.index-sha256", StatusCode::NOT_FOUND),
        ("/data/metro/.index-sha256", StatusCode::NOT_FOUND),
        ("/data/metro/%2e%2e/.index-sha256", StatusCode::NOT_FOUND),
    ] {
        let (status, _, body) = get(&router, path, None).await;
        assert_eq!(status, expected, "{path}");
        assert!(!String::from_utf8_lossy(&body).contains("secret-ish"), "{path}");
    }
}

#[tokio::test]
async fn without_a_mount_nothing_changes() {
    let (_tmp, router) = site("no-store", false);
    let (status, headers, body) = get(&router, "/data/metro/index.json", None).await;
    assert_eq!((status, body), (StatusCode::OK, br#"{"from":"web"}"#.to_vec()));
    assert_eq!(header_of(&headers, "cache-control"), "public, max-age=300, stale-while-revalidate=600");
}

#[tokio::test]
async fn an_encoded_slash_or_nul_in_the_area_path_is_refused() {
    let (_tmp, router) = site("no-cache", true);
    for path in ["/data/metro/0_0.osm.pbf%00", "/data/metro/a%2fb", "/data/metro/%2e%2e%2f.index-sha256"] {
        let (status, _, body) = get(&router, path, None).await;
        assert!(status.is_client_error(), "{path}: {status}");
        assert!(!String::from_utf8_lossy(&body).contains("secret-ish"), "{path}");
    }
}

/// The real server: `unpack_areas` sets the mount and `app::router` serves from it.
#[tokio::test]
async fn the_app_router_serves_the_unpacked_area() {
    const GRID: Grid = Grid { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, nx: 4, ny: 3 };
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(tmp.path().join("areas")).unwrap();
    std::fs::write(work.join("0_0.osm.pbf"), b"unpacked tile").unwrap();
    std::fs::write(work.join("montreal.pmtiles"), [7u8; 3000]).unwrap();
    let roads = vec![(0u16, 0u16, work.join("0_0.osm.pbf"))];
    let maps = vec![work.join("montreal.pmtiles")];
    let inputs = PackInputs { area: "montreal", source: "x", source_time: 0, created: 0, grid: GRID, roads: &roads, basemaps: &maps };
    archive::pack(&inputs, &tmp.path().join("areas/montreal.cityloom"), &mut |_, _| {}).unwrap();

    let vars = [
        ("CITYLOOM_AREAS", tmp.path().join("areas").display().to_string()),
        ("DATA_ROOT", tmp.path().join("data").display().to_string()),
        ("WEB_ROOT", tmp.path().join("web").display().to_string()),
    ];
    let mut config = Config::from_lookup(&|k| vars.iter().find(|(name, _)| *name == k).map(|(_, v)| v.clone())).unwrap();
    std::fs::create_dir_all(tmp.path().join("web/data/metro")).unwrap();
    std::fs::write(tmp.path().join("web/data/metro/0_0.osm.pbf"), b"web tile").unwrap();
    cityloom_server::unpack_areas(&mut config).await.unwrap();
    let router = cityloom_server::app::router(&config, None);

    let (status, _, body) = get(&router, "/data/metro/0_0.osm.pbf", None).await;
    assert_eq!((status, body), (StatusCode::OK, b"unpacked tile".to_vec()));
    let (status, headers, body) = get(&router, "/data/metro/index.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("0_0"));
    assert_eq!(header_of(&headers, "cache-control"), "public, max-age=300, stale-while-revalidate=600");
    let (status, _, body) = get(&router, "/data/basemap/montreal.pmtiles", None).await;
    assert_eq!((status, body.len()), (StatusCode::OK, 3000));
    for path in ["/data/metro/.index-sha256", "/data/metro/../.index-sha256", "/data/metro/%2e%2e/%2e%2e/%2e%2e/etc/passwd"] {
        let (status, _, _) = get(&router, path, None).await;
        assert!(!status.is_success(), "{path}");
    }
}
