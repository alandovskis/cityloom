use cityloom_server::accounts::{self, Claims, Repo};
use cityloom_server::crypto::{LookupKey, MasterKeys};
use cityloom_server::db::PgRepo;
use cityloom_server::work::{self, Ask, Id, Kind, Limit, Put, Verdict, WorkError, WorkRepo};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

fn repo(pool: PgPool) -> PgRepo {
    PgRepo::new(
        pool,
        MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32])), (2, zeroize::Zeroizing::new([2; 32]))], 1).unwrap(),
        LookupKey::new(zeroize::Zeroizing::new([3; 32])),
    )
}

fn claims(sub: &str) -> Claims {
    Claims {
        provider: "google".into(),
        provider_name: "Google".into(),
        sub: sub.into(),
        name: Some("Ada".into()),
        preferred_username: None,
        first_time_name: None,
        label: None,
    }
}

fn t(secs: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_800_000_000 + secs).unwrap()
}

async fn storing(r: &PgRepo, sub: &str) -> Uuid {
    let (user, _) = accounts::sign_in(r, &claims(sub)).await.unwrap();
    r.set_storage(user, true).await.unwrap();
    user
}

fn put(kind: Kind, key: &str, base_rev: i64, secs: i64, body: &str) -> Put {
    Put { id: Id { kind, key: key.into() }, base_rev, edited_at: t(secs), deleted: false, body: Some(body.as_bytes().to_vec()) }
}

fn gone(kind: Kind, key: &str, base_rev: i64, secs: i64) -> Put {
    Put { id: Id { kind, key: key.into() }, base_rev, edited_at: t(secs), deleted: true, body: None }
}

fn text(body: &Option<Vec<u8>>) -> String {
    String::from_utf8(body.clone().expect("a body")).unwrap()
}

/// The version of the user's row: it changes when the row is written, not when it is only locked or read.
async fn row_version(pool: &PgPool, user: Uuid) -> String {
    sqlx::query_scalar("select xmin::text from users where id = $1").bind(user).fetch_one(pool).await.unwrap()
}

#[sqlx::test]
async fn a_record_pushed_comes_back_pulled_with_its_body_exactly(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    let body = r#"{"street":{"row_mm":15000},"future_field":{"x":[1,2]}}"#;
    let pushed = r.push(user, vec![put(Kind::Street, "w1~n2~n3", 0, 10, body)], t(100)).await.unwrap();
    assert_eq!((pushed.rev, pushed.items[0].verdict, pushed.items[0].rev), (1, Verdict::Accepted, 1));
    let pulled = r.pull(user, Ask::first(0), t(100)).await.unwrap();
    assert_eq!((pulled.rev, pulled.more, pulled.records.len()), (1, false, 1));
    let rec = &pulled.records[0];
    assert_eq!((rec.id.kind, rec.id.key.as_str(), rec.rev, rec.deleted, rec.unreadable), (Kind::Street, "w1~n2~n3", 1, false, false));
    assert_eq!(rec.edited_at, t(10));
    assert_eq!(text(&rec.body), body, "a newer build's fields pass through, byte for byte");
}

