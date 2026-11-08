#![cfg(feature = "test-provider")]
mod support;

use axum::http::StatusCode;
use cityloom_server::accounts::Repo;
use cityloom_server::config::RateLimit;
use cityloom_server::fake_provider::FakeLogin;
use sqlx::PgPool;
use support::*;

const SESSION: &str = "__Host-cityloom-session";
const FLOW: &str = "__Host-cityloom-flow";

fn ada(sub: &str) -> FakeLogin {
    FakeLogin { sub: Some(sub.into()), name: Some("Ada".into()), email: Some(format!("{sub}@example.org")), ..Default::default() }
}

/// Starts a link flow as the signed-in browser and plays the provider as `login`; gives the callback's reply.
async fn link(b: &mut Browser, slug: &str, login: FakeLogin) -> Reply {
    let started = b.get(&format!("/auth/{slug}/start?link=1&return=/account.html")).await;
    assert_eq!(started.status, StatusCode::SEE_OTHER);
    play_provider(b, &started.location().unwrap(), &login).await
}

#[sqlx::test]
async fn a_first_sign_in_makes_the_account_a_session_and_goes_to_the_account_page(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let r = sign_in(&mut b, "fake", "/map.html", &ada("s1")).await;
    assert_eq!((r.status, r.location().as_deref()), (StatusCode::SEE_OTHER, Some("/account.html#account=new")));
    assert!(b.cookie(SESSION).is_some(), "signed in");
    assert!(b.cookie(FLOW).is_none(), "the flow cookie is cleared");
    assert!(app.repo.find_identity("fake", "s1").await.unwrap().is_some());
    let session_cookie = r.set_cookies().into_iter().find(|c| c.starts_with(SESSION)).unwrap();
    assert!(session_cookie.contains("HttpOnly") && session_cookie.contains("Secure") && session_cookie.contains("SameSite=Lax"), "{session_cookie}");
}

#[sqlx::test]
async fn a_returning_person_signs_in_to_the_same_account_and_goes_back_where_they_were(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &ada("s1")).await;
    let user = app.repo.find_identity("fake", "s1").await.unwrap().unwrap();
    let mut again = app.browser();
    let r = sign_in(&mut again, "fake", "/street.html?street=7", &ada("s1")).await;
    assert_eq!(r.location().as_deref(), Some("/street.html?street=7#account=signed-in"));
    assert_eq!(app.repo.find_identity("fake", "s1").await.unwrap(), Some(user));
}

#[sqlx::test]
async fn saying_no_at_the_provider_is_cancelled_and_makes_nothing(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let r = sign_in(&mut b, "fake", "/map.html", &FakeLogin { deny: true, sub: Some("s1".into()), ..Default::default() }).await;
    assert_eq!(r.location().as_deref(), Some("/map.html#account=cancelled"));
    assert!(b.cookie(SESSION).is_none() && b.cookie(FLOW).is_none());
    assert_eq!(app.repo.find_identity("fake", "s1").await.unwrap(), None);
}

#[sqlx::test]
async fn apples_cancel_code_is_cancelled_too(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let started = b.get("/auth/fake/start?return=/map.html").await;
    let location = started.location().unwrap();
    let state = location.split(['?', '&']).find_map(|pair| pair.strip_prefix("state=")).expect("the start sends a state").to_string();
    let r = b.get(&format!("/auth/fake/callback?error=user_cancelled_authorize&state={state}")).await;
    assert_eq!(r.location().as_deref(), Some("/map.html#account=cancelled"));
    assert!(b.cookie(SESSION).is_none() && b.cookie(FLOW).is_none());
}

#[sqlx::test]
async fn a_callback_with_the_wrong_state_fails_and_makes_nothing(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let started = b.get("/auth/fake/start?return=/map.html").await;
    let _ = started;
    let r = b.get("/auth/fake/callback?code=whatever&state=not-the-state").await;
    assert_eq!(r.location().as_deref(), Some("/#account=failed"));
    assert!(b.cookie(SESSION).is_none());
    // And no state at all.
    assert_eq!(b.get("/auth/fake/callback?code=whatever").await.location().as_deref(), Some("/#account=failed"));
}

#[sqlx::test]
async fn a_callback_with_no_flow_cookie_fails(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let r = b.get("/auth/fake/callback?code=x&state=y").await;
    assert_eq!(r.location().as_deref(), Some("/#account=failed"));
}

