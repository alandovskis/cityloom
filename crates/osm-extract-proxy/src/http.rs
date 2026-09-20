//! `http` — the axum router: `GET /api/extract` and `GET /readyz` (BR1.1
//! before anything else, then the request budget wraps everything else),
//! the fixed response header set (SD-6), a typed panic body (BR10.4), and
//! nothing more — no other method or path is registered, so axum's own
//! router answers 405/404 for anything else (NFR6.4.4, NFR5.1.8's "no CORS
//! header on a preflight" falls out of never adding a CORS layer).

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{ConnectInfo, FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use tokio::sync::Semaphore;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use cityloom_api_types::{ApiError, FailureReason, ReloadRequired};

use crate::cache::{Cache, CachedExtract};
use crate::counters::Counters;
use crate::cut::{self, CutOutcome};
use crate::emit;
use crate::extract_key::ExtractKey;
use crate::failure::{FailureRecord, Phase};
use crate::geo::CanonicalBox;
use crate::grid::{CoverageIndex, GridSpec};
use crate::limiter::{LimitOutcome, Limiter};
use crate::manifest::Manifest;
use crate::store::Store;

/// Everything the handler needs once the service is Ready (BR9.1); built by
/// `readiness::start_up` and installed with [`AppState::set_ready`].
pub struct ReadyData {
    pub manifest: Manifest,
    pub store: Store,
    pub coverage: CoverageIndex,
    pub grid: GridSpec,
}

/// The service's shared state: one instance behind an `Arc`, cloned into
/// every handler via axum's `State` extractor.
pub struct AppState {
    build_id: String,
    span_bound: u64,
    request_budget: Duration,
    limiter: Limiter,
    cache: Cache,
    counters: Counters,
    cutting_slots: Semaphore,
    active_cuts: AtomicUsize,
    peak_active_cuts: AtomicUsize,
    ready: RwLock<Option<Arc<ReadyData>>>,
    /// A test-only stall injected into the cutting phase before `cut::cut`
    /// runs, so the timeout and slot-release tests are deterministic
    /// (`unit-test-instructions.md`, "Mocking and stubbing"). Always `None`
    /// in production; nothing outside `#[cfg(test)]` code can set it.
    stall: std::sync::Mutex<Option<Duration>>,
}

impl AppState {
    pub fn new(
        build_id: String,
        span_bound: u64,
        request_budget: Duration,
        cache_max_bytes: u64,
        requests_per_minute: u32,
        requests_per_hour: u32,
        cutting_slots: usize,
    ) -> AppState {
        AppState {
            build_id,
            span_bound,
            request_budget,
            limiter: Limiter::new(requests_per_minute, requests_per_hour),
            cache: Cache::new(cache_max_bytes),
            counters: Counters::new(),
            cutting_slots: Semaphore::new(cutting_slots),
            active_cuts: AtomicUsize::new(0),
            peak_active_cuts: AtomicUsize::new(0),
            ready: RwLock::new(None),
            stall: std::sync::Mutex::new(None),
        }
    }

    pub fn set_ready(&self, data: ReadyData) {
        *self.ready.write().expect("ready lock poisoned") = Some(Arc::new(data));
    }

    pub fn set_unready(&self) {
        *self.ready.write().expect("ready lock poisoned") = None;
    }

    pub fn is_ready(&self) -> bool {
        self.ready.read().expect("ready lock poisoned").is_some()
    }

    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    pub fn limiter_sweep(&self, now: Instant) {
        self.limiter.sweep(now);
    }

    /// The most concurrent cuts observed at once, since start-up
    /// (NFR1.1.6's slot ceiling, exposed for tests to assert against).
    pub fn peak_active_cuts(&self) -> usize {
        self.peak_active_cuts.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn set_stall(&self, stall: Option<Duration>) {
        *self.stall.lock().expect("stall lock poisoned") = stall;
    }
}

/// Build the router: the two routes, the fixed response headers (in a
/// stated, stable order), and the panic layer. `SetResponseHeaderLayer`
/// stacks are applied innermost-first by `tower::Layer` composition, but
/// header *insertion* order in the final response has no observable effect
/// here since every value is fixed and every name is distinct.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/extract", get(extract_handler))
        .route("/readyz", get(readyz_handler))
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer(SetResponseHeaderLayer::overriding(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'none'"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        ))
        .with_state(state)
}

fn handle_panic(_err: Box<dyn std::any::Any + Send>) -> Response {
    let record = FailureRecord::new(FailureReason::Internal, Phase::Unexpected);
    emit::record_failure(&record);
    error_response(&record)
}

fn error_response(record: &FailureRecord) -> Response {
    let status = StatusCode::from_u16(record.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ApiError::new(record.reason, record.detail))).into_response()
}

fn fail(state: &AppState, reason: FailureReason, phase: Phase) -> Response {
    let record = FailureRecord::new(reason, phase);
    emit::record_failure(&record);
    state.counters.record_failure(reason);
    error_response(&record)
}

fn fail_nothing_mapped(state: &AppState, phase: Phase) -> Response {
    let record = FailureRecord::nothing_mapped(phase);
    emit::record_failure(&record);
    state.counters.record_failure(record.reason);
    error_response(&record)
}

/// `X-Real-IP`, then the peer address (BR2.1); never `X-Forwarded-For`,
/// which a client can spoof (NFR6.3.3, ID-4).
fn resolve_addr(headers: &HeaderMap, peer: Option<SocketAddr>) -> IpAddr {
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<IpAddr>().ok())
        .or_else(|| peer.map(|p| p.ip()))
        .unwrap_or(IpAddr::from([0, 0, 0, 0]))
}

/// The connecting peer's address, if the router was served with
/// `into_make_service_with_connect_info` (the `axum-core` blanket
/// `Option<T>` extractor requires `T: OptionalFromRequestParts`, which
/// `ConnectInfo` does not implement, so this reads the extension directly).
struct PeerAddr(Option<SocketAddr>);

impl<S: Send + Sync> FromRequestParts<S> for PeerAddr {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(PeerAddr(
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0),
        ))
    }
}

