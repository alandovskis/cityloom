//! `handlers` — the three axum route handlers (`logical-components.md`
//! LC-1): orchestrates rate limiting, validation, the authorization check,
//! and the single query per operation (`performance-design.md` PD-1).
//! Contains no SQL itself — every database interaction goes through
//! `repository`. The only module in this crate depending on `axum`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tower_http::catch_panic::CatchPanicLayer;

use cityloom_api_types::contract2::{ApiError, FailureReason, UploadReceipt, UploadRequest};
use cityloom_design_payload::DesignPayload;

use crate::failure::StorageFailure;
use crate::observability::{self, Counters, Operation};
use crate::rate_limit::{LimitOutcome, RateLimiter};
use crate::repository::{self, InsertOutcome};

/// Anonymous uploads expire 30 days after creation (BR2.1, AC11.3.1).
const ANONYMOUS_RETENTION_DAYS: i64 = 30;

struct Inner {
    pool: sqlx::PgPool,
    limiter: Arc<RateLimiter>,
    counters: Arc<Counters>,
    max_payload_bytes: usize,
}

/// The service's shared state: one instance behind an `Arc`, cloned into
/// every handler via axum's `State` extractor.
#[derive(Clone)]
pub struct AppState(Arc<Inner>);

impl AppState {
    pub fn new(pool: sqlx::PgPool, limiter: RateLimiter, max_payload_bytes: usize) -> AppState {
        AppState(Arc::new(Inner {
            pool,
            limiter: Arc::new(limiter),
            counters: Arc::new(Counters::new()),
            max_payload_bytes,
        }))
    }

    pub fn counters(&self) -> &Counters {
        &self.0.counters
    }

    /// R-02 — the same `Counters` instance handlers write to, shared with
    /// the expiry-sweep task so it can sample `stored_rows_count`
    /// (`observability-design.md` OD-2) on its own cycle without a second,
    /// disconnected `Counters` instance existing anywhere.
    pub fn counters_arc(&self) -> Arc<Counters> {
        Arc::clone(&self.0.counters)
    }

    /// R-01 — the same `RateLimiter` instance handlers check against,
    /// shared with `lib.rs::build()`'s spawned periodic sweep task so the
    /// map it sweeps is the one actually growing under real traffic.
    pub fn limiter_arc(&self) -> Arc<RateLimiter> {
        Arc::clone(&self.0.limiter)
    }

    /// Test-only introspection: the number of identifiers the shared
    /// `RateLimiter` currently tracks, used by `lib.rs`'s end-to-end
    /// `build()`-wiring test to observe the periodic sweep actually
    /// shrinking the map.
    #[cfg(test)]
    pub(crate) fn limiter_tracked_identifiers(&self) -> usize {
        self.0.limiter.tracked_identifiers()
    }

    /// Test-only: drive one rate-limiter check against `identifier`
    /// directly (rather than through a full HTTP request), used by
    /// `lib.rs`'s `build()`-wiring test to generate sustained
    /// varying-identifier traffic against the exact `RateLimiter`
    /// instance `build()` also hands to its spawned sweep task.
    #[cfg(test)]
    pub(crate) fn limiter_check_for_test(&self, identifier: &str) -> LimitOutcome {
        self.0.limiter.check(identifier, Instant::now())
    }
}

/// The router this crate's composition-root function mounts
/// (`logical-components.md` LC-3): `/api/designs*` plus this crate's own
/// contribution to `/readyz` (`reliability-design.md` RD-2). A panic in a
/// handler is caught and answered as `internal` rather than tearing down
/// the shared process (defence in depth, matching `osm-extract-proxy`'s
/// own L1 layer).
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/designs", post(upload))
        .route(
            "/api/designs/{anonymous_design_id}",
            get(fetch).delete(remove),
        )
        .route("/readyz", get(readyz))
        .layer(CatchPanicLayer::new())
        // L1 — a generous framework-level ceiling above this Unit's own
        // configurable ID-20 cap (checked precisely, per-request, inside
        // the `upload` handler itself); this only guards against a body
        // large enough to be a resource concern before that check runs.
        .layer(axum::extract::DefaultBodyLimit::max(16 * 1024 * 1024))
        .with_state(state)
}