#[sqlx::test]
async fn returning_after_ten_minutes_fails_whether_or_not_the_browser_kept_the_cookie(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let started = b.get("/auth/fake/start?return=/map.html").await;
    app.clock.advance(time::Duration::seconds(601));
    let r = play_provider(&mut b, &started.location().unwrap(), &ada("s1")).await;
    assert_eq!(r.location().as_deref(), Some("/#account=failed"), "cookie still there, but older than ten minutes");
    assert_eq!(app.repo.find_identity("fake", "s1").await.unwrap(), None);
    // The browser dropped the cookie at its Max-Age.
    let mut b = app.browser();
    let started = b.get("/auth/fake/start?return=/map.html").await;
    b.drop_cookie(FLOW);
    let r = play_provider(&mut b, &started.location().unwrap(), &ada("s1")).await;
    assert_eq!(r.location().as_deref(), Some("/#account=failed"));
    assert_eq!(app.repo.find_identity("fake", "s1").await.unwrap(), None);
}

#[sqlx::test]
async fn two_tabs_signing_in_at_once_the_first_fails_and_the_second_completes(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser(); // one browser, two tabs: one cookie jar
    let tab_a = b.get("/auth/fake/start?return=/a.html").await;
    let flow_a = b.cookie(FLOW).cloned().unwrap();
    let tab_b = b.get("/auth/fake/start?return=/b.html").await;
    assert_ne!(b.cookie(FLOW).unwrap(), &flow_a, "the second start replaced the flow cookie");
    let a = play_provider(&mut b, &tab_a.location().unwrap(), &ada("tab-a")).await;
    assert_eq!(a.location().as_deref(), Some("/#account=failed"));
    assert!(b.cookie(FLOW).is_some(), "tab B's flow was not cleared by tab A's failure");
    assert_eq!(app.repo.find_identity("fake", "tab-a").await.unwrap(), None);
    let bb = play_provider(&mut b, &tab_b.location().unwrap(), &ada("tab-b")).await;
    assert_eq!(bb.location().as_deref(), Some("/account.html#account=new"));
    assert!(app.repo.find_identity("fake", "tab-b").await.unwrap().is_some());
}

#[sqlx::test]
async fn a_token_with_a_bad_nonce_audience_expiry_or_issue_time_fails_and_one_a_little_off_is_fine(pool: PgPool) {
    let app = TestApp::new(pool);
    let bad = [
        FakeLogin { nonce: Some("not-yours".into()), ..ada("bad-nonce") },
        FakeLogin { aud: Some("someone-else".into()), ..ada("bad-aud") },
        FakeLogin { exp_offset: -3700, ..ada("expired") },
        FakeLogin { iat_offset: 90, ..ada("from-the-future") },
    ];
    for login in bad {
        let sub = login.sub.clone().unwrap();
        let mut b = app.browser();
        let r = sign_in(&mut b, "fake", "/map.html", &login).await;
        assert_eq!(r.location().as_deref(), Some("/map.html#account=failed"), "{sub}");
        assert_eq!(app.repo.find_identity("fake", &sub).await.unwrap(), None, "{sub}");
    }
    let mut b = app.browser();
    let ok = FakeLogin { iat_offset: 45, exp_offset: -3640, ..ada("slightly-off") };
    assert_eq!(sign_in(&mut b, "fake", "/map.html", &ok).await.location().as_deref(), Some("/account.html#account=new"));
}

#[sqlx::test]
async fn apple_form_post_takes_a_name_from_the_first_sign_in_only(pool: PgPool) {
    let app = TestApp::new(pool);
    let first = FakeLogin {
        sub: Some("apple-1".into()),
        email: Some("relay@privaterelay.example".into()),
        first_time_user: Some(serde_json::json!({"name": {"firstName": "Ada", "lastName": "Lovelace"}})),
        ..Default::default()
    };
    let mut b = app.browser();
    let r = sign_in(&mut b, "fake-apple", "/map.html", &first).await;
    assert_eq!(r.location().as_deref(), Some("/account.html#account=new"));
    let user = app.repo.find_identity("fake-apple", "apple-1").await.unwrap().unwrap();
    assert_eq!(app.repo.user(user).await.unwrap().unwrap().display_name, "Ada Lovelace");
    assert_eq!(app.repo.identities(user).await.unwrap()[0].label.as_deref(), Some("relay@privaterelay.example"));
    // The second sign-in has no `user` field and no email at all.
    let second = FakeLogin { sub: Some("apple-1".into()), ..Default::default() };
    let mut b = app.browser();
    let r = sign_in(&mut b, "fake-apple", "/map.html", &second).await;
    assert_eq!(r.location().as_deref(), Some("/map.html#account=signed-in"));
    assert_eq!(app.repo.user(user).await.unwrap().unwrap().display_name, "Ada Lovelace", "the name is kept");
    // A first-ever sign-in with no name falls back to the provider's name; an unreadable `user` field is ignored.
    let nameless = FakeLogin { sub: Some("apple-2".into()), first_time_user: Some(serde_json::json!("garbage")), ..Default::default() };
    let mut b = app.browser();
    assert_eq!(sign_in(&mut b, "fake-apple", "/map.html", &nameless).await.location().as_deref(), Some("/account.html#account=new"));
    let user2 = app.repo.find_identity("fake-apple", "apple-2").await.unwrap().unwrap();
    assert_eq!(app.repo.user(user2).await.unwrap().unwrap().display_name, "Fake (form post)");
}