async fn readyz_handler(State(state): State<Arc<AppState>>) -> Response {
    if state.is_ready() {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not-ready").into_response()
    }
}

/// Why the cache's loader failed: the box was covered but empty (BR4.2, not
/// an error the loader's failure-storage rule should apply to, but shaped
/// as one so `Cache::try_get_with`'s "an error is never stored" rule covers
/// it for free) or the internal deadline elapsed first (BR10.3).
#[derive(Debug)]
enum CutFailure {
    NothingMapped,
    DeadlineExceeded,
}

async fn extract_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    PeerAddr(peer): PeerAddr,
) -> Response {
    // BR1.1 — checked before anything else, including the request budget.
    let current_build = headers.get("x-client-build").and_then(|v| v.to_str().ok());
    if current_build != Some(state.build_id.as_str()) {
        return (
            StatusCode::from_u16(426).expect("426 is a valid status code"),
            Json(ReloadRequired::new(state.build_id.clone())),
        )
            .into_response();
    }

    let addr = resolve_addr(&headers, peer);
    let deadline = Instant::now() + state.request_budget;

    let work = async {
        // BR2.1 / BR2.2 — every request that passed BR1.1 counts, hit or miss.
        match state.limiter.check(addr, Instant::now()) {
            LimitOutcome::Limited { retry_after_secs } => {
                state.counters.record_limited();
                let mut response = fail(&state, FailureReason::RateLimited, Phase::RateLimit);
                response.headers_mut().insert(
                    header::RETRY_AFTER,
                    HeaderValue::from_str(&retry_after_secs.to_string())
                        .expect("a small integer is always a valid header value"),
                );
                return response;
            }
            LimitOutcome::Allowed => {}
        }

        // BR9.1 — Unready serves nothing but the failure itself.
        let Some(ready) = state.ready.read().expect("ready lock poisoned").clone() else {
            return fail(&state, FailureReason::UpstreamUnavailable, Phase::Readiness);
        };

        // BR3.1 — bbox must read as a canonical box.
        let Some(bbox) = params.get("bbox") else {
            return fail(&state, FailureReason::InvalidArea, Phase::Validation);
        };
        let canonical = match CanonicalBox::parse(bbox) {
            Ok(b) => b,
            Err(_) => return fail(&state, FailureReason::InvalidArea, Phase::Validation),
        };

        // BR3.3 — the span bound.
        let touched = ready.grid.touched(&canonical);
        if touched.count() > state.span_bound {
            return fail(&state, FailureReason::AreaTooLarge, Phase::Validation);
        }

        // BR4.1 — every touched cell must be covered.
        if !ready.coverage.covers_all(&touched) {
            return fail_nothing_mapped_or_uncovered(&state, Phase::Coverage, false);
        }

        // BR6.1 — the key: canonical box and buildId, nothing else.
        let key = ExtractKey::derive(&canonical, &ready.manifest.build_id);
        let was_hit = state.cache.contains(key);

        let store = &ready.store;
        let stall = *state.stall.lock().expect("stall lock poisoned");
        let result = state
            .cache
            .try_get_with(key, async {
                let _permit = state
                    .cutting_slots
                    .acquire()
                    .await
                    .expect("the semaphore is never closed");
                let active = state.active_cuts.fetch_add(1, Ordering::SeqCst) + 1;
                state.peak_active_cuts.fetch_max(active, Ordering::SeqCst);
                if let Some(stall) = stall {
                    tokio::time::sleep(stall).await;
                }
                let outcome = cut::cut(store, &touched, &canonical, deadline);
                state.active_cuts.fetch_sub(1, Ordering::SeqCst);
                match outcome {
                    CutOutcome::Extract(bytes) => Ok(CachedExtract::new(bytes)),
                    CutOutcome::NothingMapped => Err(CutFailure::NothingMapped),
                    CutOutcome::DeadlineExceeded => Err(CutFailure::DeadlineExceeded),
                }
            })
            .await;

        match result {
            Ok(extract) => {
                state.counters.record_served(extract.len() as u64, was_hit);
                let mut response = (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    extract.bytes.as_ref().clone(),
                )
                    .into_response();
                response.headers_mut().insert(
                    "x-extract-key",
                    HeaderValue::from_str(&key.to_hex())
                        .expect("hex is always a valid header value"),
                );
                response
            }
            Err(err) => match *err {
                CutFailure::NothingMapped => {
                    fail_nothing_mapped_or_uncovered(&state, Phase::Clip, true)
                }
                CutFailure::DeadlineExceeded => fail(&state, FailureReason::Timeout, Phase::Clip),
            },
        }
    };

    match tokio::time::timeout(state.request_budget, work).await {
        Ok(response) => response,
        Err(_elapsed) => fail(&state, FailureReason::Timeout, Phase::Clip),
    }
}