#[sqlx::test]
async fn bodies_are_sealed_at_rest_and_the_overhead_is_what_the_limits_assume(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Rule, "r1", 0, 1, r#"{"name":"Zone Verte"}"#)], t(100)).await.unwrap();
    let (stored, size): (Vec<u8>, i32) =
        sqlx::query_as("select body, octet_length(body) from work where user_id = $1").bind(user).fetch_one(&pool).await.unwrap();
    assert!(!String::from_utf8_lossy(&stored).contains("Zone Verte"));
    assert_eq!(size as usize, r#"{"name":"Zone Verte"}"#.len() + work::SEAL_OVERHEAD);
}

#[sqlx::test]
async fn a_body_moved_to_another_place_or_account_does_not_open_and_the_rest_of_the_pull_is_served(pool: PgPool) {
    let r = repo(pool.clone());
    let ada = storing(&r, "a").await;
    let bob = storing(&r, "b").await;
    r.push(
        ada,
        vec![put(Kind::Street, "k1", 0, 1, "{\"n\":1}"), put(Kind::Street, "k2", 0, 1, "{\"n\":2}"), put(Kind::Junction, "k3", 0, 1, "{\"n\":3}")],
        t(10),
    )
    .await
    .unwrap();
    r.push(bob, vec![put(Kind::Street, "k1", 0, 1, "{\"bob\":true}")], t(10)).await.unwrap();
    // k1's body is put under k2 (another place), k3 gets its kind changed (another kind), and Bob's body goes to Ada's k1
    sqlx::query("update work set body = (select body from work where user_id = $1 and key = 'k1') where user_id = $1 and key = 'k2'")
        .bind(ada)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("update work set kind = 'street' where user_id = $1 and key = 'k3'").bind(ada).execute(&pool).await.unwrap();
    sqlx::query("update work set body = (select body from work where user_id = $2) where user_id = $1 and key = 'k1'")
        .bind(ada)
        .bind(bob)
        .execute(&pool)
        .await
        .unwrap();
    let pulled = r.pull(ada, Ask::first(0), t(10)).await.unwrap();
    assert_eq!(pulled.records.len(), 3, "the rest of the pull is served");
    assert!(pulled.records.iter().all(|rec| rec.unreadable && rec.body.is_none()), "each moved body is refused");
}

#[sqlx::test]
async fn two_batches_at_once_for_one_place_accept_one_and_supersede_the_other(pool: PgPool) {
    let r = std::sync::Arc::new(repo(pool));
    let user = storing(&r, "a").await;
    let (a, b) = (r.clone(), r.clone());
    let (x, y) = tokio::join!(async move { a.push(user, vec![put(Kind::Street, "k", 0, 5, "{\"from\":\"a\"}")], t(100)).await.unwrap() }, async move {
        b.push(user, vec![put(Kind::Street, "k", 0, 5, "{\"from\":\"b\"}")], t(100)).await.unwrap()
    },);
    let mut verdicts = [x.items[0].verdict, y.items[0].verdict];
    verdicts.sort_by_key(|v| *v == Verdict::Superseded);
    assert_eq!(verdicts, [Verdict::Accepted, Verdict::Superseded]);
    let superseded = if x.items[0].verdict == Verdict::Superseded { &x } else { &y };
    assert_eq!(superseded.items[0].rev, 1);
    assert!(superseded.items[0].record.is_some(), "the loser is told what won");
    assert_eq!(r.pull(user, Ask::first(0), t(100)).await.unwrap().records.len(), 1);
}

#[sqlx::test]
async fn a_batch_that_breaks_a_limit_leaves_nothing_and_the_revision_where_it_was(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Street, "ok", 0, 1, "{}")], t(10)).await.unwrap();
    let big = format!("{{\"x\":\"{}\"}}", "y".repeat(work::MAX_RECORD_BYTES));
    let err = r.push(user, vec![put(Kind::Street, "a", 0, 2, "{}"), put(Kind::Street, "b", 0, 2, &big)], t(10)).await.unwrap_err();
    assert_eq!(err, WorkError::TooLarge(Limit::Record));
    let pulled = r.pull(user, Ask::first(0), t(10)).await.unwrap();
    assert_eq!((pulled.rev, pulled.records.len()), (1, 1));
}

#[sqlx::test]
async fn the_account_limits_are_kept_over_many_batches(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    for batch in 0..50 {
        let puts: Vec<Put> = (0..100).map(|i| put(Kind::Street, &format!("k{batch}-{i}"), 0, 1, "{}")).collect();
        r.push(user, puts, t(10)).await.unwrap();
    }
    assert_eq!(r.push(user, vec![put(Kind::Street, "one-more", 0, 1, "{}")], t(10)).await.unwrap_err(), WorkError::TooLarge(Limit::Records));
    let n: i64 = sqlx::query_scalar("select count(*) from work where user_id = $1").bind(user).fetch_one(&pool).await.unwrap();
    assert_eq!(n, 5_000);
    // 2 MB of sealed bodies, on another account: 10 records of 60,000 bytes per batch, the fourth batch is too much
    let other = storing(&r, "b").await;
    let sixty = "z".repeat(60_000 - 8);
    for batch in 0..3 {
        let puts: Vec<Put> = (0..10).map(|i| put(Kind::Rule, &format!("r{batch}-{i}"), 0, 1, &format!("{{\"x\":\"{sixty}\"}}"))).collect();
        r.push(other, puts, t(10)).await.unwrap();
    }
    let puts: Vec<Put> = (0..10).map(|i| put(Kind::Rule, &format!("r3-{i}"), 0, 1, &format!("{{\"x\":\"{sixty}\"}}"))).collect();
    assert_eq!(r.push(other, puts, t(10)).await.unwrap_err(), WorkError::TooLarge(Limit::Bytes));
}