#[sqlx::test]
async fn linking_adds_an_identity_says_already_linked_and_refuses_a_conflict(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &ada("one")).await;
    let user = app.repo.find_identity("fake", "one").await.unwrap().unwrap();
    assert_eq!(link(&mut b, "fake-apple", ada("two")).await.location().as_deref(), Some("/account.html#account=linked"));
    assert_eq!(app.repo.identities(user).await.unwrap().len(), 2);
    assert_eq!(link(&mut b, "fake-apple", ada("two")).await.location().as_deref(), Some("/account.html#account=already-linked"));
    // Someone else's identity.
    let mut other = app.browser();
    sign_in(&mut other, "fake", "/", &ada("theirs")).await;
    assert_eq!(
        link(&mut b, "fake", ada("theirs")).await.location().as_deref(),
        Some("/account.html#account=conflict&provider=fake"),
        "the conflict names the provider so the message can"
    );
    assert_eq!(app.repo.identities(user).await.unwrap().len(), 2, "nothing changed");
    let theirs = app.repo.find_identity("fake", "theirs").await.unwrap().unwrap();
    assert_ne!(theirs, user);
    // A link flow does not sign anyone in as someone else.
    assert_eq!(app.repo.identities(theirs).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn a_provider_that_is_down_ends_in_failed_and_starts_no_flow(pool: PgPool) {
    let app = TestApp::with_http(pool, std::sync::Arc::new(Down));
    let mut b = app.browser();
    let r = b.get("/auth/fake/start?return=/map.html").await;
    assert_eq!((r.status, r.location().as_deref()), (StatusCode::SEE_OTHER, Some("/map.html#account=failed")));
    assert!(b.cookie(FLOW).is_none());
}

#[sqlx::test]
async fn a_code_the_provider_does_not_accept_ends_in_failed_and_makes_nothing(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let started = b.get("/auth/fake/start?return=/map.html").await;
    let provider_url = url::Url::parse(&started.location().unwrap()).unwrap();
    let state = provider_url.query_pairs().find(|(k, _)| k == "state").unwrap().1.into_owned();
    let r = b.get(&format!("/auth/fake/callback?code=never-issued&state={state}")).await;
    assert_eq!(r.location().as_deref(), Some("/map.html#account=failed"));
    assert!(b.cookie(SESSION).is_none() && b.cookie(FLOW).is_none());
}

#[sqlx::test]
async fn linking_needs_a_session(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    assert_eq!(b.get("/auth/fake/start?link=1").await.status, StatusCode::UNAUTHORIZED);
    assert!(b.cookie(FLOW).is_none());
}

#[sqlx::test]
async fn a_link_started_by_an_account_deleted_before_the_callback_fails_and_makes_nothing(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &ada("one")).await;
    let user = app.repo.find_identity("fake", "one").await.unwrap().unwrap();
    let started = b.get("/auth/fake-apple/start?link=1&return=/account.html").await;
    app.repo.delete_user(user).await.unwrap();
    let r = play_provider(&mut b, &started.location().unwrap(), &ada("two")).await;
    assert_eq!(r.location().as_deref(), Some("/account.html#account=failed"));
    assert_eq!(app.repo.find_identity("fake-apple", "two").await.unwrap(), None);
}

