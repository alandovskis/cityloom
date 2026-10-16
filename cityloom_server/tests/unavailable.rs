use axum::body::Body;
use axum::http::{Request, StatusCode};
use cityloom_server::config::Config;
use tower::ServiceExt;

#[tokio::test]
async fn auth_and_api_answer_503_when_the_account_side_is_off_and_the_site_is_still_served() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "home").unwrap();
    let root = dir.path().to_str().unwrap().to_string();
    let config = Config::from_lookup(&|k| (k == "WEB_ROOT").then(|| root.clone())).unwrap();
    let app = cityloom_server::app::router(&config, None);
    for (method, path) in [("GET", "/api/me"), ("GET", "/api/providers"), ("POST", "/auth/signout"), ("GET", "/auth/google/start"), ("DELETE", "/api/me")] {
        let res = app.clone().oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE, "{method} {path}");
        assert_eq!(res.headers()["cache-control"], "no-store");
    }
    let res = app.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
#[tokio::test]

async fn a_database_that_cannot_be_reached_answers_503_without_a_panic_and_the_site_is_still_served() {
    use base64::Engine;
    use cityloom_server::config::AccountSide;
    let b64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "home").unwrap();
    let vars: std::collections::HashMap<&str, String> = [
        ("WEB_ROOT", dir.path().to_str().unwrap().to_string()),
        ("DATABASE_URL", "postgres://nobody:nothing@127.0.0.1:1/none".into()), // nothing listens on port 1
        ("BASE_URL", "http://127.0.0.1:8137".into()),
        ("COOKIE_KEY", b64(&[7u8; 64])),
        ("DATA_KEYS", format!("1:{}", b64(&[1u8; 32]))),
        ("DATA_KEY_CURRENT", "1".into()),
        ("LOOKUP_KEY", b64(&[9u8; 32])),
    ]
    .into_iter()
    .collect();
    let config = Config::from_lookup(&|k| vars.get(k).cloned()).unwrap();
    let AccountSide::On(account_config) = &config.account else { panic!("the variables are complete, so the account side is on: {:?}", config.account) };
    let account = cityloom_server::build_account(account_config, std::sync::Arc::new(cityloom_server::clock::SystemClock))
        .await
        .expect("no connection is made at start-up, so this cannot fail on a closed port");
    let app = cityloom_server::app::router(&config, Some(account.clone()));
    for (method, path) in [("GET", "/api/me"), ("GET", "/api/providers"), ("GET", "/auth/google/start"), ("POST", "/auth/signout"), ("DELETE", "/api/me")] {
        let res = app.clone().oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE, "{method} {path}");
        assert_eq!(res.headers()["cache-control"], "no-store");
    }
    let res = app.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!account.ready.load(std::sync::atomic::Ordering::Acquire), "not ready, and still trying in the background, not off for good");
}

#[tokio::test]
async fn the_bare_auth_and_api_paths_answer_503_too_when_the_account_side_is_off() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "home").unwrap();
    std::fs::write(dir.path().join("auth"), "a file called auth").unwrap();
    let root = dir.path().to_str().unwrap().to_string();
    let config = Config::from_lookup(&|k| (k == "WEB_ROOT").then(|| root.clone())).unwrap();
    let app = cityloom_server::app::router(&config, None);
    for path in ["/auth", "/api", "/auth/", "/api/", "/auth/a/b/c"] {
        let res = app.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
    }
}