#[sqlx::test]
async fn without_storage_on_every_call_is_refused_and_a_push_after_storage_off_writes_nothing(pool: PgPool) {
    let r = repo(pool.clone());
    let (user, _) = accounts::sign_in(&r, &claims("a")).await.unwrap();
    assert_eq!(r.pull(user, Ask::first(0), t(1)).await.unwrap_err(), WorkError::StorageOff, "nothing chosen yet");
    r.set_storage(user, true).await.unwrap();
    r.push(user, vec![put(Kind::Street, "k", 0, 1, "{}")], t(10)).await.unwrap();
    r.set_storage(user, false).await.unwrap();
    // an offline device sends what it queued
    assert_eq!(r.push(user, vec![put(Kind::Street, "k", 1, 2, "{}")], t(20)).await.unwrap_err(), WorkError::StorageOff);
    assert_eq!(r.clear(user).await.unwrap_err(), WorkError::StorageOff);
    let n: i64 = sqlx::query_scalar("select count(*) from work where user_id = $1").bind(user).fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0, "the copy was deleted with the choice and not recreated");
}

#[sqlx::test]
async fn turning_storage_off_deletes_the_copy_and_resets_the_revision_and_on_again_starts_clean(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Street, "k", 0, 1, "{}"), put(Kind::Area, "current", 0, 1, "{\"query\":\"area=1,2\"}")], t(10)).await.unwrap();
    r.set_storage(user, false).await.unwrap();
    let (rev, floor): (i64, i64) = sqlx::query_as("select work_rev, work_floor from users where id = $1").bind(user).fetch_one(&pool).await.unwrap();
    assert_eq!((rev, floor), (0, 0));
    r.set_storage(user, true).await.unwrap();
    assert_eq!(r.pull(user, Ask::first(0), t(10)).await.unwrap().records.len(), 0);
    let again = r.push(user, vec![put(Kind::Street, "k", 0, 5, "{}")], t(10)).await.unwrap();
    assert_eq!(again.rev, 1);
}

#[sqlx::test]
async fn a_since_above_the_revision_is_resync_not_an_empty_page(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Street, "k", 0, 1, "{}"), put(Kind::Street, "l", 0, 1, "{}")], t(10)).await.unwrap(); // revision 2
    r.set_storage(user, false).await.unwrap();
    r.set_storage(user, true).await.unwrap(); // the counter is back at 0
    r.push(user, vec![put(Kind::Street, "k", 0, 5, "{}")], t(10)).await.unwrap(); // revision 1
    // a device that kept the old counter (it was at 2) is told to start over
    assert_eq!(r.pull(user, Ask::first(2), t(10)).await.unwrap_err(), WorkError::Resync);
    assert_eq!(r.pull(user, Ask::first(1), t(10)).await.unwrap().records.len(), 0, "a device at the current revision is fine");
}

#[sqlx::test]
async fn a_deletion_is_a_tombstone_other_devices_pull_and_pull_from_zero_leaves_out(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Rule, "r1", 0, 1, "{}")], t(10)).await.unwrap();
    r.push(user, vec![gone(Kind::Rule, "r1", 1, 2)], t(10)).await.unwrap();
    let since_one = r.pull(user, Ask::first(1), t(10)).await.unwrap();
    assert_eq!((since_one.records.len(), since_one.records[0].deleted, since_one.records[0].rev, since_one.records[0].body.clone()), (1, true, 2, None));
    assert_eq!(r.pull(user, Ask::first(0), t(10)).await.unwrap().records.len(), 0, "everything not deleted");
}

#[sqlx::test]
async fn a_device_older_than_the_purge_is_told_to_resync_and_one_up_to_date_is_not(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Rule, "r1", 0, 1, "{}"), put(Kind::Street, "s", 0, 1, "{}")], t(10)).await.unwrap(); // revs 1, 2
    r.push(user, vec![gone(Kind::Rule, "r1", 1, 2)], t(10)).await.unwrap(); // rev 3, a tombstone
    let later = t(10) + time::Duration::days(work::TOMBSTONE_DAYS + 1);
    assert_eq!(r.pull(user, Ask::first(1), later).await.unwrap_err(), WorkError::Resync, "a device at revision 1 never saw the deletion at 3");
    assert_eq!(r.pull(user, Ask::first(2), later).await.unwrap_err(), WorkError::Resync);
    assert_eq!(r.pull(user, Ask::first(3), later).await.unwrap().records.len(), 0, "a device that saw revision 3 is fine");
    assert_eq!(r.pull(user, Ask::first(0), later).await.unwrap().records.len(), 1, "everything not deleted: the street");
    let tombstones: i64 = sqlx::query_scalar("select count(*) from work where user_id = $1 and deleted").bind(user).fetch_one(&pool).await.unwrap();
    assert_eq!(tombstones, 0, "the tombstone is gone");
}

