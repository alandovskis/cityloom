use cityloom_server::accounts::{self, Claims, IdentityInput, Outcome, Removal, Repo, RepoError};
use cityloom_server::crypto::{LookupKey, MasterKeys};
use cityloom_server::db::PgRepo;
use sqlx::PgPool;
use time::OffsetDateTime;

fn repo(pool: PgPool) -> PgRepo {
    PgRepo::new(
        pool,
        MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32])), (2, zeroize::Zeroizing::new([2; 32]))], 2).unwrap(),
        LookupKey::new(zeroize::Zeroizing::new([3; 32])),
    )
}

fn claims(provider: &str, sub: &str) -> Claims {
    Claims {
        provider: provider.into(),
        provider_name: provider.to_uppercase(),
        sub: sub.into(),
        name: Some("Ada Lovelace".into()),
        preferred_username: None,
        first_time_name: None,
        label: Some(format!("{sub}@example.org")),
    }
}

#[sqlx::test]
async fn a_user_and_identities_round_trip_through_the_sealed_columns(pool: PgPool) {
    let r = repo(pool);
    let (ada, outcome) = accounts::sign_in(&r, &claims("google", "g1")).await.unwrap();
    assert_eq!(outcome, Outcome::New);
    let u = r.user(ada).await.unwrap().unwrap();
    assert_eq!((u.display_name.as_str(), u.store_on_server), ("Ada Lovelace", None));
    assert_eq!(accounts::link(&r, ada, &claims("apple", "a1")).await.unwrap(), Outcome::Linked);
    let ids = r.identities(ada).await.unwrap();
    assert_eq!(
        ids.iter().map(|i| (i.provider.as_str(), i.label.as_deref())).collect::<Vec<_>>(),
        [("google", Some("g1@example.org")), ("apple", Some("a1@example.org"))]
    );
    assert_eq!(accounts::sign_in(&r, &claims("apple", "a1")).await.unwrap(), (ada, Outcome::SignedIn));
}

#[sqlx::test]
async fn a_label_may_be_absent(pool: PgPool) {
    let r = repo(pool);
    let mut c = claims("apple", "a1");
    c.label = None;
    let (ada, _) = accounts::sign_in(&r, &c).await.unwrap();
    assert_eq!(r.identities(ada).await.unwrap()[0].label, None);
}

#[sqlx::test]
async fn an_identity_made_twice_is_a_duplicate_and_leaves_no_second_user(pool: PgPool) {
    let r = repo(pool.clone());
    let i = IdentityInput { provider: "google".into(), sub: "g1".into(), label: None };
    r.create_user("A", &i).await.unwrap();
    assert_eq!(r.create_user("B", &i).await.unwrap_err(), RepoError::Duplicate);
    let users: i64 = sqlx::query_scalar("select count(*) from users").fetch_one(&pool).await.unwrap();
    assert_eq!(users, 1);
}

#[sqlx::test]
async fn the_last_identity_is_never_removed_and_nobody_removes_another_users(pool: PgPool) {
    let r = repo(pool);
    let (ada, _) = accounts::sign_in(&r, &claims("google", "g1")).await.unwrap();
    let (bob, _) = accounts::sign_in(&r, &claims("apple", "a1")).await.unwrap();
    let first = r.identities(ada).await.unwrap()[0].id.clone();
    assert_eq!(r.remove_identity(ada, &first).await.unwrap(), Removal::Last);
    accounts::link(&r, ada, &claims("microsoft", "m1")).await.unwrap();
    let bobs = r.identities(bob).await.unwrap()[0].id.clone();
    assert_eq!(r.remove_identity(ada, &bobs).await.unwrap(), Removal::NotFound);
    assert_eq!(r.remove_identity(ada, "zz").await.unwrap(), Removal::NotFound);
    assert_eq!(r.remove_identity(ada, &first).await.unwrap(), Removal::Removed);
    assert_eq!(r.find_identity("google", "g1").await.unwrap(), None);
}

