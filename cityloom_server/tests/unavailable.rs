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
    let app = cityloom_server::app::router(&config);
    for (method, path) in [("GET", "/api/me"), ("GET", "/api/providers"), ("POST", "/auth/signout"), ("GET", "/auth/google/start"), ("DELETE", "/api/me")] {
        let res = app.clone().oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE, "{method} {path}");
        assert_eq!(res.headers()["cache-control"], "no-store");
    }
    let res = app.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
