//! `/api/work`: pull, push and delete the work a person stores on the server. The server checks what it must know
//! (`work::Put::from_wire`) and passes the rest through sealed, as the browser sent it.
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRequest, Query, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router, middleware};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use uuid::Uuid;

use super::{authed, no_content, store_error};
use crate::app::{Account, ProxyIpKeyExtractor, json_error, json_response, require_origin};
use crate::work::{self, Ask, Limit, Put, Record, Verdict, WorkError};

/// 100 records of 64 KB, and the envelope.
const MAX_REQUEST_BYTES: usize = 7 * 1024 * 1024;

fn too_large(limit: Limit) -> Response {
    json_response(StatusCode::PAYLOAD_TOO_LARGE, serde_json::json!({ "error": "too-large", "limit": limit.as_str() }))
}

fn work_error(e: WorkError) -> Response {
    match e {
        WorkError::StorageOff => json_error(StatusCode::CONFLICT, "storage-off"),
        WorkError::Resync => json_error(StatusCode::GONE, "resync"),
        WorkError::Reset => json_error(StatusCode::GONE, "reset"),
        WorkError::TooLarge(limit) => too_large(limit),
        WorkError::Duplicate => json_error(StatusCode::BAD_REQUEST, "bad-request"),
        WorkError::Gone => json_error(StatusCode::UNAUTHORIZED, "signed-out"),
        WorkError::Unavailable => json_error(StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        WorkError::Corrupt => json_error(StatusCode::INTERNAL_SERVER_ERROR, "corrupt"),
    }
}

#[derive(Serialize)]
struct RecordOut {
    kind: &'static str,
    key: String,
    rev: i64,
    edited_at: String,
    deleted: bool,
    body: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'static str>,
}

fn record_out(r: Record) -> RecordOut {
    let body = r.body.and_then(|b| String::from_utf8(b).ok()).and_then(|text| RawValue::from_string(text).ok());
    let error = (body.is_none() && !r.deleted).then_some("corrupt");
    RecordOut { kind: r.id.kind.as_str(), key: r.id.key, rev: r.rev, edited_at: work::stamp(r.edited_at), deleted: r.deleted, body, error }
}

#[derive(Deserialize)]
struct PullQuery {
    since: Option<i64>,
    /// The revision the whole pull began from (its first page's `since`); absent means `since`.
    start: Option<i64>,
    /// The copy of the account's work the device last read.
    epoch: Option<String>,
}

#[derive(Serialize)]
struct PulledOut {
    /// The account the session belongs to: a tab that believes in another account is out of date.
    user: Uuid,
    epoch: Uuid,
    rev: i64,
    more: bool,
    records: Vec<RecordOut>,
}

async fn pull(State(a): State<Arc<Account>>, jar: CookieJar, Query(q): Query<PullQuery>) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let since = q.since.unwrap_or(0);
    let start = q.start.unwrap_or(since);
    let epoch = match q.epoch.as_deref().map(Uuid::parse_str) {
        None => None,
        Some(Ok(epoch)) => Some(epoch),
        Some(Err(_)) => return json_error(StatusCode::BAD_REQUEST, "bad-request"),
    };
    if since < 0 || start < 0 || start > since {
        return json_error(StatusCode::BAD_REQUEST, "bad-request");
    }
    match a.work.pull(user, Ask { since, start, epoch }, a.clock.now()).await {
        Ok(p) => {
            (jar, Json(PulledOut { user, epoch: p.epoch, rev: p.rev, more: p.more, records: p.records.into_iter().map(record_out).collect() })).into_response()
        }
        Err(e) => work_error(e),
    }
}

#[derive(Deserialize)]
struct PushIn {
    records: Vec<PutIn>,
}

#[derive(Deserialize)]
struct PutIn {
    kind: String,
    key: String,
    base_rev: i64,
    edited_at: String,
    deleted: bool,
    #[serde(default)]
    body: Option<Box<RawValue>>,
}

#[derive(Serialize)]
struct ResultOut {
    kind: &'static str,
    key: String,
    status: &'static str,
    rev: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    record: Option<RecordOut>,
}

#[derive(Serialize)]
struct PushedOut {
    rev: i64,
    results: Vec<ResultOut>,
}

/// The body is the last thing taken from the request: the session and the storage choice are checked first, so a
/// request that will be refused for either does not make the server buffer up to 7 MB before saying so.
async fn push(State(a): State<Arc<Account>>, jar: CookieJar, request: Request) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match a.repo.user(user).await {
        Ok(Some(u)) if u.store_on_server == Some(true) => {}
        Ok(Some(_)) => return json_error(StatusCode::CONFLICT, "storage-off"),
        Ok(None) => return json_error(StatusCode::UNAUTHORIZED, "signed-out"),
        Err(e) => return store_error(e),
    }
    // As an extractor `Bytes` honours the `DefaultBodyLimit` layer below; a body over it is a 413.
    let Ok(bytes) = Bytes::from_request(request, &()).await else { return too_large(Limit::Batch) };
    let Ok(parsed) = serde_json::from_slice::<PushIn>(&bytes) else { return json_error(StatusCode::BAD_REQUEST, "bad-request") };
    if parsed.records.len() > work::MAX_BATCH {
        return too_large(Limit::Batch);
    }
    let mut batch = Vec::with_capacity(parsed.records.len());
    for r in &parsed.records {
        match Put::from_wire(&r.kind, &r.key, r.base_rev, &r.edited_at, r.deleted, r.body.as_deref().map(RawValue::get)) {
            Ok(put) => batch.push(put),
            Err(_) => return json_error(StatusCode::BAD_REQUEST, "bad-request"),
        }
    }
    match a.work.push(user, batch, a.clock.now()).await {
        Ok(p) => {
            let results = p
                .items
                .into_iter()
                .map(|i| ResultOut {
                    kind: i.id.kind.as_str(),
                    key: i.id.key,
                    status: if i.verdict == Verdict::Accepted { "accepted" } else { "superseded" },
                    rev: i.rev,
                    record: i.record.map(record_out),
                })
                .collect();
            (jar, Json(PushedOut { rev: p.rev, results })).into_response()
        }
        Err(e) => work_error(e),
    }
}

async fn clear(State(a): State<Arc<Account>>, jar: CookieJar) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match a.work.clear(user).await {
        Ok(()) => no_content(jar),
        Err(e) => work_error(e),
    }
}

/// The three routes, behind their own rate limit (counted per address as `/auth`'s is) and the Origin check.
pub fn routes(account: Arc<Account>) -> Router<Arc<Account>> {
    let limit = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(account.work_rate.period_secs)
            .burst_size(account.work_rate.burst)
            .key_extractor(ProxyIpKeyExtractor)
            .finish()
            .expect("a rate limit with positive numbers"),
    );
    // As for `/auth` in `app::router`: the limiter keeps one entry per address; this forgets the quiet ones.
    if let Ok(runtime) = tokio::runtime::Handle::try_current() {
        let limiter = Arc::downgrade(limit.limiter());
        runtime.spawn(async move {
            let mut every = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                every.tick().await;
                let Some(limiter) = limiter.upgrade() else { return };
                limiter.retain_recent();
            }
        });
    }
    Router::new()
        .route("/api/work", get(pull).put(push).delete(clear))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .route_layer(middleware::from_fn_with_state(account, require_origin))
        .layer(GovernorLayer::new(limit))
}
