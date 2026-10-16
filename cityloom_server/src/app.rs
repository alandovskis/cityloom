//! The whole router: the account routes (or their 503), then the static site.
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum_extra::extract::CookieJar;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};
use uuid::Uuid;

use crate::accounts::Repo;
use crate::clock::Clock;
use crate::config::{Config, RateLimit};
#[cfg(feature = "test-provider")]
use crate::fake_provider;
use crate::providers::{HttpClient, Providers};
use crate::{api, auth, session, static_files};

/// How often the rate limiter forgets the addresses that have been quiet long enough to have their full allowance back.
const LIMITER_CLEANUP: std::time::Duration = std::time::Duration::from_secs(60);

pub struct Account {
    pub repo: Arc<dyn Repo>,
    pub http: Arc<dyn HttpClient>,
    pub clock: Arc<dyn Clock>,
    pub providers: Providers,
    /// Built once from `AccountConfig.cookie_key` (a `Zeroizing`). `cookie::Key` is the cookie crate's type and does
    /// not wipe itself on drop: the one key copy this server cannot zeroize.
    pub cookie_key: cookie::Key,
    pub base_url: url::Url,
    pub rate: RateLimit,
    /// False until the migrations have run once; `require_ready` answers 503 on `/auth/*` and `/api/*` meanwhile.
    pub ready: Arc<AtomicBool>,
    #[cfg(feature = "test-provider")]
    pub fake: Option<Arc<fake_provider::FakeState>>,
}

impl Account {
    pub fn callback_url(&self, slug: &str) -> url::Url {
        self.base_url.join(&format!("auth/{slug}/callback")).expect("a slug joins")
    }

    /// The signed-in user of this request: `None` with no session cookie, an unknown or expired session, or a store error.
    pub async fn current_user(&self, jar: &CookieJar) -> Option<Uuid> {
        let token = jar.get(session::COOKIE)?.value().to_string();
        match session::read(&*self.repo, &token, self.clock.now()).await {
            Ok(session::Read::Valid(u)) | Ok(session::Read::Refreshed(u)) => Some(u),
            _ => None,
        }
    }
}

fn json_body(status: StatusCode, body: String) -> Response {
    let mut res = (status, body).into_response();
    res.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

/// `{"error": code}`, never cached.
pub fn json_error(status: StatusCode, code: &str) -> Response {
    json_body(status, serde_json::json!({ "error": code }).to_string())
}

async fn unavailable() -> Response {
    json_error(StatusCode::SERVICE_UNAVAILABLE, "account-unavailable")
}

async fn not_found() -> Response {
    json_error(StatusCode::NOT_FOUND, "not-found")
}

/// Which of the account paths (`/auth`, `/api`, each with anything below) this path is under. A wildcard route cannot
/// say this: it conflicts with `/auth/{provider}/start`, and misses `/auth/`.
fn under_account_path(path: &str) -> Option<&'static str> {
    ["/auth", "/api"].into_iter().find(|root| path.strip_prefix(root).is_some_and(|rest| rest.is_empty() || rest.starts_with('/')))
}

/// With the account side off, everything under `/auth` and `/api` is a 503: never a file of the site.
async fn account_off(req: Request, next: Next) -> Response {
    if under_account_path(req.uri().path()).is_some() { unavailable().await } else { next.run(req).await }
}

/// What no account route answered under `/auth` or `/api` is not a file of the site either: a 404.
async fn account_on(req: Request, next: Next) -> Response {
    if under_account_path(req.uri().path()).is_some() { not_found().await } else { next.run(req).await }
}

/// POST, PUT, PATCH and DELETE need this site's own `Origin`; reads pass.
pub async fn require_origin(State(a): State<Arc<Account>>, req: Request, next: Next) -> Response {
    if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(req).await;
    }
    let expected = a.base_url.origin().ascii_serialization();
    match req.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        Some(origin) if origin == expected => next.run(req).await,
        _ => json_error(StatusCode::FORBIDDEN, "origin"),
    }
}

/// Method, path without the query, status: never anything from a query, a header or a body.
async fn log_requests(req: Request, next: Next) -> Response {
    let (method, path) = (req.method().clone(), req.uri().path().to_string());
    let res = next.run(req).await;
    tracing::info!(%method, %path, status = res.status().as_u16(), "request");
    res
}

/// 503 on the account routes until the database has answered and been migrated.
pub async fn require_ready(State(a): State<Arc<Account>>, req: Request, next: Next) -> Response {
    if a.ready.load(Ordering::Acquire) { next.run(req).await } else { unavailable().await }
}

/// The address the rate limit counts. Behind Railway's proxy the peer is the proxy, and the proxy APPENDS the address
/// it saw to `X-Forwarded-For`, so the rightmost entry is the one nobody else wrote: whatever a client puts on the
/// left (tower_governor's stock smart extractor would trust the leftmost) cannot change the key. Without the header
/// (a direct connection, a test) the peer's address is used; an entry that is not an address counts as no header.
pub fn client_address(headers: &HeaderMap, peer: Option<IpAddr>) -> Option<IpAddr> {
    headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .next_back()
        .and_then(|entry| entry.parse().ok())
        .or(peer)
}