#[sqlx::test]
async fn an_unknown_provider_is_404_and_a_hostile_return_path_becomes_the_home_page(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    assert_eq!(b.get("/auth/nope/start").await.status, StatusCode::NOT_FOUND);
    assert_eq!(b.get("/auth/nope/callback?code=x&state=y").await.status, StatusCode::NOT_FOUND);
    sign_in(&mut b, "fake", "/", &ada("s1")).await; // makes the account
    for hostile in ["//evil.example", "/\\evil.example", "https://evil.example", "/%2f%2fevil.example"] {
        let mut b = app.browser();
        let r = sign_in(&mut b, "fake", hostile, &ada("s1")).await;
        assert_eq!(r.location().as_deref(), Some("/#account=signed-in"), "{hostile}");
    }
}

#[sqlx::test]
async fn signing_out_ends_the_session_and_needs_this_sites_origin(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &ada("s1")).await;
    let token = b.cookie(SESSION).cloned().unwrap();
    assert_eq!(b.request("POST", "/auth/signout", None, None).await.status, StatusCode::FORBIDDEN);
    assert_eq!(b.request("POST", "/auth/signout", Some("https://evil.example"), None).await.status, StatusCode::FORBIDDEN);
    assert!(b.cookie(SESSION).is_some(), "still signed in after the refused tries");
    let r = b.request("POST", "/auth/signout", Some("http://127.0.0.1:8137"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(b.cookie(SESSION).is_none());
    assert_eq!(app.repo.session(&cityloom_server::session::hash(&token)).await.unwrap(), None);
}

#[sqlx::test]
async fn signing_out_clears_a_link_flow_begun_before(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &ada("s1")).await;
    b.get("/auth/fake/start?link=1&return=/account.html").await;
    assert!(b.cookie(FLOW).is_some(), "a link flow is under way");
    let r = b.request("POST", "/auth/signout", Some("http://127.0.0.1:8137"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(b.cookie(SESSION).is_none() && b.cookie(FLOW).is_none(), "both cookies are gone");
}

#[tokio::test]
async fn signing_out_when_the_session_cannot_be_ended_still_clears_the_cookie_and_answers_204() {
    let mut b = Browser::new(TestApp::with_repo(std::sync::Arc::new(DownRepo)));
    b.set_cookie(SESSION, &cityloom_server::session::new_token());
    let r = b.request("POST", "/auth/signout", Some("http://127.0.0.1:8137"), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(b.cookie(SESSION).is_none());
}

#[sqlx::test]
async fn too_many_requests_to_auth_from_one_address_are_429(pool: PgPool) {
    let app = TestApp::with_rate(pool, RateLimit { period_secs: 60, burst: 3 });
    let mut b = app.browser();
    for _ in 0..3 {
        assert_eq!(b.get("/auth/fake/start").await.status, StatusCode::SEE_OTHER);
    }
    assert_eq!(b.get("/auth/fake/start").await.status, StatusCode::TOO_MANY_REQUESTS);
    b.forwarded_for = "198.51.100.9".into();
    assert_eq!(b.get("/auth/fake/start").await.status, StatusCode::SEE_OTHER, "another address has its own allowance");
}

#[sqlx::test]
async fn a_client_cannot_get_a_fresh_allowance_by_sending_its_own_forwarded_for(pool: PgPool) {
    let app = TestApp::with_rate(pool, RateLimit { period_secs: 60, burst: 2 });
    let mut b = app.browser();
    // The proxy appends the real address on the right; whatever the client wrote on the left is ignored.
    for spoofed in ["1.1.1.1", "2.2.2.2"] {
        b.forwarded_for = format!("{spoofed}, 203.0.113.7");
        assert_eq!(b.get("/auth/fake/start").await.status, StatusCode::SEE_OTHER);
    }
    b.forwarded_for = "3.3.3.3, 203.0.113.7".into();
    assert_eq!(b.get("/auth/fake/start").await.status, StatusCode::TOO_MANY_REQUESTS, "still the one address on the right");
}

#[sqlx::test]
async fn the_logs_say_method_path_status_provider_and_outcome_and_nothing_secret(pool: PgPool) {
    use std::cell::RefCell;
    use std::sync::{Arc, Mutex, Once};
    // One global subscriber for the whole test binary, writing to whichever sink the running test's thread holds
    // (a `sqlx::test` runs on one thread, so that is all of its log). A per-thread subscriber (`set_default`) is
    // not enough: tracing caches whether a call site is wanted, and a test of another thread that ran first, with
    // no subscriber, makes it "never", so this test would randomly miss lines.
    thread_local! { static SINK: RefCell<Option<Arc<Mutex<Vec<u8>>>>> = const { RefCell::new(None) }; }
    struct Routed;
    impl std::io::Write for Routed {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            SINK.with(|s| {
                if let Some(sink) = s.borrow().as_ref() {
                    sink.lock().unwrap().extend_from_slice(b);
                }
            });
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct RoutedWriters;
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RoutedWriters {
        type Writer = Routed;
        fn make_writer(&'a self) -> Routed {
            Routed
        }
    }
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| tracing::subscriber::set_global_default(tracing_subscriber::fmt().with_writer(RoutedWriters).with_ansi(false).finish()).unwrap());
    let sink = Arc::new(Mutex::new(Vec::new()));
    SINK.with(|s| *s.borrow_mut() = Some(sink.clone()));
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let login = FakeLogin {
        sub: Some("sub-very-secret-123".into()),
        name: Some("Zelda Hidden".into()),
        email: Some("zelda.hidden@example.org".into()),
        ..Default::default()
    };
    sign_in(&mut b, "fake", "/map.html", &login).await;
    b.get("/auth/fake/callback?code=CODE-SECRET-999&state=STATE-SECRET-999").await;
    let session = b.cookie(SESSION).cloned().unwrap_or_default();
    let logs = String::from_utf8(sink.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("/auth/fake/callback") && logs.contains("fake") && logs.contains("new"), "{logs}");
    for secret in [
        "sub-very-secret-123",
        "Zelda Hidden",
        "zelda.hidden",
        "CODE-SECRET-999",
        "STATE-SECRET-999",
        "code=",
        "state=",
        "nonce",
        "verifier",
        "cityloom-session",
        "cityloom-flow",
        session.as_str(),
    ] {
        assert!(secret.is_empty() || !logs.contains(secret), "the log holds {secret:?}:\n{logs}");
    }
}

#[sqlx::test]
async fn paths_under_auth_and_api_that_no_route_answers_are_never_the_static_files(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    for path in ["/auth", "/auth/", "/auth/x/y/z", "/auth/fake/start/more", "/auth/fake"] {
        let r = b.get(path).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(r.json()["error"], "not-found", "{path}");
    }
    for path in ["/api", "/api/", "/api/x/y"] {
        let r = b.get(path).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(r.json()["error"], "not-found", "{path}");
    }
    assert_eq!(b.get("/api/me").await.status, StatusCode::UNAUTHORIZED, "a real route");
    // A path that only starts like them is the site's.
    assert_ne!(b.get("/authors.html").await.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(b.get("/auth/signout").await.status, StatusCode::METHOD_NOT_ALLOWED, "a GET is not a sign-out");
}

#[sqlx::test]
async fn a_flow_cookie_that_cannot_be_used_is_cleared_and_one_of_another_provider_is_left_alone(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    let started = b.get("/auth/fake/start?return=/map.html").await;
    app.clock.advance(time::Duration::seconds(601));
    let r = play_provider(&mut b, &started.location().unwrap(), &ada("s1")).await;
    assert_eq!(r.location().as_deref(), Some("/#account=failed"));
    assert!(b.cookie(FLOW).is_none(), "an expired flow is unusable, so its cookie goes");
    b.set_cookie(FLOW, "damaged");
    assert_eq!(b.get("/auth/fake/callback?code=x&state=y").await.location().as_deref(), Some("/#account=failed"));
    assert!(b.cookie(FLOW).is_none(), "so does one that cannot be read");

    let started = b.get("/auth/fake/start?return=/map.html").await;
    let flow = b.cookie(FLOW).cloned().unwrap();
    let state = url::Url::parse(&started.location().unwrap()).unwrap().query_pairs().find(|(k, _)| k == "state").unwrap().1.into_owned();
    let r = b.get(&format!("/auth/fake-apple/callback?code=x&state={state}")).await;
    assert_eq!(r.location().as_deref(), Some("/#account=failed"), "the flow is the other provider's");
    assert_eq!(b.cookie(FLOW), Some(&flow), "and it is still live");
}

#[sqlx::test]
async fn a_return_path_with_other_than_ascii_is_percent_encoded_in_the_location_and_never_panics(pool: PgPool) {
    let app = TestApp::new(pool);
    let mut b = app.browser();
    sign_in(&mut b, "fake", "/", &ada("s1")).await;
    let mut again = app.browser();
    let r = sign_in(&mut again, "fake", "/caf\u{e9}.html?q=\u{1f5fa}", &ada("s1")).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    assert_eq!(r.location().as_deref(), Some("/caf%C3%A9.html?q=%F0%9F%97%BA#account=signed-in"));
}