#[sqlx::test]
async fn the_storage_choice_is_kept_and_a_missing_user_is_gone(pool: PgPool) {
    let r = repo(pool);
    let (ada, _) = accounts::sign_in(&r, &claims("google", "g1")).await.unwrap();
    r.set_storage(ada, true).await.unwrap();
    assert_eq!(r.user(ada).await.unwrap().unwrap().store_on_server, Some(true));
    r.set_storage(ada, false).await.unwrap();
    assert_eq!(r.user(ada).await.unwrap().unwrap().store_on_server, Some(false));
    assert_eq!(r.set_storage(uuid::Uuid::new_v4(), true).await.unwrap_err(), RepoError::Gone);
    assert_eq!(r.add_identity(uuid::Uuid::new_v4(), &IdentityInput { provider: "p".into(), sub: "s".into(), label: None }).await.unwrap_err(), RepoError::Gone);
}

#[sqlx::test]
async fn deleting_a_user_leaves_no_row_in_any_table(pool: PgPool) {
    let r = repo(pool.clone());
    let (ada, _) = accounts::sign_in(&r, &claims("google", "g1")).await.unwrap();
    accounts::link(&r, ada, &claims("apple", "a1")).await.unwrap();
    let now = OffsetDateTime::now_utc();
    r.create_session([1; 32], ada, now, now + time::Duration::days(30)).await.unwrap();
    let (bob, _) = accounts::sign_in(&r, &claims("google", "g2")).await.unwrap();
    accounts::delete(&r, ada).await.unwrap();
    for table in ["users", "identities", "sessions"] {
        let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "select count(*) from {table} where {}",
            if table == "users" { "id = $1" } else { "user_id = $1" }
        )))
        .bind(ada)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    assert!(r.user(bob).await.unwrap().is_some(), "another user is untouched");
}

#[sqlx::test]
async fn sessions_are_kept_touched_and_expired_ones_deleted(pool: PgPool) {
    let r = repo(pool);
    let (ada, _) = accounts::sign_in(&r, &claims("google", "g1")).await.unwrap();
    let t0 = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    r.create_session([1; 32], ada, t0, t0 + time::Duration::days(30)).await.unwrap();
    r.create_session([2; 32], ada, t0, t0 + time::Duration::days(1)).await.unwrap();
    let later = t0 + time::Duration::days(2);
    assert_eq!(r.delete_expired_sessions(later).await.unwrap(), 1);
    r.touch_session(&[1; 32], later, later + time::Duration::days(30)).await.unwrap();
    let s = r.session(&[1; 32]).await.unwrap().unwrap();
    assert_eq!((s.user_id, s.last_seen_at), (ada, later));
    r.delete_session(&[1; 32]).await.unwrap();
    assert_eq!(r.session(&[1; 32]).await.unwrap(), None);
}

#[sqlx::test]
async fn a_value_moved_to_another_row_or_read_with_other_keys_is_corrupt_not_plaintext(pool: PgPool) {
    let r = repo(pool.clone());
    let (ada, _) = accounts::sign_in(&r, &claims("google", "g1")).await.unwrap();
    let mut other = claims("google", "g2");
    other.name = Some("Bob".into());
    let (bob, _) = accounts::sign_in(&r, &other).await.unwrap();
    sqlx::query("update users set display_name = (select display_name from users where id = $2) where id = $1")
        .bind(ada)
        .bind(bob)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(r.user(ada).await.unwrap_err(), RepoError::Corrupt);
    let strangers = PgRepo::new(pool, MasterKeys::new(&[(2, zeroize::Zeroizing::new([9; 32]))], 2).unwrap(), LookupKey::new(zeroize::Zeroizing::new([3; 32])));
    assert_eq!(strangers.user(bob).await.unwrap_err(), RepoError::Corrupt);
}

#[sqlx::test]
async fn the_background_migration_marks_a_reachable_database_ready(pool: PgPool) {
    let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let task = cityloom_server::db::spawn_migrations(pool, ready.clone());
    tokio::time::timeout(std::time::Duration::from_secs(10), task).await.expect("it finishes once the database answers").unwrap();
    assert!(ready.load(std::sync::atomic::Ordering::Acquire));
}

#[tokio::test]
async fn the_background_migration_keeps_trying_while_the_database_is_down() {
    let pool = cityloom_server::db::connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none").unwrap();
    let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let task = cityloom_server::db::spawn_migrations_every(pool, ready.clone(), std::time::Duration::from_millis(10), std::time::Duration::from_millis(40));
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(!ready.load(std::sync::atomic::Ordering::Acquire) && !task.is_finished(), "still retrying, not given up");
    task.abort();
}