fn api_error_response(status: u16, api_error: ApiError) -> Response {
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(api_error)).into_response()
}

fn storage_failure_response(operation: Operation, failure: &StorageFailure) -> Response {
    observability::record_failure(operation, failure.reason, failure.status());
    api_error_response(failure.status(), failure.to_api_error())
}

fn rate_limited_response(operation: Operation, retry_after_secs: u32) -> Response {
    observability::record_failure(operation, FailureReason::RateLimited, 429);
    let mut response = api_error_response(
        429,
        ApiError::new(FailureReason::RateLimited, "too many requests"),
    );
    if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

fn format_rfc3339(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .expect("an OffsetDateTime always formats as RFC 3339")
}

/// `security-design.md` SD-3 — the raw payload byte-length check, run
/// before the payload is deserialized into a `DesignPayload` at all. A
/// pure function: it takes no database handle, so calling it can never
/// reach the database (BR6.1's "before any write" guarantee is structural,
/// not just an ordering convention).
fn check_payload_size(payload_bytes: &str, max_payload_bytes: usize) -> Result<(), StorageFailure> {
    if payload_bytes.len() > max_payload_bytes {
        Err(StorageFailure::new(
            FailureReason::PayloadTooLarge,
            "the design payload exceeds the size limit",
        ))
    } else {
        Ok(())
    }
}

/// `security-design.md` SD-3 — the `payloadVersion` check, reusing
/// `cityloom-design-payload`'s own fail-closed `DesignPayload::from_json`
/// rather than re-implementing it (`tech-stack-decisions.md`). Also a pure
/// function with no database handle.
fn validate_payload_version(payload_bytes: &str) -> Result<(), StorageFailure> {
    DesignPayload::from_json(payload_bytes)
        .map(|_| ())
        .map_err(|_| {
            StorageFailure::new(
                FailureReason::UnsupportedPayloadVersion,
                "the design payload version is not supported",
            )
        })
}

async fn upload(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    let state = state.0;

    // L1/SD-3 — the whole request body's raw length is checked before any
    // parsing, using this Unit's own payload cap as the request ceiling
    // (the JSON envelope's own overhead beyond `payload` is negligible).
    if body.len() > state.max_payload_bytes {
        state
            .counters
            .record_upload_failure(FailureReason::PayloadTooLarge);
        return storage_failure_response(
            Operation::Upload,
            &StorageFailure::new(
                FailureReason::PayloadTooLarge,
                "the design payload exceeds the size limit",
            ),
        );
    }

    // Parse only the envelope (`anonymousDesignId` + the payload as an
    // undeserialized `RawValue`) — this is not the SD-3 "validation" step,
    // only enough structure to know the rate-limiting key and to hand the
    // payload bytes to the checks below unparsed.
    let request: UploadRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => {
            state
                .counters
                .record_upload_failure(FailureReason::UnsupportedPayloadVersion);
            return storage_failure_response(
                Operation::Upload,
                &StorageFailure::new(
                    FailureReason::UnsupportedPayloadVersion,
                    "the request body is not a well-formed upload request",
                ),
            );
        }
    };

    // L2 — rate limiting runs before validation (`security-design.md`
    // SD-1).
    match state
        .limiter
        .check(&request.anonymous_design_id, Instant::now())
    {
        LimitOutcome::Limited { retry_after_secs } => {
            state
                .counters
                .record_upload_failure(FailureReason::RateLimited);
            return rate_limited_response(Operation::Upload, retry_after_secs);
        }
        LimitOutcome::Allowed => {}
    }

    let payload_text = request.payload.get();

    // L3 — payloadVersion + size cap, both before any write (BR6.1).
    if let Err(failure) = check_payload_size(payload_text, state.max_payload_bytes) {
        state.counters.record_upload_failure(failure.reason);
        return storage_failure_response(Operation::Upload, &failure);
    }
    if let Err(failure) = validate_payload_version(payload_text) {
        state.counters.record_upload_failure(failure.reason);
        return storage_failure_response(Operation::Upload, &failure);
    }

    let created_at = OffsetDateTime::now_utc();
    let expires_at = created_at + time::Duration::days(ANONYMOUS_RETENTION_DAYS);

    let outcome = repository::insert_anonymous(
        &state.pool,
        &request.anonymous_design_id,
        payload_text,
        created_at,
        expires_at,
    )
    .await;

    match outcome {
        Ok(InsertOutcome::Inserted) => {
            state.counters.record_upload_success();
            let receipt = UploadReceipt {
                anonymous_design_id: request.anonymous_design_id,
                uploaded_at: format_rfc3339(created_at),
                expires_at: format_rfc3339(expires_at),
            };
            (StatusCode::CREATED, Json(receipt)).into_response()
        }
        Ok(InsertOutcome::Conflict) => {
            // BR1.1/AC8.3.1 — a conflicting anonymousDesignId is a named,
            // expected outcome (security-design.md SD-6), not one of this
            // Unit's six closed StorageFailure reasons (entities.md has no
            // "already exists" value, and the test suite pins that set to
            // exactly six) — answered as status-only, the same "no body"
            // shape Contract 2 already uses for DELETE's 204.
            state.counters.record_upload_conflict();
            StatusCode::CONFLICT.into_response()
        }
        Err(failure) => {
            state.counters.record_upload_failure(failure.reason);
            storage_failure_response(Operation::Upload, &failure)
        }
    }
}

