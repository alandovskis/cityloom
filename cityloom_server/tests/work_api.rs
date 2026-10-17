#![cfg(feature = "test-provider")]
mod support;

use std::sync::Arc;

use axum::http::{StatusCode, header};
use cityloom_server::config::RateLimit;
use cityloom_server::fake_provider::FakeLogin;
use serde_json::{Value, json};
use sqlx::PgPool;
use support::*;

const ORIGIN: &str = "http://127.0.0.1:8137";
const EARLY: &str = "2027-01-15T06:00:00.000Z";
const MIDDLE: &str = "2027-01-15T07:00:00.000Z";
const LATE: &str = "2027-01-15T07:30:00.000Z";

fn login(sub: &str) -> FakeLogin {
    FakeLogin { sub: Some(sub.into()), name: Some("Ada".into()), email: Some(format!("{sub}@example.org")), ..Default::default() }
}

async fn signed_in(app: &TestApp, sub: &str) -> Browser {
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &login(sub)).await;
    b
}

async fn storing(app: &TestApp, sub: &str) -> Browser {
    let mut b = signed_in(app, sub).await;
    let r = b.request("PUT", "/api/me/storage", Some(ORIGIN), Some(json!({"store_on_server": true}))).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    b
}

fn record(kind: &str, key: &str, base_rev: i64, edited_at: &str, body: Option<Value>) -> Value {
    json!({"kind": kind, "key": key, "base_rev": base_rev, "edited_at": edited_at, "deleted": body.is_none(), "body": body})
}

async fn put(b: &mut Browser, records: Vec<Value>) -> Reply {
    b.request("PUT", "/api/work", Some(ORIGIN), Some(json!({"records": records}))).await
}

