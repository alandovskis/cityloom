#![cfg(feature = "test-provider")]
mod support;

use axum::body::Body;
use axum::http::{StatusCode, header};
use cityloom_server::config::RateLimit;
use cityloom_server::fake_provider::FakeLogin;
use cityloom_server::share::{MAX_BODY_BYTES, MAX_LIVE_SHARES, MAX_SHARES_PER_DAY};
use serde_json::{Value, json};
use sqlx::PgPool;
use support::*;

const ORIGIN: &str = "http://127.0.0.1:8137";

fn login(sub: &str) -> FakeLogin {
    FakeLogin { sub: Some(sub.into()), name: Some("Ada".into()), email: Some(format!("{sub}@example.org")), ..Default::default() }
}

async fn signed_in(app: &TestApp, sub: &str) -> Browser {
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &login(sub)).await;
    b
}

fn design() -> Value {
    json!({
        "kind": "cityloom-share", "format_version": 1, "app_version": "0.96.0", "created_at": "2027-01-15T08:00:00.000Z",
        "project": {"name": "Plateau", "area": "area=45.5230,-73.5810&name=Plateau"},
        "edits": {"version": 6, "streets": {"w1~n2~n3": {"street": {"name": "Rue Rachel"}}}, "junctions": {}},
        "restrictions": {"version": 1, "restrictions": {"r1": {"name": "Car-free"}}},
        "future": [1, 2]
    })
}

async fn share(b: &mut Browser, body: &Value) -> Reply {
    b.request("POST", "/api/share", Some(ORIGIN), Some(body.clone())).await
}

/// Reads the share behind `token`, which rides in the `Share-Token` header: never in the path, which proxies log.
async fn open_link(b: &mut Browser, token: &str) -> Reply {
    b.send(http::Request::builder().uri("/api/share").header("share-token", token).body(Body::empty()).unwrap()).await
}

/// Deletes the share behind `token` (in the `Share-Token` header), from the page at `origin`.
async fn delete_link(b: &mut Browser, token: &str, origin: Option<&str>) -> Reply {
    let mut req = http::Request::builder().method("DELETE").uri("/api/share").header("share-token", token);
    if let Some(origin) = origin {
        req = req.header(header::ORIGIN, origin);
    }
    b.send(req.body(Body::empty()).unwrap()).await
}

async fn made(b: &mut Browser) -> String {
    let r = share(b, &design()).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.json()["token"].as_str().unwrap().to_string()
}

#[sqlx::test]
async fn a_share_is_made_signed_in_and_read_by_anyone_with_the_link(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let r = share(&mut ada, &design()).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.headers[header::CACHE_CONTROL], "no-store");
    let out = r.json();
    let token = out["token"].as_str().unwrap();
    assert_eq!(token.len(), 43, "256 bits as base64url");
    assert_eq!(out["expires_at"].as_str(), Some("2027-04-15T08:00:00.000Z"), "90 days after t0 (2027-01-15T08:00Z)");

    let mut anyone = app.browser(); // no account, no cookie
    let got = open_link(&mut anyone, &token).await;
    assert_eq!(got.status, StatusCode::OK, "{}", got.body);
    assert_eq!(got.json(), design(), "the body comes back as sent, a newer build's fields too");
    assert_eq!(got.headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(got.headers[header::REFERRER_POLICY], "no-referrer");
    assert_eq!(got.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff", "the body as sent is never read as anything but JSON");
    assert_eq!(r.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff", "nor is any other answer of the API");
    assert_ne!(made(&mut ada).await, token, "each share has a link of its own");
}

#[sqlx::test]
async fn signed_out_nobody_makes_or_deletes_a_share(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut anyone = app.browser();
    assert_eq!(share(&mut anyone, &design()).await.status, StatusCode::UNAUTHORIZED);
    let mut ada = signed_in(&app, "ada").await;
    let token = made(&mut ada).await;
    assert_eq!(delete_link(&mut anyone, &token, Some(ORIGIN)).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(open_link(&mut anyone, &token).await.status, StatusCode::OK, "still there");
}

#[sqlx::test]
async fn a_change_needs_the_sites_own_origin(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let r = ada.request("POST", "/api/share", Some("https://evil.example"), Some(design())).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::FORBIDDEN, Some("origin")));
    let token = made(&mut ada).await;
    let r = delete_link(&mut ada, &token, None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn an_unknown_or_malformed_link_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut anyone = app.browser();
    for token in ["AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "not-a-token", "..", ""] {
        let r = open_link(&mut anyone, token).await;
        assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::NOT_FOUND, Some("not-found")), "{token}");
    }
    let r = anyone.get("/api/share").await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::NOT_FOUND, Some("not-found")), "no token at all");
}

#[sqlx::test]
async fn after_ninety_days_the_link_says_expired(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let token = made(&mut ada).await;
    let mut anyone = app.browser();
    app.clock.advance(time::Duration::days(90) - time::Duration::seconds(1));
    assert_eq!(open_link(&mut anyone, &token).await.status, StatusCode::OK, "the last second of the 90 days");
    app.clock.advance(time::Duration::seconds(1));
    let r = open_link(&mut anyone, &token).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::GONE, Some("expired")));
    let again = open_link(&mut anyone, &token).await;
    assert_eq!((again.status, again.json()["error"].as_str()), (StatusCode::GONE, Some("expired")), "and so it stays, body gone");
}