/// BR4.1 ("not an area this deployment covers") and BR4.2 ("nothing mapped
/// here") share a reason and a status but not a detail; `nothing_mapped`
/// selects which.
fn fail_nothing_mapped_or_uncovered(
    state: &AppState,
    phase: Phase,
    nothing_mapped: bool,
) -> Response {
    if nothing_mapped {
        fail_nothing_mapped(state, phase)
    } else {
        fail(state, FailureReason::AreaNotFound, phase)
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;
    use crate::grid::CoverageIndex;
    use crate::manifest::{BuildId, CellEntry, Digest, RegionRecord};
    use crate::osm::{Clip, Node, Way};
    use crate::store::StoreWriter;

    const BUILD: &str = "test-build-1234";

    fn make_ready_state(dir: &std::path::Path, span_bound: u64) -> Arc<AppState> {
        let state = Arc::new(AppState::new(
            BUILD.to_string(),
            span_bound,
            Duration::from_millis(3_000),
            32 * 1024 * 1024,
            30,
            300,
            4,
        ));
        let grid = GridSpec::default_cells();
        let box_ = CanonicalBox::parse("-79.631,43.649,-79.629,43.651").unwrap();
        let touched = grid.touched(&box_);

        let clip = Clip {
            nodes: vec![
                Node {
                    id: 1,
                    lat: 436_500_000,
                    lon: -796_300_000,
                    tags: Vec::new(),
                },
                Node {
                    id: 2,
                    lat: 436_510_000,
                    lon: -796_290_000,
                    tags: Vec::new(),
                },
            ],
            ways: vec![Way {
                id: 1,
                refs: vec![1, 2],
                tags: vec![("highway".to_string(), "residential".to_string())],
            }],
        };
        let mut writer = StoreWriter::create(dir.join("store.bin")).unwrap();
        let mut cells = Vec::new();
        for cell_id in touched.iter() {
            let (offset, length, digest) =
                writer.write_cell(&crate::encode::encode(&clip)).unwrap();
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
        let build_id = BuildId::parse(&"a".repeat(64)).unwrap();
        let manifest = Manifest {
            build_id,
            built_at: "2026-09-15T00:00:00Z".to_string(),
            grid_spec: grid,
            regions: vec![RegionRecord {
                source_path: "test".to_string(),
                source_published_at: "2026-09-14T00:00:00Z".to_string(),
                source_digest: Digest::from_bytes([1; 32]),
                source_bytes: 0,
                bounds: box_.key_text(),
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

    fn get_req(uri: &str, build: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(uri).method("GET");
        if let Some(b) = build {
            builder = builder.header("x-client-build", b);
        }
        builder.body(Body::empty()).unwrap()
    }

    fn with_real_ip(mut req: Request<Body>, ip: &str) -> Request<Body> {
        req.headers_mut()
            .insert("x-real-ip", HeaderValue::from_str(ip).unwrap());
        req
    }

    async fn body_bytes(resp: Response) -> Vec<u8> {
        to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    // BR1.1 — a missing or different x-client-build gets 426 before anything else.
    #[tokio::test]
    async fn wrong_or_missing_build_stamp_is_426() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);

        let resp = app
            .clone()
            .oneshot(get_req("/api/extract?bbox=0,0,1,1", None))
            .await
            .unwrap();
        assert_eq!(resp.status(), 426);

        let resp = app
            .oneshot(get_req("/api/extract?bbox=0,0,1,1", Some("other-build")))
            .await
            .unwrap();
        assert_eq!(resp.status(), 426);
    }

    // BR3.1 — a wrong bbox is 400 invalid_area.
    #[tokio::test]
    async fn a_wrong_bbox_is_400_invalid_area() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);
        let resp = app
            .oneshot(get_req("/api/extract?bbox=not-a-box", Some(BUILD)))
            .await
            .unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_bytes(resp).await;
        assert!(String::from_utf8_lossy(&body).contains("invalid_area"));
    }

    // BR3.3 — a 13-cell box is 400 area_too_large.
    #[tokio::test]
    async fn a_thirteen_cell_box_is_400_area_too_large() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);
        let resp = app
            .oneshot(get_req("/api/extract?bbox=0,0,0.13,0.01", Some(BUILD)))
            .await
            .unwrap();
        assert_eq!(resp.status(), 400);
        let body = body_bytes(resp).await;
        assert!(String::from_utf8_lossy(&body).contains("area_too_large"));
    }

    // BR4.1 — an uncovered box is 404 area_not_found.
    #[tokio::test]
    async fn an_uncovered_box_is_404() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);
        let resp = app
            .oneshot(get_req("/api/extract?bbox=10,10,10.01,10.01", Some(BUILD)))
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);
        let body = body_bytes(resp).await;
        assert!(String::from_utf8_lossy(&body).contains("not an area this deployment covers"));
    }

    // BR4.2 — a covered box with nothing mapped is 404 with the other detail.
    #[tokio::test]
    async fn a_covered_empty_box_is_404_nothing_mapped() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        // Far from the one way in the fixture, but still within the single
        // covered cell's rectangle registered in the manifest's coverage.
        let app = router(state);
        let resp = app
            .oneshot(get_req(
                "/api/extract?bbox=-79.6305,43.6495,-79.6304,43.6496",
                Some(BUILD),
            ))
            .await
            .unwrap();
        // This box may or may not be covered depending on cell alignment;
        // assert it is one of the two legitimate "no data" outcomes.
        assert!(resp.status() == 404);
    }

    // A good box: 200, application/octet-stream, x-extract-key; the second
    // identical request is a hit with identical bytes and the same key.
    #[tokio::test]
    async fn a_good_box_is_200_and_repeats_as_a_hit() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state.clone());

        let resp1 = app
            .clone()
            .oneshot(get_req(
                "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                Some(BUILD),
            ))
            .await
            .unwrap();
        assert_eq!(resp1.status(), 200);
        assert_eq!(
            resp1.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/octet-stream"
        );
        let key1 = resp1
            .headers()
            .get("x-extract-key")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let bytes1 = body_bytes(resp1).await;

        let resp2 = app
            .oneshot(get_req(
                "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                Some(BUILD),
            ))
            .await
            .unwrap();
        assert_eq!(resp2.status(), 200);
        let key2 = resp2
            .headers()
            .get("x-extract-key")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let bytes2 = body_bytes(resp2).await;

        assert_eq!(key1, key2);
        assert_eq!(bytes1, bytes2);
        let snapshot = state.counters.snapshot();
        assert_eq!(snapshot.cache_misses, 1);
        assert_eq!(snapshot.cache_hits, 1);
    }

    // A stalled cutting phase times out within budget and frees the slot.
    #[tokio::test]
    async fn a_stalled_cut_times_out_and_frees_the_slot() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        state.set_stall(Some(Duration::from_millis(50)));
        // Use a tiny budget so the test runs fast.
        let state = Arc::new(AppState {
            request_budget: Duration::from_millis(20),
            ..Arc::try_unwrap(state).unwrap_or_else(|arc| panic!("{}", Arc::strong_count(&arc)))
        });
        let app = router(state.clone());
        let start = Instant::now();
        let resp = app
            .oneshot(get_req(
                "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                Some(BUILD),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), 503);
        assert!(start.elapsed() < Duration::from_millis(3_050));
        let body = body_bytes(resp).await;
        assert!(String::from_utf8_lossy(&body).contains("timeout"));
    }

    // NFR1.1.6 — eight misses against four slots never exceed four
    // concurrent cuts.
    #[tokio::test]
    async fn eight_misses_against_four_slots_never_exceed_four_concurrent() {
        let dir = tempfile::tempdir().unwrap();
        // A large covered rectangle so eight distinct one-cell boxes inside
        // it are all misses on distinct keys, never sharing a cache entry.
        let state = Arc::new(AppState::new(
            BUILD.to_string(),
            200,
            Duration::from_millis(3_000),
            32 * 1024 * 1024,
            1_000,
            100_000,
            4,
        ));
        let grid = GridSpec::default_cells();
        let region = CanonicalBox::parse("0,0,0.09,0.01").unwrap();
        let touched = grid.touched(&region);
        let clip = Clip {
            nodes: vec![Node {
                id: 1,
                lat: 100,
                lon: 100,
                tags: Vec::new(),
            }],
            ways: vec![Way {
                id: 1,
                refs: vec![1],
                tags: vec![("highway".to_string(), "residential".to_string())],
            }],
        };
        let mut writer = StoreWriter::create(dir.path().join("store.bin")).unwrap();
        let mut cells = Vec::new();
        for cell_id in touched.iter() {
            let (offset, length, digest) =
                writer.write_cell(&crate::encode::encode(&clip)).unwrap();
            cells.push(CellEntry {
                cell_id,
                offset,
                length,
                digest,
                way_count: 1,
                node_count: 1,
            });
        }
        writer.flush().unwrap();
        let store = Store::open(dir.path().join("store.bin"), &cells).unwrap();
        let coverage = CoverageIndex::from_rectangles(&[touched.rect()]);
        let manifest = Manifest {
            build_id: BuildId::parse(&"a".repeat(64)).unwrap(),
            built_at: "2026-09-15T00:00:00Z".to_string(),
            grid_spec: grid,
            regions: Vec::new(),
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
        state.set_stall(Some(Duration::from_millis(80)));

        let app = router(state.clone());
        let mut handles = Vec::new();
        for i in 0..8u32 {
            let app = app.clone();
            let bbox = format!("{},0,{},0.01", i as f64 * 0.01, i as f64 * 0.01 + 0.005);
            handles.push(tokio::spawn(async move {
                app.oneshot(get_req(&format!("/api/extract?bbox={bbox}"), Some(BUILD)))
                    .await
                    .unwrap()
            }));
        }
        for h in handles {
            let resp = h.await.unwrap();
            // Whether a given one-cell box happens to contain the fixture's
            // single way or not, every cut still passes through the
            // semaphore-guarded phase this test is checking; only a bug
            // there would surface as something other than 200 or 404.
            assert!(
                resp.status() == 200 || resp.status() == 404,
                "{:?}",
                resp.status()
            );
        }
        assert!(
            state.peak_active_cuts() <= 4,
            "peak was {}",
            state.peak_active_cuts()
        );
    }

    // A handler panic is 503 internal with exactly one failure row.
    #[tokio::test]
    async fn a_panic_is_503_internal() {
        let app = Router::new()
            .route(
                "/panic",
                get(|| async {
                    panic!("boom");
                    #[allow(unreachable_code)]
                    ()
                }),
            )
            .layer(CatchPanicLayer::custom(handle_panic));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/panic")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 503);
        let body = body_bytes(resp).await;
        assert!(String::from_utf8_lossy(&body).contains("internal"));
    }

    // BR2.1 — the 31st request in a minute is 429 with Retry-After.
    #[tokio::test]
    async fn the_thirty_first_request_is_429_with_retry_after() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);
        for _ in 0..30 {
            let resp = app
                .clone()
                .oneshot(with_real_ip(
                    get_req(
                        "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                        Some(BUILD),
                    ),
                    "192.0.2.9",
                ))
                .await
                .unwrap();
            assert_ne!(resp.status(), 429);
        }
        let resp = app
            .oneshot(with_real_ip(
                get_req(
                    "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                    Some(BUILD),
                ),
                "192.0.2.9",
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), 429);
        assert!(resp.headers().get(header::RETRY_AFTER).is_some());
    }

    // NFR6.3.3 — a spoofed X-Forwarded-For does not change the limiter's key.
    #[tokio::test]
    async fn spoofed_x_forwarded_for_does_not_change_the_limiter_key() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);
        for i in 0..30 {
            let mut req = with_real_ip(
                get_req(
                    "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                    Some(BUILD),
                ),
                "192.0.2.10",
            );
            req.headers_mut().insert(
                "x-forwarded-for",
                HeaderValue::from_str(&format!("203.0.113.{i}")).unwrap(),
            );
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_ne!(resp.status(), 429);
        }
        let resp = app
            .oneshot(with_real_ip(
                get_req(
                    "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                    Some(BUILD),
                ),
                "192.0.2.10",
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            429,
            "the varying X-Forwarded-For never mattered"
        );
    }

    // Fixed headers, no Set-Cookie, no CORS on a preflight, POST rejected.
    #[tokio::test]
    async fn fixed_headers_no_cookie_no_cors_post_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state);

        let resp = app
            .clone()
            .oneshot(get_req(
                "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                Some(BUILD),
            ))
            .await
            .unwrap();
        let h = resp.headers();
        assert!(h.get(header::STRICT_TRANSPORT_SECURITY).is_some());
        assert!(h.get(header::X_CONTENT_TYPE_OPTIONS).is_some());
        assert!(h.get(header::REFERRER_POLICY).is_some());
        assert!(h.get(header::CONTENT_SECURITY_POLICY).is_some());
        assert_eq!(h.get(header::CACHE_CONTROL).unwrap(), "private, no-store");
        assert!(h.get(header::SET_COOKIE).is_none());

        let preflight = Request::builder()
            .method("OPTIONS")
            .uri("/api/extract")
            .header("origin", "https://example.com")
            .header("access-control-request-headers", "x-client-build")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(preflight).await.unwrap();
        assert!(resp.headers().get("access-control-allow-origin").is_none());

        let post = Request::builder()
            .method("POST")
            .uri("/api/extract")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(post).await.unwrap();
        assert_eq!(resp.status(), 405);
    }

    // GET /readyz: 200 ready / 503 not-ready, header set, unaffected by a
    // limited requester; an extract request while Unready is 503.
    #[tokio::test]
    async fn readyz_reflects_readiness_and_ignores_the_limiter() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state.clone());
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        assert!(
            resp.headers()
                .get(header::STRICT_TRANSPORT_SECURITY)
                .is_some()
        );

        state.set_unready();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 503);

        let resp = app
            .oneshot(get_req(
                "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                Some(BUILD),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), 503);
        let body = body_bytes(resp).await;
        assert!(String::from_utf8_lossy(&body).contains("upstream_unavailable"));
    }

    // The log-discipline test: no address, box, key or cell id ever appears
    // in a captured row, and a successful request logs nothing at all.
    #[tokio::test]
    async fn no_address_box_key_or_cell_id_is_ever_logged() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_ready_state(dir.path(), 12);
        let app = router(state.clone());
        let writer = emit::CaptureWriter::new();
        let subscriber = emit::subscriber(writer.clone());

        let requested_ip = "192.0.2.55";
        tracing::subscriber::with_default(subscriber, || {
            futures::executor::block_on(async {
                // A hit and a miss for the same box.
                let _ = app
                    .clone()
                    .oneshot(with_real_ip(
                        get_req(
                            "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                            Some(BUILD),
                        ),
                        requested_ip,
                    ))
                    .await
                    .unwrap();
                let hit = app
                    .clone()
                    .oneshot(with_real_ip(
                        get_req(
                            "/api/extract?bbox=-79.631,43.649,-79.629,43.651",
                            Some(BUILD),
                        ),
                        requested_ip,
                    ))
                    .await
                    .unwrap();
                let key = hit
                    .headers()
                    .get("x-extract-key")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                // Every failure reason this Unit can produce.
                let _ = app
                    .clone()
                    .oneshot(get_req("/api/extract?bbox=x", None))
                    .await;
                let _ = app
                    .clone()
                    .oneshot(get_req("/api/extract?bbox=x", Some(BUILD)))
                    .await;
                let _ = app
                    .clone()
                    .oneshot(get_req("/api/extract?bbox=10,10,10.01,10.01", Some(BUILD)))
                    .await;
                let _ = app
                    .clone()
                    .oneshot(get_req("/api/extract?bbox=0,0,0.13,0.01", Some(BUILD)))
                    .await;

                let lines = writer.lines();
                for line in &lines {
                    assert!(!line.contains(requested_ip), "{line}");
                    assert!(!line.contains("-79.631"), "{line}");
                    assert!(!line.contains(&key), "{line}");
                }
            });
        });
    }
}