#[sqlx::test]
async fn a_late_arriving_old_edit_does_not_expire_its_tombstone_at_once(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    // a device offline for a year sends a deletion it made then: the tombstone's age counts from when the server stored it
    let year = time::Duration::days(365);
    r.push(user, vec![gone(Kind::Rule, "r1", 0, 0)], t(0) + year).await.unwrap(); // revision 1
    let a_day_later = t(0) + year + time::Duration::days(1);
    r.push(user, vec![put(Kind::Rule, "r2", 0, 5, "{}")], a_day_later).await.unwrap(); // revision 2
    let from_one = r.pull(user, Ask::first(1), a_day_later).await.unwrap();
    assert_eq!(from_one.records.iter().map(|rec| rec.id.key.as_str()).collect::<Vec<_>>(), ["r2"]);
    let tombstones: i64 = sqlx::query_scalar("select count(*) from work where user_id = $1 and deleted").bind(user).fetch_one(&pool).await.unwrap();
    assert_eq!(tombstones, 1, "kept");
}

#[sqlx::test]
async fn pages_hold_500_records_and_say_where_to_go_on(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    for batch in 0..6 {
        let puts: Vec<Put> = (0..100).map(|i| put(Kind::Street, &format!("k{batch}-{i}"), 0, 1, "{}")).collect();
        r.push(user, puts, t(10)).await.unwrap();
    }
    let first = r.pull(user, Ask::first(0), t(10)).await.unwrap();
    assert_eq!((first.records.len(), first.more, first.rev), (500, true, 500));
    let second = r.pull(user, Ask::first(0).continued(first.rev), t(10)).await.unwrap();
    assert_eq!((second.records.len(), second.more, second.rev), (100, false, 600));
}

#[sqlx::test]
async fn a_pull_from_zero_over_several_pages_is_not_told_to_start_over_by_a_tombstone_purged_meanwhile(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    for batch in 0..6 {
        let puts: Vec<Put> = (0..100).map(|i| put(Kind::Street, &format!("k{batch}-{i}"), 0, 1, "{}")).collect();
        r.push(user, puts, t(10)).await.unwrap(); // revisions 1 to 600
    }
    r.push(user, vec![put(Kind::Rule, "old", 0, 1, "{}")], t(10)).await.unwrap(); // 601
    r.push(user, vec![gone(Kind::Rule, "old", 601, 2)], t(10)).await.unwrap(); // 602, a tombstone
    let later = t(10) + time::Duration::days(work::TOMBSTONE_DAYS + 1);
    // the first page purges it: the floor is now 602, above the revision the second page goes on from
    let first = r.pull(user, Ask::first(0), later).await.unwrap();
    assert_eq!((first.records.len(), first.more, first.rev), (500, true, 500));
    let second = r.pull(user, Ask::first(0).continued(first.rev), later).await.unwrap();
    assert_eq!((second.records.len(), second.more, second.rev), (100, false, 602));
    // a pull that began at 500 is told, as before
    assert_eq!(r.pull(user, Ask::first(500), later).await.unwrap_err(), WorkError::Resync);
}

#[sqlx::test]
async fn turning_storage_off_and_on_gives_a_new_epoch_and_a_device_that_missed_it_is_told_to_reset(pool: PgPool) {
    let r = repo(pool);
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Street, "k", 0, 1, "{}")], t(10)).await.unwrap(); // revision 1
    let before = r.pull(user, Ask::first(0), t(10)).await.unwrap();
    let known = Ask::first(before.rev).with_epoch(before.epoch);
    assert_eq!(r.pull(user, known, t(10)).await.unwrap().records.len(), 0, "the same copy: nothing new");
    // another device turns storage off and on again, and makes a record: the counter is back at 1
    r.set_storage(user, false).await.unwrap();
    r.set_storage(user, true).await.unwrap();
    r.push(user, vec![put(Kind::Street, "other", 0, 5, "{}")], t(10)).await.unwrap();
    // the revision this device holds (1) is not above the counter: without the epoch it would be told nothing at all
    assert_eq!(r.pull(user, known, t(10)).await.unwrap_err(), WorkError::Reset);
    let fresh = r.pull(user, Ask::first(0), t(10)).await.unwrap();
    assert_ne!(fresh.epoch, before.epoch);
    assert_eq!(fresh.records.len(), 1);
    assert_eq!(r.pull(user, Ask::first(fresh.rev).with_epoch(fresh.epoch), t(10)).await.unwrap().records.len(), 0);
}

