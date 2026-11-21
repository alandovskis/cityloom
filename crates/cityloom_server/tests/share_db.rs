use cityloom_server::accounts::{self, Claims, Repo};
use cityloom_server::crypto::{LookupKey, MasterKeys};
use cityloom_server::db::PgRepo;
use cityloom_server::share::{self, Purged, ShareError, ShareRepo, Token};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn repo(pool: PgPool) -> PgRepo {
    PgRepo::new(pool, MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32]))], 1).unwrap(), LookupKey::new(zeroize::Zeroizing::new([3; 32])))
}

fn t0() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
}

async fn user(r: &PgRepo, sub: &str) -> Uuid {
    let claims = Claims {
        provider: "google".into(),
        provider_name: "Google".into(),
        sub: sub.into(),
        name: Some("Ada".into()),
        preferred_username: None,
        first_time_name: None,
        label: None,
    };
    accounts::sign_in(r, &claims).await.unwrap().0
}

const BODY: &[u8] = br#"{"kind":"cityloom-share","format_version":1,"project":{"name":"Plateau"}}"#;

#[sqlx::test]
async fn a_share_opens_with_its_token_only_and_comes_back_exactly(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    let token = Token::random();
    let created = r.create(ada, &token, BODY, t0()).await.unwrap();
    assert_eq!(created.expires_at, t0() + Duration::days(90));
    assert_eq!(r.open(&token, t0()).await.unwrap(), BODY);
    assert_eq!(r.open(&Token::random(), t0()).await.unwrap_err(), ShareError::NotFound);
    // The row holds the token's lookup hash, never the token, and the body sealed under a key only the token gives.
    let (lookup, body): (Vec<u8>, Vec<u8>) = sqlx::query_as("select lookup, body from shares").fetch_one(&pool).await.unwrap();
    assert_eq!(lookup, token.lookup());
    assert_ne!(lookup, token.text().as_bytes());
    assert_eq!(body.len(), BODY.len() + 1 + 12 + 16, "sealed: version, nonce, tag");
}

#[sqlx::test]
async fn a_body_moved_to_another_row_does_not_open(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    let (one, two) = (Token::random(), Token::random());
    r.create(ada, &one, BODY, t0()).await.unwrap();
    r.create(ada, &two, b"{\"kind\":\"cityloom-share\",\"format_version\":1}", t0()).await.unwrap();
    // Swap the lookups: each token now finds the other's row, whose body is sealed under the other key and AD.
    for (from, to) in [(one.lookup(), [0u8; 32]), (two.lookup(), one.lookup()), ([0u8; 32], two.lookup())] {
        sqlx::query("update shares set lookup = $2 where lookup = $1").bind(&from[..]).bind(&to[..]).execute(&pool).await.unwrap();
    }
    assert_eq!(r.open(&one, t0()).await.unwrap_err(), ShareError::Corrupt);
}

#[sqlx::test]
async fn an_expired_share_is_refused_and_its_body_dropped_and_the_purge_forgets_it_a_year_on(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    let (read, purged) = (Token::random(), Token::random());
    r.create(ada, &read, BODY, t0()).await.unwrap();
    r.create(ada, &purged, BODY, t0() + Duration::days(1)).await.unwrap();
    let at_90 = t0() + Duration::days(90);
    assert_eq!(r.open(&read, at_90).await.unwrap_err(), ShareError::Expired);
    let bodies: i64 = sqlx::query_scalar("select count(*) from shares where body is not null").fetch_one(&pool).await.unwrap();
    assert_eq!(bodies, 1, "the read dropped the expired body at once");
    assert_eq!(r.purge(at_90).await.unwrap(), Purged { bodies: 0, markers: 0 }, "the other is not expired yet");
    assert_eq!(r.purge(at_90 + Duration::days(1)).await.unwrap(), Purged { bodies: 1, markers: 0 });
    let bodies: i64 = sqlx::query_scalar("select count(*) from shares where body is not null").fetch_one(&pool).await.unwrap();
    assert_eq!(bodies, 0);
    assert_eq!(r.open(&purged, at_90 + Duration::days(2)).await.unwrap_err(), ShareError::Expired, "the marker still says expired");
    assert_eq!(r.purge(at_90 + Duration::days(share::MARKER_DAYS + 2)).await.unwrap(), Purged { bodies: 0, markers: 2 }, "a year past expiry the markers go");
    assert_eq!(r.open(&purged, at_90 + Duration::days(share::MARKER_DAYS + 2)).await.unwrap_err(), ShareError::NotFound);
}

#[sqlx::test]
async fn only_the_creator_deletes_and_the_body_goes_with_it(pool: PgPool) {
    let r = repo(pool.clone());
    let (ada, bea) = (user(&r, "a").await, user(&r, "b").await);
    let token = Token::random();
    r.create(ada, &token, BODY, t0()).await.unwrap();
    assert_eq!(r.delete(bea, &token, t0()).await.unwrap_err(), ShareError::NotYours);
    assert_eq!(r.open(&token, t0()).await.unwrap(), BODY);
    r.delete(ada, &token, t0()).await.unwrap();
    assert_eq!(r.open(&token, t0()).await.unwrap_err(), ShareError::Deleted);
    let body: Option<Vec<u8>> = sqlx::query_scalar("select body from shares").fetch_one(&pool).await.unwrap();
    assert_eq!(body, None);
    assert_eq!(r.delete(ada, &Token::random(), t0()).await.unwrap_err(), ShareError::NotFound);
}

