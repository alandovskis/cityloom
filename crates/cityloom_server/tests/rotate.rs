#![cfg(feature = "test-provider")]
mod support;

use std::sync::Arc;

use cityloom_server::accounts::{self, Claims, Repo};
use cityloom_server::config::RateLimit;
use cityloom_server::crypto::{LookupKey, MasterKeys};
use cityloom_server::db::PgRepo;
use cityloom_server::db::rotate::Rehashed;
use cityloom_server::fake_provider::FakeLogin;
use sqlx::PgPool;
use support::*;

fn claims(provider: &str, sub: &str) -> Claims {
    Claims {
        provider: provider.into(),
        provider_name: provider.into(),
        sub: sub.into(),
        name: Some("Ada".into()),
        preferred_username: None,
        first_time_name: None,
        label: Some(format!("{sub}@example.org")),
    }
}

fn repo(pool: &PgPool, masters: &[(u8, [u8; 32])], current: u8, lookup: u8) -> PgRepo {
    let masters: Vec<_> = masters.iter().map(|(id, key)| (*id, zeroize::Zeroizing::new(*key))).collect();
    PgRepo::new(pool.clone(), MasterKeys::new(&masters, current).unwrap(), LookupKey::new(zeroize::Zeroizing::new([lookup; 32])))
}

#[sqlx::test]
async fn rewrap_moves_every_data_key_under_the_current_master_key_so_the_old_one_can_go(pool: PgPool) {
    let v1 = repo(&pool, &[(1, [1; 32])], 1, 3);
    let (ada, _) = accounts::sign_in(&v1, &claims("google", "g1")).await.unwrap();
    let (bob, _) = accounts::sign_in(&v1, &claims("google", "g2")).await.unwrap();
    let both = repo(&pool, &[(1, [1; 32]), (2, [2; 32])], 2, 3);
    assert!(both.user(ada).await.unwrap().is_some(), "the old key still opens what it sealed");
    assert_eq!(both.rewrap().await.unwrap(), 2);
    let ids: Vec<i16> = sqlx::query_scalar("select master_key_id from users").fetch_all(&pool).await.unwrap();
    assert_eq!(ids, [2, 2]);
    let only_new = repo(&pool, &[(2, [2; 32])], 2, 3);
    assert_eq!(only_new.user(ada).await.unwrap().unwrap().display_name, "Ada");
    assert_eq!(only_new.identities(bob).await.unwrap().len(), 1);
    assert!(v1.user(ada).await.is_err(), "the old key alone no longer opens it");
    assert_eq!(both.rewrap().await.unwrap(), 0, "nothing left to do");
}

#[sqlx::test]
async fn rehash_recomputes_every_lookup_and_the_data_stays_readable_and_bound(pool: PgPool) {
    let old = repo(&pool, &[(1, [1; 32])], 1, 3);
    let (ada, _) = accounts::sign_in(&old, &claims("google", "g1")).await.unwrap();
    accounts::link(&old, ada, &claims("apple", "a1")).await.unwrap();
    let (bob, _) = accounts::sign_in(&old, &claims("google", "g2")).await.unwrap();
    let new = repo(&pool, &[(1, [1; 32])], 1, 4);
    assert_eq!(new.find_identity("google", "g1").await.unwrap(), None, "the new key does not find the old hashes");
    assert_eq!(
        new.rehash(&LookupKey::new(zeroize::Zeroizing::new([9; 32]))).await.unwrap_err(),
        accounts::RepoError::Corrupt,
        "a wrong old key changes nothing"
    );
    assert_eq!(new.find_identity("google", "g1").await.unwrap(), None);
    assert_eq!(old.find_identity("google", "g1").await.unwrap(), Some(ada), "still the old hashes");
    assert_eq!(new.rehash(&LookupKey::new(zeroize::Zeroizing::new([3; 32]))).await.unwrap(), Rehashed { checked: 3, rehashed: 3, already_new: 0 });
    assert_eq!(new.find_identity("google", "g1").await.unwrap(), Some(ada));
    assert_eq!(new.find_identity("apple", "a1").await.unwrap(), Some(ada));
    assert_eq!(new.find_identity("google", "g2").await.unwrap(), Some(bob));
    assert_eq!(old.find_identity("google", "g1").await.unwrap(), None);
    let ids = new.identities(ada).await.unwrap();
    assert_eq!(
        ids.iter().map(|i| (i.provider.as_str(), i.label.as_deref())).collect::<Vec<_>>(),
        [("google", Some("g1@example.org")), ("apple", Some("a1@example.org"))]
    );
    assert_eq!(
        new.rehash(&LookupKey::new(zeroize::Zeroizing::new([3; 32]))).await.unwrap(),
        Rehashed { checked: 3, rehashed: 0, already_new: 3 },
        "running it twice finds nothing left to do"
    );
    assert_eq!(new.rehash(&LookupKey::new(zeroize::Zeroizing::new([4; 32]))).await.unwrap(), Rehashed { checked: 3, rehashed: 0, already_new: 3 });
}

