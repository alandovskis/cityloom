#![cfg(feature = "test-provider")]
// Every file in `tests/` is its own crate and says `mod support;`, so each compiles all of this and uses only some
// of it: `InProcess` is unused in `static_files.rs`, `Browser` in `providers.rs`. CI builds with
// `RUSTFLAGS=-D warnings`, which would turn those `dead_code` warnings into errors; this is the one place that
// allows them, and it covers everything added to the file later.
#![allow(dead_code)]
//! Plays the network and the browser for the integration tests.
use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, StatusCode, header};
use cityloom_server::accounts::{IdentityInput, IdentityView, Removal, Repo, RepoError, SessionRow, UserView};
use cityloom_server::app::{Account, router};
use cityloom_server::clock::FixedClock;
use cityloom_server::config::{Config, RateLimit};
use cityloom_server::crypto::{LookupKey, MasterKeys};
use cityloom_server::db::PgRepo;
use cityloom_server::fake_provider::{self, FakeLogin, FakeState, LOGIN_COOKIE, parse_form_post};
use cityloom_server::providers::{HttpClient, Providers};
use cityloom_server::work::{Ask, Pulled, Pushed, Put, WorkError, WorkRepo};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

pub const ISSUER: &str = "http://127.0.0.1:8137/fake";

pub fn t0() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
}

/// The fake provider, answering as if it were on the network.
pub struct InProcess(pub Router);

impl InProcess {
    pub fn new(clock: Arc<FixedClock>) -> InProcess {
        InProcess(Router::new().nest("/fake", fake_provider::router(FakeState::new(ISSUER.into(), clock))))
    }
}

#[async_trait::async_trait]
impl HttpClient for InProcess {
    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, String> {
        let (parts, body) = request.into_parts();
        let res = self.0.clone().oneshot(http::Request::from_parts(parts, Body::from(body))).await.map_err(|e| e.to_string())?;
        let (parts, body) = res.into_parts();
        Ok(http::Response::from_parts(parts, body.collect().await.map_err(|e| e.to_string())?.to_bytes().to_vec()))
    }
}

/// A client for a provider that is down.
pub struct Down;