#[sqlx::test]
async fn the_creator_deletes_a_share_and_its_link_then_says_so(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let token = made(&mut ada).await;
    let mut bea = signed_in(&app, "bea").await;
    let r = delete_link(&mut bea, &token, Some(ORIGIN)).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::FORBIDDEN, Some("not-yours")));
    assert_eq!(open_link(&mut bea, &token).await.status, StatusCode::OK, "another account cannot delete it");

    let r = delete_link(&mut ada, &token, Some(ORIGIN)).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let r = open_link(&mut bea, &token).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::GONE, Some("deleted")));
    assert_eq!(delete_link(&mut ada, &token, Some(ORIGIN)).await.status, StatusCode::NO_CONTENT, "again: nothing to do");
    let r = delete_link(&mut ada, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", Some(ORIGIN)).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    app.clock.advance(time::Duration::days(91));
    let r = open_link(&mut app.browser(), &token).await;
    assert_eq!(r.json()["error"].as_str(), Some("deleted"), "deleted, not expired, after the 90 days too");
}

#[sqlx::test]
async fn a_body_that_is_no_share_newer_or_too_large_is_refused(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let mut newer = design();
    newer["format_version"] = json!(2);
    let r = share(&mut ada, &newer).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::UNPROCESSABLE_ENTITY, Some("newer")));
    let r = share(&mut ada, &json!({"kind": "cityloom-design", "format_version": 1})).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::BAD_REQUEST, Some("bad-request")));
    let r = ada.send(http::Request::builder().method("POST").uri("/api/share").header(header::ORIGIN, ORIGIN).body(Body::from("not json")).unwrap()).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let mut big = design();
    big["pad"] = json!(" ".repeat(MAX_BODY_BYTES));
    let r = share(&mut ada, &big).await;
    assert_eq!((r.status, r.json()["error"].as_str(), r.json()["limit"].as_str()), (StatusCode::PAYLOAD_TOO_LARGE, Some("too-large"), Some("share")));
    let mut far_too_big = design();
    far_too_big["pad"] = json!(" ".repeat(3 * MAX_BODY_BYTES));
    assert_eq!(share(&mut ada, &far_too_big).await.status, StatusCode::PAYLOAD_TOO_LARGE, "refused before it is read whole");
}

#[sqlx::test]
async fn an_account_holds_at_most_fifty_live_shares(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let mut tokens = Vec::new();
    for _ in 0..MAX_LIVE_SHARES {
        tokens.push(made(&mut ada).await);
    }
    let r = share(&mut ada, &design()).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::CONFLICT, Some("full")));
    assert_eq!(delete_link(&mut ada, &tokens[0], Some(ORIGIN)).await.status, StatusCode::NO_CONTENT);
    made(&mut ada).await; // a deleted one no longer counts
    let mut bea = signed_in(&app, "bea").await;
    made(&mut bea).await; // the cap is per account
    app.clock.advance(time::Duration::days(90));
    let mut ada = signed_in(&app, "ada").await; // the session lasts 30 days
    made(&mut ada).await; // nor do expired ones
}

#[sqlx::test]
async fn deleting_the_account_deletes_its_shares(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let token = made(&mut ada).await;
    assert_eq!(ada.request("DELETE", "/api/me", Some(ORIGIN), None).await.status, StatusCode::NO_CONTENT);
    let r = open_link(&mut app.browser(), &token).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn reading_links_has_its_own_rate_limit(pool: PgPool) {
    let app = TestApp::with_share_rate(pool, RateLimit { period_secs: 3600, burst: 2 });
    let mut anyone = app.browser();
    let token = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    assert_eq!(open_link(&mut anyone, token).await.status, StatusCode::NOT_FOUND);
    assert_eq!(open_link(&mut anyone, token).await.status, StatusCode::NOT_FOUND);
    assert_eq!(open_link(&mut anyone, token).await.status, StatusCode::TOO_MANY_REQUESTS);
    anyone.forwarded_for = "198.51.100.9".into();
    assert_eq!(open_link(&mut anyone, token).await.status, StatusCode::NOT_FOUND, "counted per address");
    assert_eq!(anyone.get("/api/providers").await.status, StatusCode::OK, "the other routes are not counted with it");
}

#[sqlx::test]
async fn an_account_makes_a_bounded_number_of_shares_a_day(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    for _ in 0..MAX_SHARES_PER_DAY {
        let token = made(&mut ada).await;
        assert_eq!(delete_link(&mut ada, &token, Some(ORIGIN)).await.status, StatusCode::NO_CONTENT);
    }
    let r = share(&mut ada, &design()).await;
    assert_eq!(
        (r.status, r.json()["error"].as_str(), r.json()["limit"].as_u64()),
        (StatusCode::CONFLICT, Some("too-many-today"), Some(MAX_SHARES_PER_DAY as u64))
    );
}

#[sqlx::test]
async fn a_link_s_token_is_never_in_a_path(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut ada = signed_in(&app, "ada").await;
    let token = made(&mut ada).await;
    // The token in a path, as an older page sent it, is no route: a proxy's log of the path holds no working link.
    let r = app.browser().get(&format!("/api/share/{token}")).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::NOT_FOUND, Some("not-found")));
    assert_eq!(ada.request("DELETE", &format!("/api/share/{token}"), Some(ORIGIN), None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(open_link(&mut app.browser(), &token).await.status, StatusCode::OK, "neither deleted it");
}