#[derive(Clone, Copy, Debug)]
pub struct ProxyIpKeyExtractor;

impl KeyExtractor for ProxyIpKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<IpAddr, GovernorError> {
        let peer = req.extensions().get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip());
        client_address(req.headers(), peer).ok_or(GovernorError::UnableToExtractKey)
    }
}

pub fn router(config: &Config, account: Option<Arc<Account>>) -> Router {
    let site = static_files::router(config.web_root.clone(), config.cache_control.clone());
    let Some(account) = account else {
        return Router::new().fallback_service(site.layer(middleware::from_fn(account_off))).layer(middleware::from_fn(log_requests));
    };
    let limit = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(account.rate.period_secs)
            .burst_size(account.rate.burst)
            .key_extractor(ProxyIpKeyExtractor)
            .finish()
            .expect("a rate limit with positive numbers"),
    );
    // The limiter keeps one entry per address it has seen; this drops those back at a full allowance. It stops when
    // the router (and with it the limiter) is gone. Without a runtime (a plain unit test) there is nothing to run it on.
    if let Ok(runtime) = tokio::runtime::Handle::try_current() {
        let limiter = Arc::downgrade(limit.limiter());
        runtime.spawn(async move {
            let mut every = tokio::time::interval(LIMITER_CLEANUP);
            loop {
                every.tick().await;
                let Some(limiter) = limiter.upgrade() else { return };
                limiter.retain_recent();
            }
        });
    }
    let auth = Router::new()
        .route("/auth/{provider}/start", get(auth::start))
        .route("/auth/{provider}/callback", get(auth::callback_get).post(auth::callback_post))
        .route("/auth/signout", post(auth::signout).route_layer(middleware::from_fn_with_state(account.clone(), require_origin)))
        .layer(GovernorLayer::new(limit))
        // The outermost layer of the account routes: no database, no sign-in, and no request counted against the limit.
        .layer(middleware::from_fn_with_state(account.clone(), require_ready))
        .with_state(account.clone());
    let app = Router::new().merge(auth).merge(api::routes(account.clone())).fallback_service(site.layer(middleware::from_fn(account_on)));
    #[cfg(feature = "test-provider")]
    let app = match &account.fake {
        Some(fake) => app.nest("/fake", fake_provider::router(fake.clone())),
        None => app,
    };
    app.layer(middleware::from_fn(log_requests))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(lines: &[&str]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for line in lines {
            h.append("x-forwarded-for", HeaderValue::from_str(line).unwrap());
        }
        h
    }

    fn ip(text: &str) -> Option<IpAddr> {
        Some(text.parse().unwrap())
    }

    #[test]
    fn the_rightmost_forwarded_entry_is_the_client() {
        assert_eq!(client_address(&headers(&["203.0.113.7"]), ip("10.0.0.1")), ip("203.0.113.7"));
        assert_eq!(client_address(&headers(&["198.51.100.1, 203.0.113.7"]), ip("10.0.0.1")), ip("203.0.113.7"));
        assert_eq!(client_address(&headers(&["2001:db8::1"]), None), ip("2001:db8::1"));
    }

    #[test]
    fn a_leftmost_entry_the_client_wrote_does_not_change_the_key() {
        let honest = client_address(&headers(&["203.0.113.7"]), ip("10.0.0.1"));
        for spoof in ["1.2.3.4, 203.0.113.7", "9.9.9.9,203.0.113.7", "1.1.1.1, 2.2.2.2, 203.0.113.7"] {
            assert_eq!(client_address(&headers(&[spoof]), ip("10.0.0.1")), honest, "{spoof}");
        }
        // Several header lines count as one list, in order.
        assert_eq!(client_address(&headers(&["1.2.3.4", "203.0.113.7"]), None), honest);
    }

    #[test]
    fn without_a_usable_header_the_peer_is_the_client_and_without_either_there_is_no_key() {
        assert_eq!(client_address(&headers(&[]), ip("10.0.0.1")), ip("10.0.0.1"));
        assert_eq!(client_address(&headers(&["not-an-address"]), ip("10.0.0.1")), ip("10.0.0.1"));
        assert_eq!(client_address(&headers(&[" , "]), ip("10.0.0.1")), ip("10.0.0.1"));
        assert_eq!(client_address(&headers(&[]), None), None);
    }

    #[test]
    fn the_extractor_reads_the_header_and_then_the_connection() {
        let key = |req: axum::http::Request<()>| ProxyIpKeyExtractor.extract(&req);
        let with_header = axum::http::Request::builder().header("x-forwarded-for", "7.7.7.7, 203.0.113.7").body(()).unwrap();
        assert_eq!(key(with_header).unwrap(), "203.0.113.7".parse::<IpAddr>().unwrap());
        let mut direct = axum::http::Request::builder().body(()).unwrap();
        direct.extensions_mut().insert(ConnectInfo("192.0.2.5:4000".parse::<SocketAddr>().unwrap()));
        assert_eq!(key(direct).unwrap(), "192.0.2.5".parse::<IpAddr>().unwrap());
        assert!(key(axum::http::Request::builder().body(()).unwrap()).is_err());
    }
}