#[async_trait::async_trait]
impl HttpClient for Down {
    async fn send(&self, _: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, String> {
        Err("connection refused".into())
    }
}

impl InProcess {
    pub fn with_state(state: Arc<FakeState>) -> InProcess {
        InProcess(Router::new().nest("/fake", fake_provider::router(state)))
    }
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Reply {
    pub fn location(&self) -> Option<String> {
        self.headers.get(header::LOCATION).and_then(|v| v.to_str().ok()).map(str::to_string)
    }
    pub fn set_cookies(&self) -> Vec<String> {
        self.headers.get_all(header::SET_COOKIE).iter().filter_map(|v| v.to_str().ok()).map(str::to_string).collect()
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap_or_else(|e| panic!("not JSON ({e}): {}", self.body))
    }
}

/// A browser: a cookie jar in front of the router.
pub struct Browser {
    app: Router,
    cookies: BTreeMap<String, String>,
    pub forwarded_for: String,
}

impl Browser {
    pub fn new(app: Router) -> Browser {
        Browser { app, cookies: BTreeMap::new(), forwarded_for: "203.0.113.7".into() }
    }
    pub fn set_cookie(&mut self, name: &str, value: &str) {
        self.cookies.insert(name.into(), value.into());
    }
    pub fn cookie(&self, name: &str) -> Option<&String> {
        self.cookies.get(name)
    }
    pub fn drop_cookie(&mut self, name: &str) {
        self.cookies.remove(name);
    }
    /// Sends `req` with the jar's cookies, and keeps the cookies the answer sets (or drops those it expires).
    pub async fn send(&mut self, mut req: http::Request<Body>) -> Reply {
        let jar = self.cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
        if !jar.is_empty() {
            req.headers_mut().insert(header::COOKIE, jar.parse().unwrap());
        }
        req.headers_mut().insert("x-forwarded-for", self.forwarded_for.parse().unwrap());
        let res = self.app.clone().oneshot(req).await.unwrap();
        let (parts, body) = res.into_parts();
        let reply =
            Reply { status: parts.status, headers: parts.headers, body: String::from_utf8_lossy(&body.collect().await.unwrap().to_bytes()).into_owned() };
        for set in reply.set_cookies() {
            let (pair, attrs) = set.split_once(';').unwrap_or((set.as_str(), ""));
            let (name, value) = pair.split_once('=').unwrap();
            if attrs.contains("Max-Age=0") {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.into(), value.into());
            }
        }
        reply
    }
    pub async fn get(&mut self, path: &str) -> Reply {
        self.send(http::Request::builder().uri(path).body(Body::empty()).unwrap()).await
    }
    pub async fn post_form(&mut self, path: &str, fields: &[(String, String)]) -> Reply {
        let body = url::form_urlencoded::Serializer::new(String::new()).extend_pairs(fields).finish();
        self.send(
            http::Request::builder().method("POST").uri(path).header(header::CONTENT_TYPE, "application/x-www-form-urlencoded").body(Body::from(body)).unwrap(),
        )
        .await
    }
    /// A request with a body-less method and the page's own Origin.
    pub async fn request(&mut self, method: &str, path: &str, origin: Option<&str>, json: Option<serde_json::Value>) -> Reply {
        let mut b = http::Request::builder().method(method).uri(path);
        if let Some(o) = origin {
            b = b.header(header::ORIGIN, o);
        }
        let body = match json {
            Some(j) => {
                b = b.header(header::CONTENT_TYPE, "application/json");
                Body::from(j.to_string())
            }
            None => Body::empty(),
        };
        self.send(b.body(body).unwrap()).await
    }
}

fn path_and_query(url: &str) -> String {
    let u = url::Url::parse(url).unwrap_or_else(|_| url::Url::parse(&format!("http://x{url}")).unwrap());
    match u.query() {
        Some(q) => format!("{}?{q}", u.path()),
        None => u.path().to_string(),
    }
}

/// The browser goes to the provider's authorize URL as `login`, and on to our callback.
pub async fn play_provider(b: &mut Browser, provider_url: &str, login: &FakeLogin) -> Reply {
    b.set_cookie(LOGIN_COOKIE, &login.cookie_value());
    let first = b.get(&path_and_query(provider_url)).await;
    if first.status == StatusCode::OK {
        let (action, fields) = parse_form_post(&first.body).expect("a form_post page");
        return b.post_form(&path_and_query(&action), &fields).await;
    }
    b.get(&path_and_query(&first.location().expect("a redirect back"))).await
}

pub async fn sign_in(b: &mut Browser, slug: &str, return_path: &str, login: &FakeLogin) -> Reply {
    let enc = percent_encoding::utf8_percent_encode(return_path, percent_encoding::NON_ALPHANUMERIC);
    let started = b.get(&format!("/auth/{slug}/start?return={enc}")).await;
    assert_eq!(started.status, StatusCode::SEE_OTHER, "start: {}", started.body);
    play_provider(b, &started.location().unwrap(), login).await
}

/// The whole router over `repo` and `work`; `http: None` means the in-process fake provider.
pub fn account_router(
    repo: Arc<dyn Repo>,
    work: Arc<dyn WorkRepo>,
    http: Option<Arc<dyn HttpClient>>,
    clock: Arc<FixedClock>,
    fake: Arc<FakeState>,
    rate: RateLimit,
    work_rate: RateLimit,
) -> Router {
    let mut providers = Providers::from_config_for_test(&[]);
    for spec in fake_provider::specs(ISSUER) {
        providers = providers.with(spec);
    }
    let account = Arc::new(Account {
        repo,
        work,
        http: http.unwrap_or_else(|| Arc::new(InProcess::with_state(fake.clone()))),
        clock,
        providers,
        cookie_key: cookie::Key::from(&[7u8; 64]),
        base_url: "http://127.0.0.1:8137".parse().unwrap(),
        rate,
        work_rate,
        ready: Arc::new(std::sync::atomic::AtomicBool::new(true)), // `#[sqlx::test]` has migrated the database already
        fake: Some(fake),
    });
    let config = Config::from_lookup(&|_| None).unwrap();
    router(&config, Some(account))
}

/// A work store that is down.
pub struct NoWork;

#[async_trait::async_trait]
impl WorkRepo for NoWork {
    async fn pull(&self, _: uuid::Uuid, _: Ask, _: time::OffsetDateTime) -> Result<Pulled, WorkError> {
        Err(WorkError::Unavailable)
    }
    async fn push(&self, _: uuid::Uuid, _: Vec<Put>, _: time::OffsetDateTime) -> Result<Pushed, WorkError> {
        Err(WorkError::Unavailable)
    }
    async fn clear(&self, _: uuid::Uuid) -> Result<(), WorkError> {
        Err(WorkError::Unavailable)
    }
}

/// A store that is down: every call fails as `Unavailable`.
pub struct DownRepo;

#[async_trait::async_trait]
impl Repo for DownRepo {
    async fn find_identity(&self, _: &str, _: &str) -> Result<Option<uuid::Uuid>, RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn create_user(&self, _: &str, _: &IdentityInput) -> Result<uuid::Uuid, RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn add_identity(&self, _: uuid::Uuid, _: &IdentityInput) -> Result<(), RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn user(&self, _: uuid::Uuid) -> Result<Option<UserView>, RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn identities(&self, _: uuid::Uuid) -> Result<Vec<IdentityView>, RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn remove_identity(&self, _: uuid::Uuid, _: &str) -> Result<Removal, RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn set_storage(&self, _: uuid::Uuid, _: bool) -> Result<(), RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn delete_user(&self, _: uuid::Uuid) -> Result<(), RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn create_session(&self, _: [u8; 32], _: uuid::Uuid, _: time::OffsetDateTime, _: time::OffsetDateTime) -> Result<(), RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn session(&self, _: &[u8; 32]) -> Result<Option<SessionRow>, RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn touch_session(&self, _: &[u8; 32], _: time::OffsetDateTime, _: time::OffsetDateTime) -> Result<(), RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn delete_session(&self, _: &[u8; 32]) -> Result<(), RepoError> {
        Err(RepoError::Unavailable)
    }
    async fn delete_expired_sessions(&self, _: time::OffsetDateTime) -> Result<u64, RepoError> {
        Err(RepoError::Unavailable)
    }
}

pub struct TestApp {
    pub router: Router,
    pub repo: Arc<PgRepo>,
    pub clock: Arc<FixedClock>,
    pub fake: Arc<FakeState>,
}

impl TestApp {
    pub fn new(pool: PgPool) -> TestApp {
        TestApp::build(pool, RateLimit { period_secs: 1, burst: 1000 }, RateLimit { period_secs: 1, burst: 1000 }, None)
    }
    pub fn with_rate(pool: PgPool, rate: RateLimit) -> TestApp {
        TestApp::build(pool, rate, RateLimit { period_secs: 1, burst: 1000 }, None)
    }
    /// The work routes' own limit (the sign-in routes keep the generous one).
    pub fn with_work_rate(pool: PgPool, work_rate: RateLimit) -> TestApp {
        TestApp::build(pool, RateLimit { period_secs: 1, burst: 1000 }, work_rate, None)
    }
    /// The server's calls out go to `http` instead of the fake provider (a provider that is down, say).
    pub fn with_http(pool: PgPool, http: Arc<dyn HttpClient>) -> TestApp {
        TestApp::build(pool, RateLimit { period_secs: 1, burst: 1000 }, RateLimit { period_secs: 1, burst: 1000 }, Some(http))
    }
    fn build(pool: PgPool, rate: RateLimit, work_rate: RateLimit, http: Option<Arc<dyn HttpClient>>) -> TestApp {
        let clock = Arc::new(FixedClock::new(t0()));
        let fake = FakeState::new(ISSUER.into(), clock.clone());
        let repo = Arc::new(PgRepo::new(
            pool,
            MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32]))], 1).unwrap(),
            LookupKey::new(zeroize::Zeroizing::new([3; 32])),
        ));
        let router = account_router(repo.clone(), repo.clone(), http, clock.clone(), fake.clone(), rate, work_rate);
        TestApp { router, repo, clock, fake }
    }
    /// The router over a repository of the test's choosing (one that is down, say).
    pub fn with_repo(repo: Arc<dyn Repo>) -> Router {
        let clock = Arc::new(FixedClock::new(t0()));
        let fake = FakeState::new(ISSUER.into(), clock.clone());
        let rate = RateLimit { period_secs: 1, burst: 1000 };
        account_router(repo, Arc::new(NoWork), None, clock, fake, rate, rate)
    }
    pub fn browser(&self) -> Browser {
        Browser::new(self.router.clone())
    }
}