#[sqlx::test]
async fn a_pull_does_not_write_the_users_row_unless_a_tombstone_is_old_enough_to_purge(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    r.push(user, vec![put(Kind::Rule, "r1", 0, 1, "{}")], t(10)).await.unwrap();
    r.push(user, vec![gone(Kind::Rule, "r1", 1, 2)], t(10)).await.unwrap(); // a tombstone, stored at t(10)
    let before = row_version(&pool, user).await;
    r.pull(user, Ask::first(0), t(20)).await.unwrap();
    r.pull(user, Ask::first(1), t(20)).await.unwrap();
    assert_eq!(row_version(&pool, user).await, before, "reads stay reads: no tombstone is 90 days old");
    r.pull(user, Ask::first(2), t(10) + time::Duration::days(work::TOMBSTONE_DAYS + 1)).await.unwrap();
    assert_ne!(row_version(&pool, user).await, before, "the purge raised the floor");
}

#[sqlx::test]
async fn deleting_the_account_deletes_the_work_and_rewrapping_the_key_leaves_it_readable(pool: PgPool) {
    let v1 = repo(pool.clone());
    let user = storing(&v1, "a").await;
    v1.push(user, vec![put(Kind::Street, "k", 0, 1, "{\"n\":1}")], t(10)).await.unwrap();
    let v2 = PgRepo::new(
        pool.clone(),
        MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32])), (2, zeroize::Zeroizing::new([2; 32]))], 2).unwrap(),
        LookupKey::new(zeroize::Zeroizing::new([3; 32])),
    );
    assert_eq!(v2.rewrap().await.unwrap(), 1);
    assert_eq!(text(&v2.pull(user, Ask::first(0), t(10)).await.unwrap().records[0].body), "{\"n\":1}");
    v2.delete_user(user).await.unwrap();
    let n: i64 = sqlx::query_scalar("select count(*) from work").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test]
async fn a_batch_that_replaces_bodies_with_tombstones_and_smaller_ones_brings_a_full_account_back_under_the_limit(pool: PgPool) {
    let r = repo(pool.clone());
    let user = storing(&r, "a").await;
    let sixty = "z".repeat(60_000 - 8);
    let big = format!("{{\"x\":\"{sixty}\"}}");
    // 34 sealed bodies of about 60,000 bytes: the account is nearly full
    for batch in 0..4 {
        let puts: Vec<Put> = (0..10).filter(|i| batch * 10 + i < 34).map(|i| put(Kind::Rule, &format!("r{}", batch * 10 + i), 0, 1, &big)).collect();
        r.push(user, puts, t(10)).await.unwrap();
    }
    let stored = |pool: PgPool| async move {
        sqlx::query_scalar::<_, i64>("select coalesce(sum(octet_length(body)), 0)::bigint from work where user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    let full = stored(pool.clone()).await;
    assert!(full as usize + 60_000 > work::MAX_SEALED_BYTES, "another 60 KB body does not fit");
    assert_eq!(r.push(user, vec![put(Kind::Rule, "extra", 0, 1, &big)], t(10)).await.unwrap_err(), WorkError::TooLarge(Limit::Bytes));
    // one batch: ten tombstones and ten shrunk bodies free about 1.2 MB, and an addition of 60 KB then fits
    let mut batch: Vec<Put> = (0..10).map(|i| gone(Kind::Rule, &format!("r{i}"), i as i64 + 1, 20)).collect();
    batch.extend((10..20).map(|i| put(Kind::Rule, &format!("r{i}"), i as i64 + 1, 20, "{}")));
    batch.push(put(Kind::Rule, "extra", 0, 20, &big));
    let pushed = r.push(user, batch, t(30)).await.unwrap();
    assert!(pushed.items.iter().all(|i| i.verdict == Verdict::Accepted));
    let after = stored(pool).await;
    let sealed_big = 60_000 + work::SEAL_OVERHEAD as i64;
    assert_eq!(full, 34 * sealed_big);
    assert_eq!(after, 15 * sealed_big + 10 * (2 + work::SEAL_OVERHEAD as i64), "the planner's byte count matches the bytes stored");
}