async fn fetch(State(state): State<AppState>, Path(anonymous_design_id): Path<String>) -> Response {
    let state = state.0;

    match state.limiter.check(&anonymous_design_id, Instant::now()) {
        LimitOutcome::Limited { retry_after_secs } => {
            state.counters.record_fetch_failure(429);
            return rate_limited_response(Operation::Fetch, retry_after_secs);
        }
        LimitOutcome::Allowed => {}
    }

    // security-design.md SD-4 — the same query authorizes and reads: there
    // is no window between "authorized" and "payload read" for a race.
    match repository::select_by_anonymous_id(&state.pool, &anonymous_design_id).await {
        Ok(Some(design)) => {
            state.counters.record_fetch_success();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                design.payload,
            )
                .into_response()
        }
        Ok(None) => {
            let failure = StorageFailure::new(FailureReason::DesignNotFound, "no such design");
            state.counters.record_fetch_failure(failure.status());
            storage_failure_response(Operation::Fetch, &failure)
        }
        Err(failure) => {
            state.counters.record_fetch_failure(failure.status());
            storage_failure_response(Operation::Fetch, &failure)
        }
    }
}

async fn remove(
    State(state): State<AppState>,
    Path(anonymous_design_id): Path<String>,
) -> Response {
    let state = state.0;

    match state.limiter.check(&anonymous_design_id, Instant::now()) {
        LimitOutcome::Limited { retry_after_secs } => {
            state.counters.record_delete_rate_limited();
            return rate_limited_response(Operation::Remove, retry_after_secs);
        }
        LimitOutcome::Allowed => {}
    }

    // BR5.1 — no account or authentication is checked beyond the
    // identifier itself; deletion is idempotent (204 whether or not a row
    // existed).
    match repository::delete_by_anonymous_id(&state.pool, &anonymous_design_id).await {
        Ok(()) => {
            state.counters.record_delete_success();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(failure) => {
            // R-03 — a genuine repository failure is never folded into
            // `deletes_success`.
            state.counters.record_delete_failure();
            storage_failure_response(Operation::Remove, &failure)
        }
    }
}

/// `reliability-design.md` RD-2 — this crate's own contribution to
/// `/readyz`: a lightweight `SELECT 1` against the pool within a short
/// timeout. A failed or timed-out ping reports not-ready while the process
/// keeps running.
async fn readyz(State(state): State<AppState>) -> Response {
    let ping = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(&state.0.pool),
    )
    .await;

    match ping {
        Ok(Ok(_)) => (StatusCode::OK, Json(serde_json::json!({"status": "ready"}))).into_response(),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not_ready"})),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::RateLimiter;
    use crate::test_support::{test_pool, unique_id};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn app_state() -> AppState {
        let pool = test_pool().await;
        AppState::new(pool, RateLimiter::new(10, 100), 5 * 1024 * 1024)
    }

    fn upload_body(anonymous_design_id: &str) -> String {
        format!(
            r#"{{"anonymousDesignId":"{anonymous_design_id}","payload":{{"payloadVersion":1,"design":{{"designId":"d1","name":"n","streets":[],"createdAt":"2026-09-19T00:00:00Z","updatedAt":"2026-09-19T00:00:00Z"}},"corrections":[]}}}}"#
        )
    }

    async fn body_bytes(response: Response) -> Vec<u8> {
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    }

    // AC8.3.1 — a valid upload returns 201 with an UploadReceipt.
    #[test]
    fn a_valid_upload_returns_201_with_an_upload_receipt() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let app = router(state);
            let id = unique_id("upload-ok");
            let request = axum::http::Request::builder()
                .method("POST")
                .uri("/api/designs")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(upload_body(&id)))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
            let bytes = body_bytes(response).await;
            let receipt: UploadReceipt = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(receipt.anonymous_design_id, id);
        });
    }

    // BR1.1 — a conflicting identifier returns 409.
    #[test]
    fn a_conflicting_identifier_returns_409() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let id = unique_id("upload-conflict");
            let make_request = || {
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/designs")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(upload_body(&id)))
                    .unwrap()
            };
            let first = router(state.clone()).oneshot(make_request()).await.unwrap();
            assert_eq!(first.status(), StatusCode::CREATED);
            let second = router(state).oneshot(make_request()).await.unwrap();
            assert_eq!(second.status(), StatusCode::CONFLICT);
        });
    }

    // BR6.1/AC8.3.3 — an oversized payload returns 413 and leaves no row.
    #[test]
    fn an_oversized_payload_returns_413_and_leaves_no_row() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let pool = state.0.pool.clone();
            let small_state = AppState::new(pool.clone(), RateLimiter::new(10, 100), 16);
            let id = unique_id("upload-oversized");
            let request = axum::http::Request::builder()
                .method("POST")
                .uri("/api/designs")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(upload_body(&id)))
                .unwrap();
            let response = router(small_state).oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
            assert!(
                repository::select_by_anonymous_id(&pool, &id)
                    .await
                    .unwrap()
                    .is_none()
            );
        });
    }

    // BR6.1/AC8.3.3 — an invalid payloadVersion returns 400 and leaves no
    // row.
    #[test]
    fn an_invalid_payload_version_returns_400_and_leaves_no_row() {
        crate::test_support::runtime().block_on(async {
        let state = app_state().await;
        let pool = state.0.pool.clone();
        let id = unique_id("upload-bad-version");
        let body = format!(
            r#"{{"anonymousDesignId":"{id}","payload":{{"payloadVersion":99,"design":{{}},"corrections":[]}}}}"#
        );
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/api/designs")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap();
        let response = router(state).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            repository::select_by_anonymous_id(&pool, &id)
                .await
                .unwrap()
                .is_none()
        );
            });
    }

    // BR4.1 — rate-limited upload returns 429.
    #[test]
    fn a_rate_limited_upload_returns_429() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let state = AppState::new(pool, RateLimiter::new(1, 100), 5 * 1024 * 1024);
            let id = unique_id("upload-rate-limited");
            let app = router(state);
            let make_request = || {
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/designs")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(upload_body(&id)))
                    .unwrap()
            };
            let first = app.clone().oneshot(make_request()).await.unwrap();
            assert_eq!(first.status(), StatusCode::CREATED);
            let second = app.oneshot(make_request()).await.unwrap();
            assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
            assert!(second.headers().contains_key(header::RETRY_AFTER));
        });
    }

    // AC10.1.2/BR3.1 — a valid, unexpired, matching id returns 200 with the
    // unchanged payload.
    #[test]
    fn a_valid_unexpired_fetch_returns_200_with_the_unchanged_payload() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let id = unique_id("fetch-ok");
            let app = router(state);
            let upload_request = axum::http::Request::builder()
                .method("POST")
                .uri("/api/designs")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(upload_body(&id)))
                .unwrap();
            let uploaded = app.clone().oneshot(upload_request).await.unwrap();
            assert_eq!(uploaded.status(), StatusCode::CREATED);

            let fetch_request = axum::http::Request::builder()
                .method("GET")
                .uri(format!("/api/designs/{id}"))
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.oneshot(fetch_request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = body_bytes(response).await;
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("\"payloadVersion\":1"));
        });
    }

    // BR3.1/AC10.1.2/AC10.1.3 — a missing id returns 404, and the body
    // contains no payload field.
    #[test]
    fn a_missing_id_returns_404_with_no_payload_field() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let app = router(state);
            let id = unique_id("fetch-missing");
            let request = axum::http::Request::builder()
                .method("GET")
                .uri(format!("/api/designs/{id}"))
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let bytes = body_bytes(response).await;
            let text = String::from_utf8(bytes).unwrap();
            assert!(!text.contains("payloadVersion"));
            assert!(!text.contains("\"design\""));
            assert!(!text.contains("\"streets\""));
        });
    }

    // BR4.1 — rate-limited fetch returns 429.
    #[test]
    fn a_rate_limited_fetch_returns_429() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let state = AppState::new(pool, RateLimiter::new(1, 100), 5 * 1024 * 1024);
            let id = unique_id("fetch-rate-limited");
            let app = router(state);
            let make_request = || {
                axum::http::Request::builder()
                    .method("GET")
                    .uri(format!("/api/designs/{id}"))
                    .body(axum::body::Body::empty())
                    .unwrap()
            };
            let first = app.clone().oneshot(make_request()).await.unwrap();
            assert_eq!(first.status(), StatusCode::NOT_FOUND);
            let second = app.oneshot(make_request()).await.unwrap();
            assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        });
    }

    // BR5.1/AC8.3.4 — DELETE always returns 204, whether the row existed
    // or not.
    #[test]
    fn delete_always_returns_204_whether_the_row_existed_or_not() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let app = router(state.clone());
            let id = unique_id("delete-existing");
            let upload_request = axum::http::Request::builder()
                .method("POST")
                .uri("/api/designs")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(upload_body(&id)))
                .unwrap();
            let uploaded = app.clone().oneshot(upload_request).await.unwrap();
            assert_eq!(uploaded.status(), StatusCode::CREATED);

            let delete_existing = axum::http::Request::builder()
                .method("DELETE")
                .uri(format!("/api/designs/{id}"))
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.clone().oneshot(delete_existing).await.unwrap();
            assert_eq!(response.status(), StatusCode::NO_CONTENT);

            let never_existed = unique_id("delete-never-existed");
            let delete_absent = axum::http::Request::builder()
                .method("DELETE")
                .uri(format!("/api/designs/{never_existed}"))
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.oneshot(delete_absent).await.unwrap();
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        });
    }

    // BR4.1 — rate limiting still applies to remove.
    #[test]
    fn a_rate_limited_delete_still_returns_429() {
        crate::test_support::runtime().block_on(async {
            let pool = test_pool().await;
            let state = AppState::new(pool, RateLimiter::new(1, 100), 5 * 1024 * 1024);
            let id = unique_id("delete-rate-limited");
            let app = router(state);
            let make_request = || {
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/designs/{id}"))
                    .body(axum::body::Body::empty())
                    .unwrap()
            };
            let first = app.clone().oneshot(make_request()).await.unwrap();
            assert_eq!(first.status(), StatusCode::NO_CONTENT);
            let second = app.oneshot(make_request()).await.unwrap();
            assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        });
    }

    // NFR7.4.1/security-design.md SD-7 — a log-capture test across one
    // upload failure, one fetch-404, and one rate-limit-429 asserts no
    // identifier, payload, or requester-identifying field appears in any
    // emitted row.
    #[test]
    fn failure_logs_never_carry_an_identifier_payload_or_requester_field() {
        use std::sync::{Arc as StdArc, Mutex};

        #[derive(Clone, Default)]
        struct CapturingWriter(StdArc<Mutex<Vec<u8>>>);
        impl std::io::Write for CapturingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'w> tracing_subscriber::fmt::MakeWriter<'w> for CapturingWriter {
            type Writer = CapturingWriter;
            fn make_writer(&'w self) -> Self::Writer {
                self.clone()
            }
        }

        let buffer = StdArc::new(Mutex::new(Vec::new()));
        let writer = CapturingWriter(buffer.clone());
        let subscriber = observability::subscriber(writer);

        let secret_id = unique_id("log-secret-identifier-value");
        let secret_payload_marker = "sentinel-payload-marker-xyz";

        // Force the shared pool to exist (created, as everywhere else,
        // under this crate's one shared multi-thread test runtime —
        // `test_support::runtime` — so its background maintenance task is
        // never orphaned) *before* entering the thread-local subscriber
        // scope below. `tracing::subscriber::with_default` only overrides
        // the *calling* thread's default, and a multi-thread runtime's
        // `block_on` can otherwise poll a future on a different worker
        // thread than the one that installed it; an ad-hoc *current-thread*
        // runtime avoids that migration for this one test's own request
        // handling, while touching only an already-initialized pool (no
        // new connections/background tasks are created here).
        crate::test_support::runtime().block_on(crate::test_support::test_pool());

        tracing::subscriber::with_default(subscriber, || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build this test's own current-thread runtime");
            rt.block_on(async {
                let pool = test_pool().await;
                // 2/minute: the upload below and the first fetch both need
                // to succeed past the limiter so the second fetch is the
                // one that trips it.
                let state = AppState::new(pool, RateLimiter::new(2, 100), 5 * 1024 * 1024);
                let app = router(state);

                // One upload failure (invalid payloadVersion).
                let bad_body = format!(
                    r#"{{"anonymousDesignId":"{secret_id}","payload":{{"payloadVersion":99,"marker":"{secret_payload_marker}"}}}}"#
                );
                let upload_request = axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/designs")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(bad_body))
                    .unwrap();
                let _ = app.clone().oneshot(upload_request).await.unwrap();

                // One fetch 404.
                let fetch_request = axum::http::Request::builder()
                    .method("GET")
                    .uri(format!("/api/designs/{secret_id}"))
                    .body(axum::body::Body::empty())
                    .unwrap();
                let first_fetch = app.clone().oneshot(fetch_request).await.unwrap();
                assert_eq!(first_fetch.status(), StatusCode::NOT_FOUND);

                // One rate-limit 429 (the fetch above already consumed the
                // one-per-minute budget for this identifier).
                let second_fetch_request = axum::http::Request::builder()
                    .method("GET")
                    .uri(format!("/api/designs/{secret_id}"))
                    .body(axum::body::Body::empty())
                    .unwrap();
                let second_fetch = app.oneshot(second_fetch_request).await.unwrap();
                assert_eq!(second_fetch.status(), StatusCode::TOO_MANY_REQUESTS);
            });
        });

        let log_text = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(!log_text.is_empty(), "expected at least one log row");
        assert!(!log_text.contains(&secret_id));
        assert!(!log_text.contains(secret_payload_marker));
    }

    // NFR7.4.4/NFR3.2.1 — a simulated database outage causes /readyz to
    // report not-ready while the process keeps running.
    #[test]
    fn readyz_reports_not_ready_when_the_database_is_unreachable() {
        crate::test_support::runtime().block_on(async {
            // A pool pointed at a port nothing listens on, so every query
            // times out/fails quickly rather than the real test database.
            let broken_pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://cityloom:cityloom@127.0.0.1:1/nonexistent")
                .unwrap();
            let state = AppState::new(broken_pool, RateLimiter::new(10, 100), 5 * 1024 * 1024);
            let app = router(state);
            let request = axum::http::Request::builder()
                .method("GET")
                .uri("/readyz")
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        });
    }

    #[test]
    fn readyz_reports_ready_when_the_database_is_reachable() {
        crate::test_support::runtime().block_on(async {
            let state = app_state().await;
            let app = router(state);
            let request = axum::http::Request::builder()
                .method("GET")
                .uri("/readyz")
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        });
    }

    // security-design.md SD-3 — the size and version checks are pure
    // functions with no database handle: calling them can never reach the
    // database.
    #[test]
    fn size_check_rejects_before_any_database_handle_could_exist() {
        let oversized = "x".repeat(20);
        assert!(check_payload_size(&oversized, 10).is_err());
        assert!(check_payload_size("short", 10).is_ok());
    }

    #[test]
    fn version_check_rejects_before_any_database_handle_could_exist() {
        assert!(validate_payload_version(r#"{"payloadVersion":99}"#).is_err());
        assert!(
            validate_payload_version(
                r#"{"payloadVersion":1,"design":{"designId":"d","name":"n","streets":[],"createdAt":"x","updatedAt":"x"},"corrections":[]}"#
            )
            .is_ok()
        );
    }
}