#[sqlx::test]
async fn rehash_finishes_a_mixed_table_where_some_rows_are_already_on_the_new_hash(pool: PgPool) {
    let old = repo(&pool, &[(1, [1; 32])], 1, 3);
    let new = repo(&pool, &[(1, [1; 32])], 1, 4);
    let (ada, _) = accounts::sign_in(&old, &claims("google", "g1")).await.unwrap();
    accounts::link(&old, ada, &claims("apple", "a1")).await.unwrap();
    let (bob, _) = accounts::sign_in(&new, &claims("google", "g2")).await.unwrap();
    assert_eq!(
        new.rehash(&LookupKey::new(zeroize::Zeroizing::new([9; 32]))).await.unwrap_err(),
        accounts::RepoError::Corrupt,
        "a wrong old key still matches neither"
    );
    assert_eq!(
        new.rehash(&LookupKey::new(zeroize::Zeroizing::new([3; 32]))).await.unwrap(),
        Rehashed { checked: 3, rehashed: 2, already_new: 1 },
        "only the rows still on the old hash change"
    );
    assert_eq!(new.find_identity("google", "g1").await.unwrap(), Some(ada));
    assert_eq!(new.find_identity("apple", "a1").await.unwrap(), Some(ada));
    assert_eq!(new.find_identity("google", "g2").await.unwrap(), Some(bob));
    assert_eq!(new.identities(ada).await.unwrap().len(), 2);
}

#[sqlx::test]
async fn signing_in_still_works_after_a_rewrap_and_a_rehash(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let mut b = app.browser();
    let login = FakeLogin { sub: Some("s1".into()), name: Some("Ada".into()), ..Default::default() };
    sign_in(&mut b, "fake", "/", &login).await;
    let user = app.repo.find_identity("fake", "s1").await.unwrap().unwrap();
    let rotated = Arc::new(repo(&pool, &[(1, [1; 32]), (2, [2; 32])], 2, 4));
    rotated.rewrap().await.unwrap();
    rotated.rehash(&LookupKey::new(zeroize::Zeroizing::new([3; 32]))).await.unwrap();
    let router = account_router(
        rotated.clone(),
        rotated.clone(),
        None,
        app.clock.clone(),
        app.fake.clone(),
        RateLimit { period_secs: 1, burst: 1000 },
        RateLimit { period_secs: 1, burst: 1000 },
    );
    let mut again = Browser::new(router);
    let r = sign_in(&mut again, "fake", "/map.html", &login).await;
    assert_eq!(r.location().as_deref(), Some("/map.html#account=signed-in"));
    assert_eq!(rotated.find_identity("fake", "s1").await.unwrap(), Some(user), "the same account");
    // And the session made before the rotation still reads.
    let old_session = b.cookie("__Host-cityloom-session").cloned().unwrap();
    let mut old_tab = Browser::new(account_router(
        rotated.clone(),
        rotated,
        None,
        app.clock.clone(),
        app.fake.clone(),
        RateLimit { period_secs: 1, burst: 1000 },
        RateLimit { period_secs: 1, burst: 1000 },
    ));
    old_tab.set_cookie("__Host-cityloom-session", &old_session);
    assert_eq!(old_tab.get("/api/me").await.status, axum::http::StatusCode::OK);
}
