//! `/api/share`: make a shared design (signed in), read one by its link (anyone), delete one (its creator). The server
//! checks what it must know of a body (`share::check_body`) and passes the rest through sealed, as the browser sent it.
//! A token is never logged (`app::loggable_path` hides it in the request log).
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRequest, Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Router, middleware};
use axum_extra::extract::CookieJar;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;

use super::{authed, json, no_content};
use crate::app::{Account, ProxyIpKeyExtractor, json_error, json_response, require_origin};
use crate::share::{self, Invalid, ShareError, Token};

fn share_error(e: ShareError) -> Response {
    match e {
        ShareError::NotFound => json_error(StatusCode::NOT_FOUND, "not-found"),
        ShareError::Expired => json_error(StatusCode::GONE, "expired"),
        ShareError::Deleted => json_error(StatusCode::GONE, "deleted"),
        ShareError::NotYours => json_error(StatusCode::FORBIDDEN, "not-yours"),
        ShareError::Full => json_response(StatusCode::CONFLICT, serde_json::json!({ "error": "full", "limit": share::MAX_LIVE_SHARES })),
        ShareError::Gone => json_error(StatusCode::UNAUTHORIZED, "signed-out"),
        ShareError::Unavailable => json_error(StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        ShareError::Corrupt => json_error(StatusCode::INTERNAL_SERVER_ERROR, "corrupt"),
    }
}

fn too_large() -> Response {
    json_response(StatusCode::PAYLOAD_TOO_LARGE, serde_json::json!({ "error": "too-large", "limit": "share" }))
}

/// The body is the last thing taken from the request: a signed-out request is refused before the server buffers it.
async fn create(State(a): State<Arc<Account>>, jar: CookieJar, request: Request) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    // As an extractor `Bytes` honours the `DefaultBodyLimit` layer below; a body over it is a 413.
    let Ok(body) = Bytes::from_request(request, &()).await else { return too_large() };
    match share::check_body(&body) {
        Ok(()) => {}
        Err(Invalid::TooLarge) => return too_large(),
        Err(Invalid::NotAShare) => return json_error(StatusCode::BAD_REQUEST, "bad-request"),
        Err(Invalid::Newer) => return json_error(StatusCode::UNPROCESSABLE_ENTITY, "newer"),
    }
    match a.shares.create(user, &Token::random(), &body, a.clock.now()).await {
        Ok(created) => {
            let out = serde_json::json!({ "token": created.token.text(), "expires_at": crate::work::stamp(created.expires_at) });
            (StatusCode::CREATED, json(jar, &out)).into_response()
        }
        Err(e) => share_error(e),
    }
}

/// No session needed: the link is the permission. A text that is no token is a 404, like a token nobody made.
async fn open(State(a): State<Arc<Account>>, Path(token): Path<String>) -> Response {
    let Some(token) = Token::parse(&token) else { return share_error(ShareError::NotFound) };
    match a.shares.open(&token, a.clock.now()).await {
        Ok(body) => {
            let mut res = (StatusCode::OK, body).into_response();
            res.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
            // The address holds the link's secret: a page that follows a link from here must not be told it.
            res.headers_mut().insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
            res
        }
        Err(e) => share_error(e),
    }
}

async fn delete(State(a): State<Arc<Account>>, jar: CookieJar, Path(token): Path<String>) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let Some(token) = Token::parse(&token) else { return share_error(ShareError::NotFound) };
    match a.shares.delete(user, &token, a.clock.now()).await {
        Ok(()) => no_content(jar),
        Err(e) => share_error(e),
    }
}

/// The routes, behind their own rate limit (counted per address as `/auth`'s is) and the Origin check on changes.
pub fn routes(account: Arc<Account>) -> Router<Arc<Account>> {
    let limit = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(account.share_rate.period_secs)
            .burst_size(account.share_rate.burst)
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
        .route("/api/share", post(create))
        .route("/api/share/{token}", get(open).delete(delete))
        // A little over the largest body, so a body just over it is read and refused by `check_body` as `share`.
        .layer(DefaultBodyLimit::max(share::MAX_BODY_BYTES + 1024))
        .route_layer(middleware::from_fn_with_state(account, require_origin))
        .layer(GovernorLayer::new(limit))
}
