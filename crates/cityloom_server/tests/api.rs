#![cfg(feature = "test-provider")]
mod support;

use std::sync::Arc;

use axum::http::{StatusCode, header};
use cityloom_server::accounts::Repo;
use cityloom_server::fake_provider::FakeLogin;
use sqlx::PgPool;
use support::*;

const ORIGIN: &str = "http://127.0.0.1:8137";
const SESSION: &str = "__Host-cityloom-session";

fn login(sub: &str) -> FakeLogin {
    FakeLogin { sub: Some(sub.into()), name: Some("Ada".into()), email: Some(format!("{sub}@example.org")), ..Default::default() }
}

async fn signed_in(app: &TestApp, sub: &str) -> Browser {
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &login(sub)).await;
    b
}

async fn link(b: &mut Browser, slug: &str, who: &FakeLogin) {
    let started = b.get(&format!("/auth/{slug}/start?link=1&return=/account.html")).await;
    let r = play_provider(b, &started.location().unwrap(), who).await;
    assert!(r.location().unwrap().ends_with("#account=linked"), "{:?}", r.location());
}

#[sqlx::test]
async fn providers_are_listed_without_a_session_and_are_never_cached(pool: PgPool) {
    let app = TestApp::new(pool);
    let r = app.browser().get("/api/providers").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers[header::CACHE_CONTROL], "no-store");
    let slugs: Vec<_> = r.json()["providers"].as_array().unwrap().iter().map(|p| p["slug"].as_str().unwrap().to_string()).collect();
    assert_eq!(slugs, ["fake", "fake-apple"]);
    assert_eq!(r.json()["providers"][0]["name"], "Fake");
}

#[sqlx::test]
async fn me_is_401_without_a_session_and_clears_a_stale_cookie(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let r = b.get("/api/me").await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::UNAUTHORIZED, Some("signed-out")));
    assert_eq!(r.headers[header::CACHE_CONTROL], "no-store");
    b.set_cookie(SESSION, &cityloom_server::session::new_token());
    let r = b.get("/api/me").await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert!(r.set_cookies().iter().any(|c| c.starts_with(SESSION) && c.contains("Max-Age=0")), "the stale cookie is cleared");
    assert!(b.cookie(SESSION).is_none());
}

#[sqlx::test]
async fn me_describes_the_account_and_never_shows_the_sub(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = signed_in(&app, "sub-zz-91").await;
    let r = b.get("/api/me").await;
    assert_eq!(r.status, StatusCode::OK);
    let me = r.json();
    assert_eq!((me["display_name"].as_str(), me["store_on_server"].clone()), (Some("Ada"), serde_json::Value::Null));
    assert!(uuid::Uuid::parse_str(me["id"].as_str().unwrap()).is_ok());
    let i = &me["identities"][0];
    assert_eq!((i["provider"].as_str(), i["label"].as_str()), (Some("fake"), Some("sub-zz-91@example.org")));
    assert_eq!(i["id"].as_str().unwrap().len(), 64);
    assert!(time::OffsetDateTime::parse(i["created_at"].as_str().unwrap(), &time::format_description::well_known::Rfc3339).is_ok());
    assert!(!r.body.contains("\"sub\"") && !r.body.contains("sub-zz-91\""), "{}", r.body);
}

#[sqlx::test]
async fn the_storage_choice_is_set_and_changed(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = signed_in(&app, "s1").await;
    assert_eq!(b.request("PUT", "/api/me/storage", Some(ORIGIN), Some(serde_json::json!({"store_on_server": true}))).await.status, StatusCode::NO_CONTENT);
    assert_eq!(b.get("/api/me").await.json()["store_on_server"], true);
    assert_eq!(b.request("PUT", "/api/me/storage", Some(ORIGIN), Some(serde_json::json!({"store_on_server": false}))).await.status, StatusCode::NO_CONTENT);
    assert_eq!(b.get("/api/me").await.json()["store_on_server"], false);
    for bad in [serde_json::json!({}), serde_json::json!({"store_on_server": "yes"}), serde_json::json!({"store_on_server": null}), serde_json::json!([true])] {
        assert_eq!(b.request("PUT", "/api/me/storage", Some(ORIGIN), Some(bad.clone())).await.status, StatusCode::BAD_REQUEST, "{bad}");
    }
}

