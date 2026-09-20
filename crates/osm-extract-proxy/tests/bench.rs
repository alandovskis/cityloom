//! The CI benchmark (`code-generation-plan.md` step 15.6): 200 misses and
//! 1,000 hits against a synthetic build, asserting the p95 budgets
//! (performance-design.md): miss ≤ 300 ms, hit ≤ 30 ms. `#[ignore]`d —
//! the slow CI tier runs it explicitly:
//! `cargo test -p osm-extract-proxy --release --test bench -- --ignored`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::Request;
use osm_extract_proxy::encode::encode;
use osm_extract_proxy::geo::CanonicalBox;
use osm_extract_proxy::grid::{CoverageIndex, GridSpec};
use osm_extract_proxy::http::{AppState, ReadyData};
use osm_extract_proxy::manifest::Digest;
use osm_extract_proxy::manifest::{BuildId, CellEntry, Manifest, RegionRecord};
use osm_extract_proxy::osm::{Clip, Node, Way};
use osm_extract_proxy::store::{Store, StoreWriter};
use tower::ServiceExt;

const BUILD: &str = "bench-build";

fn percentile(mut samples: Vec<Duration>, p: f64) -> Duration {
    samples.sort();
    let idx = ((samples.len() as f64 - 1.0) * p).round() as usize;
    samples[idx]
}

fn make_state(dir: &std::path::Path) -> Arc<AppState> {
    let state = Arc::new(AppState::new(
        BUILD.to_string(),
        12,
        Duration::from_millis(3_000),
        32 * 1024 * 1024,
        1_000_000,
        1_000_000,
        4,
    ));
    let grid = GridSpec::default_cells();
    // A wide covered rectangle so many distinct one-cell boxes are misses
    // on distinct keys.
    let region = CanonicalBox::parse("0,0,2.00,0.01").unwrap();
    let touched = grid.touched(&region);
    let mut writer = StoreWriter::create(dir.join("store.bin")).unwrap();
    let mut cells = Vec::new();
    // Each column's cell gets its own way, placed inside that column's own
    // 0.005°-wide miss box below, so every one of the 200 distinct miss
    // requests is a genuine 200 rather than a "nothing mapped" 404.
    for (i, cell_id) in touched.iter().enumerate() {
        let raw = (i as i64) * 100_000 + 10_000; // degree (i*0.01 + 0.001) * 1e7
        let clip = Clip {
            nodes: vec![
                Node {
                    id: 2 * i as i64 + 1,
                    lat: 10_000,
                    lon: raw,
                    tags: Vec::new(),
                },
                Node {
                    id: 2 * i as i64 + 2,
                    lat: 20_000,
                    lon: raw + 1_000,
                    tags: Vec::new(),
                },
            ],
            ways: vec![Way {
                id: i as i64 + 1,
                refs: vec![2 * i as i64 + 1, 2 * i as i64 + 2],
                tags: vec![("highway".to_string(), "residential".to_string())],
            }],
        };
        let (offset, length, digest) = writer.write_cell(&encode(&clip)).unwrap();
        cells.push(CellEntry {
            cell_id,
            offset,
            length,
            digest,
            way_count: 1,
            node_count: 2,
        });
    }
    writer.flush().unwrap();
    let store = Store::open(dir.join("store.bin"), &cells).unwrap();
    let coverage = CoverageIndex::from_rectangles(&[touched.rect()]);
    let manifest = Manifest {
        build_id: BuildId::parse(&"b".repeat(64)).unwrap(),
        built_at: "2026-09-15T00:00:00Z".to_string(),
        grid_spec: grid,
        regions: vec![RegionRecord {
            source_path: "bench".to_string(),
            source_published_at: "2026-09-14T00:00:00Z".to_string(),
            source_digest: Digest::from_bytes([1; 32]),
            source_bytes: 0,
            bounds: region.key_text(),
        }],
        filter_profile: "highway-v1".to_string(),
        cell_count: cells.len() as u64,
        total_bytes: cells.iter().map(|c| c.length).sum(),
        coverage: vec![touched.rect()],
        cells,
    };
    state.set_ready(ReadyData {
        manifest,
        store,
        coverage,
        grid,
    });
    state
}

async fn request(app: &axum::Router, bbox: &str) -> Duration {
    let start = Instant::now();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/extract?bbox={bbox}"))
                .header("x-client-build", BUILD)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200, "the bench fixture must always hit");
    start.elapsed()
}

#[tokio::test]
#[ignore = "the slow CI tier runs this explicitly, in --release"]
async fn p95_latency_is_within_budget_for_misses_and_hits() {
    let dir = tempfile::tempdir().unwrap();
    let state = make_state(dir.path());
    let app = osm_extract_proxy::http::router(state);

    let miss_boxes: Vec<String> = (0..200)
        .map(|i| {
            let lon = i as f64 * 0.01;
            format!("{lon:.5},0,{:.5},0.005", lon + 0.005)
        })
        .collect();
    let mut miss_latencies = Vec::with_capacity(miss_boxes.len());
    for bbox in &miss_boxes {
        miss_latencies.push(request(&app, bbox).await);
    }

    let hit_box = &miss_boxes[0];
    let mut hit_latencies = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        hit_latencies.push(request(&app, hit_box).await);
    }

    let miss_p95 = percentile(miss_latencies, 0.95);
    let hit_p95 = percentile(hit_latencies, 0.95);
    println!("miss p95: {miss_p95:?}, hit p95: {hit_p95:?}");

    assert!(
        miss_p95 <= Duration::from_millis(300),
        "miss p95 {miss_p95:?} exceeds the 300 ms budget (performance-design.md)"
    );
    assert!(
        hit_p95 <= Duration::from_millis(30),
        "hit p95 {hit_p95:?} exceeds the 30 ms budget (performance-design.md)"
    );
}