#[sqlx::test]
async fn the_cap_counts_live_shares_and_a_gone_account_makes_none(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    for _ in 0..share::MAX_LIVE_SHARES {
        r.create(ada, &Token::random(), BODY, t0()).await.unwrap();
    }
    assert_eq!(r.create(ada, &Token::random(), BODY, t0()).await.unwrap_err(), ShareError::Full);
    assert!(r.create(ada, &Token::random(), BODY, t0() + Duration::days(90)).await.is_ok(), "expired ones do not count");
    r.delete_user(ada).await.unwrap();
    let rows: i64 = sqlx::query_scalar("select count(*) from shares").fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 0, "the account's shares went with it");
    assert_eq!(r.create(ada, &Token::random(), BODY, t0()).await.unwrap_err(), ShareError::Gone);
}

#[sqlx::test]
async fn deleting_a_share_that_has_expired_leaves_it_expired(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    let token = Token::random();
    r.create(ada, &token, BODY, t0()).await.unwrap();
    let later = t0() + Duration::days(91);
    r.delete(ada, &token, later).await.unwrap();
    assert_eq!(r.open(&token, later).await.unwrap_err(), ShareError::Expired, "it expired before it was deleted: so says its link");
    let (deleted, body): (bool, Option<Vec<u8>>) = sqlx::query_as("select deleted, body from shares").fetch_one(&pool).await.unwrap();
    assert_eq!((deleted, body), (false, None), "its body goes all the same");
    // One deleted before it expired stays deleted when deleted again.
    let other = Token::random();
    r.create(ada, &other, BODY, t0()).await.unwrap();
    r.delete(ada, &other, t0()).await.unwrap();
    r.delete(ada, &other, later).await.unwrap();
    assert_eq!(r.open(&other, later).await.unwrap_err(), ShareError::Deleted);
}

#[sqlx::test]
async fn opening_an_expired_share_writes_only_when_a_body_is_left_to_drop(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    let token = Token::random();
    r.create(ada, &token, BODY, t0()).await.unwrap();
    let later = t0() + Duration::days(90);
    // Each write of a row makes a new version of it, with a new `xmin`.
    let version = || async { sqlx::query_scalar::<_, String>("select xmin::text from shares").fetch_one(&pool).await.unwrap() };
    let before = version().await;
    assert_eq!(r.open(&token, later).await.unwrap_err(), ShareError::Expired);
    let dropped = version().await;
    assert_ne!(dropped, before, "the first read of it expired drops its body");
    assert_eq!(r.open(&token, later).await.unwrap_err(), ShareError::Expired);
    assert_eq!(version().await, dropped, "the next writes nothing: there is no body left");
    r.delete(ada, &token, later).await.unwrap();
    assert_eq!(r.open(&token, later).await.unwrap_err(), ShareError::Expired);
}

#[sqlx::test]
async fn a_live_share_whose_body_is_gone_is_corrupt(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = user(&r, "a").await;
    let token = Token::random();
    r.create(ada, &token, BODY, t0()).await.unwrap();
    sqlx::query("update shares set body = null").execute(&pool).await.unwrap();
    assert_eq!(r.open(&token, t0()).await.unwrap_err(), ShareError::Corrupt);
}

#[sqlx::test]
async fn an_account_makes_a_bounded_number_of_shares_a_day_deleted_ones_too(pool: PgPool) {
    let r = repo(pool.clone());
    let (ada, bea) = (user(&r, "a").await, user(&r, "b").await);
    for n in 0..share::MAX_SHARES_PER_DAY {
        let token = Token::random();
        r.create(ada, &token, BODY, t0() + Duration::minutes(n as i64)).await.unwrap();
        r.delete(ada, &token, t0() + Duration::minutes(n as i64)).await.unwrap();
    }
    let last = t0() + Duration::minutes(share::MAX_SHARES_PER_DAY as i64 - 1);
    assert_eq!(r.create(ada, &Token::random(), BODY, last).await.unwrap_err(), ShareError::TooMany, "deleting does not make room today");
    assert!(r.create(bea, &Token::random(), BODY, last).await.is_ok(), "the bound is per account");
    assert!(r.create(ada, &Token::random(), BODY, t0() + Duration::days(1)).await.is_ok(), "a day after the first, room for one");
}

#[sqlx::test]
async fn the_checks_before_a_share_is_kept_have_indexes_that_skip_the_markers(pool: PgPool) {
    let defs: Vec<(String, String)> =
        sqlx::query_as("select indexname::text, indexdef from pg_indexes where tablename = 'shares' order by indexname").fetch_all(&pool).await.unwrap();
    let def = |name: &str| defs.iter().find(|(n, _)| n == name).map(|(_, d)| d.clone()).unwrap_or_else(|| panic!("no {name}: {defs:?}"));
    assert!(def("shares_live").contains("(user_id, expires_at) WHERE (body IS NOT NULL)"), "{}", def("shares_live"));
    assert!(def("shares_user_created").contains("(user_id, created_at)"), "{}", def("shares_user_created"));
    assert!(defs.iter().all(|(n, _)| n != "shares_user"), "the account's rows are found by shares_user_created: {defs:?}");
}