#[sqlx::test]
async fn unlinking_removes_one_identity_refuses_the_last_and_the_unknown(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = signed_in(&app, "one").await;
    let only = b.get("/api/me").await.json()["identities"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(b.request("DELETE", &format!("/api/me/identities/{only}"), Some(ORIGIN), None).await.status, StatusCode::CONFLICT);
    link(&mut b, "fake-apple", &login("two")).await;
    assert_eq!(b.get("/api/me").await.json()["identities"].as_array().unwrap().len(), 2);
    assert_eq!(b.request("DELETE", &format!("/api/me/identities/{only}"), Some(ORIGIN), None).await.status, StatusCode::NO_CONTENT);
    let me = b.get("/api/me").await.json();
    assert_eq!((me["identities"].as_array().unwrap().len(), me["identities"][0]["provider"].as_str()), (1, Some("fake-apple")));
    let last = me["identities"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(b.request("DELETE", &format!("/api/me/identities/{last}"), Some(ORIGIN), None).await.status, StatusCode::CONFLICT);
    assert_eq!(b.request("DELETE", &format!("/api/me/identities/{}", "ab".repeat(32)), Some(ORIGIN), None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(b.request("DELETE", "/api/me/identities/not-hex", Some(ORIGIN), None).await.status, StatusCode::NOT_FOUND);
    // Another account's identity is not found here.
    let mut other = signed_in(&app, "theirs").await;
    let theirs = other.get("/api/me").await.json()["identities"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(b.request("DELETE", &format!("/api/me/identities/{theirs}"), Some(ORIGIN), None).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn deleting_the_account_leaves_no_row_and_ends_the_session(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let mut b = signed_in(&app, "s1").await;
    link(&mut b, "fake-apple", &login("s2")).await;
    let user = app.repo.find_identity("fake", "s1").await.unwrap().unwrap();
    let r = b.request("DELETE", "/api/me", Some(ORIGIN), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(b.cookie(SESSION).is_none());
    for (table, sql) in [
        ("users", "select count(*) from users where id = $1"),
        ("identities", "select count(*) from identities where user_id = $1"),
        ("sessions", "select count(*) from sessions where user_id = $1"),
    ] {
        let n: i64 = sqlx::query_scalar(sql).bind(user).fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0, "{table}");
    }
    assert_eq!(app.repo.find_identity("fake-apple", "s2").await.unwrap(), None);
}

#[sqlx::test]
async fn a_deleted_account_whose_cookie_lives_on_in_another_tab_is_401_everywhere_never_500(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut tab_a = signed_in(&app, "s1").await;
    let mut tab_b = app.browser(); // another browser context that kept the same cookie
    tab_b.set_cookie(SESSION, &tab_a.cookie(SESSION).cloned().unwrap());
    assert_eq!(tab_b.get("/api/me").await.status, StatusCode::OK);
    assert_eq!(tab_a.request("DELETE", "/api/me", Some(ORIGIN), None).await.status, StatusCode::NO_CONTENT);
    let stale = tab_b.get("/api/me").await;
    assert_eq!(stale.status, StatusCode::UNAUTHORIZED);
    assert!(stale.set_cookies().iter().any(|c| c.contains("Max-Age=0")), "cleared in the other tab");
    tab_b.set_cookie(SESSION, "x".repeat(43).as_str());
    for (method, path, body) in [
        ("PUT", "/api/me/storage", Some(serde_json::json!({"store_on_server": true}))),
        ("DELETE", "/api/me/identities/abab", None),
        ("DELETE", "/api/me", None),
    ] {
        assert_eq!(tab_b.request(method, path, Some(ORIGIN), body).await.status, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
    // Signing in again with the same identity is a brand new account, not a resurrection.
    let mut again = app.browser();
    let r = sign_in(&mut again, "fake", "/", &login("s1")).await;
    assert_eq!(r.location().as_deref(), Some("/account.html#account=new"));
}

#[sqlx::test]
async fn every_state_changing_call_needs_this_sites_origin(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = signed_in(&app, "s1").await;
    for origin in [None, Some("https://evil.example"), Some("http://127.0.0.1:9999")] {
        for (method, path, body) in [
            ("PUT", "/api/me/storage", Some(serde_json::json!({"store_on_server": true}))),
            ("DELETE", "/api/me/identities/abab", None),
            ("DELETE", "/api/me", None),
        ] {
            assert_eq!(b.request(method, path, origin, body).await.status, StatusCode::FORBIDDEN, "{method} {path} {origin:?}");
        }
    }
    assert_eq!(b.get("/api/me").await.status, StatusCode::OK, "nothing was deleted");
}

#[sqlx::test]
async fn a_day_old_session_is_refreshed_by_the_next_call(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = signed_in(&app, "s1").await;
    assert!(b.get("/api/me").await.set_cookies().is_empty(), "a fresh session is left alone");
    app.clock.advance(time::Duration::days(1) + time::Duration::minutes(1));
    let r = b.get("/api/me").await;
    assert!(r.set_cookies().iter().any(|c| c.starts_with(SESSION) && c.contains("Max-Age=2592000")), "{:?}", r.set_cookies());
}

#[tokio::test]
async fn a_store_that_is_down_is_503_and_unknown_api_paths_are_404() {
    let app = TestApp::with_repo(Arc::new(DownRepo));
    let mut b = Browser::new(app);
    b.set_cookie(SESSION, &cityloom_server::session::new_token());
    let r = b.get("/api/me").await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::SERVICE_UNAVAILABLE, Some("unavailable")));
    assert_eq!(b.get("/api/nope").await.status, StatusCode::NOT_FOUND);
    assert_eq!(b.get("/api/providers").await.status, StatusCode::OK, "the list of providers needs no store");
}