#[sqlx::test]
async fn signed_out_every_route_is_401(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    assert_eq!(b.get("/api/work").await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(put(&mut b, vec![]).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(b.request("DELETE", "/api/work", Some(ORIGIN), None).await.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn without_storage_on_every_route_is_409_storage_off(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = signed_in(&app, "a").await; // nothing chosen yet
    let got = b.get("/api/work").await;
    let pushed = put(&mut b, vec![]).await;
    let deleted = b.request("DELETE", "/api/work", Some(ORIGIN), None).await;
    for r in [got, pushed, deleted] {
        assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::CONFLICT, Some("storage-off")));
    }
    b.request("PUT", "/api/me/storage", Some(ORIGIN), Some(json!({"store_on_server": false}))).await;
    assert_eq!(b.get("/api/work").await.status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn a_record_is_stored_and_comes_back_with_a_newer_builds_fields_untouched(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    let body = json!({"street": {"row_mm": 15000}, "future": {"x": [1, 2], "y": null}});
    let r = put(&mut b, vec![record("street", "w1~n2~n3", 0, MIDDLE, Some(body.clone()))]).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.headers[header::CACHE_CONTROL], "no-store");
    let out = r.json();
    assert_eq!((out["rev"].clone(), out["results"][0]["status"].as_str(), out["results"][0]["rev"].clone()), (json!(1), Some("accepted"), json!(1)));
    assert!(out["results"][0].get("record").is_none());
    let pulled = b.get("/api/work?since=0").await.json();
    assert_eq!((pulled["rev"].clone(), pulled["more"].clone()), (json!(1), json!(false)));
    assert_eq!(pulled["epoch"].as_str().map(str::len), Some(36), "the copy the device read, a uuid");
    let rec = &pulled["records"][0];
    assert_eq!((rec["kind"].as_str(), rec["key"].as_str(), rec["deleted"].clone()), (Some("street"), Some("w1~n2~n3"), json!(false)));
    assert_eq!(rec["edited_at"].as_str(), Some(MIDDLE));
    assert_eq!(rec["body"], body);
}

#[sqlx::test]
async fn the_later_edit_wins_and_the_loser_is_told_what_won(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut a = storing(&app, "same-person").await;
    let mut other = app.browser();
    sign_in(&mut other, "fake", "/", &login("same-person")).await; // the same account on another device
    put(&mut a, vec![record("rule", "r-1", 0, MIDDLE, Some(json!({"name": "A"})))]).await;
    let r = put(&mut other, vec![record("rule", "r-1", 0, EARLY, Some(json!({"name": "B"})))]).await.json();
    assert_eq!((r["results"][0]["status"].as_str(), r["results"][0]["rev"].clone()), (Some("superseded"), json!(1)));
    assert_eq!(r["results"][0]["record"]["body"], json!({"name": "A"}));
    assert_eq!(r["rev"], json!(1), "nothing changed");
    let r = put(&mut other, vec![record("rule", "r-1", 0, LATE, Some(json!({"name": "B"})))]).await.json();
    assert_eq!((r["results"][0]["status"].as_str(), r["results"][0]["rev"].clone()), (Some("accepted"), json!(2)));
}

#[sqlx::test]
async fn a_deletion_reaches_a_device_that_pulls_from_before_it(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    put(&mut b, vec![record("rule", "r-1", 0, EARLY, Some(json!({})))]).await;
    put(&mut b, vec![record("rule", "r-1", 1, MIDDLE, None)]).await;
    let since_one = b.get("/api/work?since=1").await.json();
    let rec = &since_one["records"][0];
    assert_eq!((rec["deleted"].clone(), rec["rev"].clone(), rec["body"].clone()), (json!(true), json!(2), Value::Null));
    assert_eq!(b.get("/api/work?since=0").await.json()["records"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn what_the_server_will_not_take_is_refused_whole(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    let ok = record("street", "fine", 0, MIDDLE, Some(json!({})));
    for bad in [
        record("shape", "k", 0, MIDDLE, Some(json!({}))),
        record("street", "../etc/passwd", 0, MIDDLE, Some(json!({}))),
        record("street", "a/b", 0, MIDDLE, Some(json!({}))),
        record("street", &"k".repeat(201), 0, MIDDLE, Some(json!({}))),
        record("street", "k", -1, MIDDLE, Some(json!({}))),
        record("street", "k", 0, "yesterday", Some(json!({}))),
        json!({"kind": "street", "key": "k", "base_rev": 0, "edited_at": MIDDLE, "deleted": false, "body": null}),
        json!({"kind": "street", "key": "k", "base_rev": 0, "edited_at": MIDDLE, "deleted": true, "body": {}}),
    ] {
        let r = put(&mut b, vec![ok.clone(), bad.clone()]).await;
        assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::BAD_REQUEST, Some("bad-request")), "{bad}");
    }
    assert_eq!(put(&mut b, vec![ok.clone(), ok.clone()]).await.status, StatusCode::BAD_REQUEST, "a place twice");
    assert_eq!(b.request("PUT", "/api/work", Some(ORIGIN), Some(json!({"nope": 1}))).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(b.get("/api/work?since=0").await.json()["records"].as_array().unwrap().len(), 0, "none of the batches left anything");
}

#[sqlx::test]
async fn a_limit_is_a_413_that_names_it(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    let big = json!({"x": "y".repeat(65 * 1024)});
    let r = put(&mut b, vec![record("street", "big", 0, MIDDLE, Some(big))]).await;
    assert_eq!((r.status, r.json()["error"].as_str(), r.json()["limit"].as_str()), (StatusCode::PAYLOAD_TOO_LARGE, Some("too-large"), Some("record")));
    let many: Vec<Value> = (0..101).map(|i| record("street", &format!("k{i}"), 0, MIDDLE, Some(json!({})))).collect();
    let r = put(&mut b, many).await;
    assert_eq!((r.status, r.json()["limit"].as_str()), (StatusCode::PAYLOAD_TOO_LARGE, Some("batch")));
    let huge = json!({"x": "y".repeat(8 * 1024 * 1024)});
    let r = put(&mut b, vec![record("street", "huge", 0, MIDDLE, Some(huge))]).await;
    assert_eq!((r.status, r.json()["limit"].as_str()), (StatusCode::PAYLOAD_TOO_LARGE, Some("batch")), "a request over the whole body limit is JSON too");
}

#[sqlx::test]
async fn the_session_and_the_storage_choice_are_checked_before_the_body_is_read(pool: PgPool) {
    let app = TestApp::new(pool);
    let huge = json!({"records": [record("street", "huge", 0, MIDDLE, Some(json!({"x": "y".repeat(8 * 1024 * 1024)})))]});
    let mut out = app.browser();
    assert_eq!(out.request("PUT", "/api/work", Some(ORIGIN), Some(huge.clone())).await.status, StatusCode::UNAUTHORIZED);
    let mut b = signed_in(&app, "a").await; // storage not chosen
    let r = b.request("PUT", "/api/work", Some(ORIGIN), Some(huge)).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::CONFLICT, Some("storage-off")), "an 8 MB body is not looked at, so it is not a 413");
}

#[sqlx::test]
async fn writes_need_this_sites_origin_and_reads_do_not(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    for origin in [None, Some("https://evil.example")] {
        assert_eq!(b.request("PUT", "/api/work", origin, Some(json!({"records": []}))).await.status, StatusCode::FORBIDDEN);
        assert_eq!(b.request("DELETE", "/api/work", origin, None).await.status, StatusCode::FORBIDDEN);
    }
    assert_eq!(b.get("/api/work").await.status, StatusCode::OK);
}

#[sqlx::test]
async fn delete_clears_the_copy_and_the_revision(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    put(&mut b, vec![record("area", "current", 0, MIDDLE, Some(json!({"query": "area=45.5,-73.6&name=X"})))]).await;
    assert_eq!(b.request("DELETE", "/api/work", Some(ORIGIN), None).await.status, StatusCode::NO_CONTENT);
    let pulled = b.get("/api/work?since=0").await.json();
    assert_eq!((pulled["rev"].clone(), pulled["records"].as_array().unwrap().len()), (json!(0), 0));
}

#[sqlx::test]
async fn turning_storage_off_deletes_the_copy_and_a_queued_push_is_refused(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let mut phone = storing(&app, "p").await;
    let mut laptop = app.browser();
    sign_in(&mut laptop, "fake", "/", &login("p")).await;
    put(&mut phone, vec![record("street", "k", 0, MIDDLE, Some(json!({})))]).await;
    assert_eq!(phone.request("PUT", "/api/me/storage", Some(ORIGIN), Some(json!({"store_on_server": false}))).await.status, StatusCode::NO_CONTENT);
    let queued = put(&mut laptop, vec![record("street", "k", 1, LATE, Some(json!({})))]).await;
    assert_eq!((queued.status, queued.json()["error"].as_str()), (StatusCode::CONFLICT, Some("storage-off")));
    let n: i64 = sqlx::query_scalar("select count(*) from work").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test]
async fn deleting_the_account_removes_the_work(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let mut b = storing(&app, "a").await;
    put(&mut b, vec![record("street", "k", 0, MIDDLE, Some(json!({})))]).await;
    assert_eq!(b.request("DELETE", "/api/me", Some(ORIGIN), None).await.status, StatusCode::NO_CONTENT);
    let n: i64 = sqlx::query_scalar("select count(*) from work").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test]
async fn a_device_from_before_the_purge_gets_410_resync(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    put(&mut b, vec![record("rule", "r-1", 0, EARLY, Some(json!({})))]).await;
    put(&mut b, vec![record("rule", "r-1", 1, MIDDLE, None)]).await; // a tombstone, revision 2
    app.clock.advance(time::Duration::days(91));
    sign_in(&mut b, "fake", "/", &login("a")).await; // the 30-day session ended too: the same account signs in again
    let r = b.get("/api/work?since=1").await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::GONE, Some("resync")));
    assert_eq!(b.get("/api/work?since=0").await.status, StatusCode::OK);
    assert_eq!(b.get("/api/work?since=2").await.status, StatusCode::OK);
    assert_eq!(b.get("/api/work?since=-1").await.status, StatusCode::BAD_REQUEST);
    // the pages after the first of a pull are judged by where the pull began (`start`), not by their own `since`
    assert_eq!(b.get("/api/work?since=1&start=0").await.status, StatusCode::OK);
    assert_eq!(b.get("/api/work?since=1&start=1").await.status, StatusCode::GONE);
    assert_eq!(b.get("/api/work?since=1&start=-1").await.status, StatusCode::BAD_REQUEST);
    assert_eq!(b.get("/api/work?since=1&start=2").await.status, StatusCode::BAD_REQUEST, "a pull cannot begin after the page it is on");
}

#[sqlx::test]
async fn a_device_that_missed_an_off_and_on_again_is_told_to_reset(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = storing(&app, "a").await;
    put(&mut b, vec![record("street", "k", 0, MIDDLE, Some(json!({})))]).await;
    let epoch = b.get("/api/work?since=0").await.json()["epoch"].as_str().unwrap().to_string();
    assert_eq!(b.get(&format!("/api/work?since=1&epoch={epoch}")).await.status, StatusCode::OK);
    for store in [false, true] {
        b.request("PUT", "/api/me/storage", Some(ORIGIN), Some(json!({"store_on_server": store}))).await;
    }
    put(&mut b, vec![record("street", "other", 0, MIDDLE, Some(json!({})))]).await; // the counter is at 1 again
    let r = b.get(&format!("/api/work?since=1&epoch={epoch}")).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::GONE, Some("reset")));
    let fresh = b.get("/api/work?since=0").await.json();
    assert_ne!(fresh["epoch"].as_str().unwrap(), epoch, "turning storage on gave the account a new copy");
    assert_eq!(b.get("/api/work?since=0&epoch=nonsense").await.status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn a_body_that_does_not_open_is_flagged_and_the_rest_of_the_pull_is_served(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let mut b = storing(&app, "a").await;
    put(&mut b, vec![record("street", "one", 0, MIDDLE, Some(json!({"n": 1}))), record("street", "two", 0, MIDDLE, Some(json!({"n": 2})))]).await;
    sqlx::query("update work set body = (select body from work where key = 'one') where key = 'two'").execute(&pool).await.unwrap();
    let pulled = b.get("/api/work?since=0").await.json();
    let records = pulled["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let two = records.iter().find(|r| r["key"] == "two").unwrap();
    assert_eq!((two["error"].as_str(), two["body"].clone()), (Some("corrupt"), Value::Null));
    assert_eq!(records.iter().find(|r| r["key"] == "one").unwrap()["body"], json!({"n": 1}));
}

#[sqlx::test]
async fn the_work_routes_are_rate_limited_on_their_own(pool: PgPool) {
    let app = TestApp::with_work_rate(pool, RateLimit { period_secs: 3600, burst: 2 });
    let mut b = storing(&app, "a").await; // the call to /api/me/storage is not a work call
    assert_eq!(b.get("/api/work").await.status, StatusCode::OK);
    assert_eq!(b.get("/api/work").await.status, StatusCode::OK);
    assert_eq!(b.get("/api/work").await.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(b.get("/api/me").await.status, StatusCode::OK, "the rest of the API is not affected");
}

#[tokio::test]
async fn a_store_that_is_down_is_503_on_the_work_routes_too() {
    let mut b = Browser::new(TestApp::with_repo(Arc::new(DownRepo)));
    b.set_cookie("__Host-cityloom-session", &cityloom_server::session::new_token());
    let r = b.get("/api/work").await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::SERVICE_UNAVAILABLE, Some("unavailable")));
}
