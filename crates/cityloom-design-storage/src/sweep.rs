//! `sweep` — the batched expiry-sweep task (`performance-design.md` PD-3,
//! `reliability-design.md` RD-3, `infrastructure-specification.md` ID-18):
//! deletes expired anonymous rows in bounded batches, never one long
//! transaction, and never touches an account-owned row. Idempotent and
//! stateless across runs — it always asks "what is expired now," never
//! "what changed since the last run" — so a skipped or delayed cycle only
//! ever means the next cycle finds a larger backlog; no design ever
//! disappears early.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::failure::StorageFailure;
use crate::observability::Counters;
use crate::{observability, repository};

/// `infrastructure-specification.md` ID-18's numbers, owned by the
/// composition root (`DesignStorageConfig::production_defaults`).
#[derive(Clone, Copy, Debug)]
pub struct SweepConfig {
    pub interval: Duration,
    pub batch_size: i64,
}

/// `observability-design.md` OD-3's one row per *cycle*: every batch this
/// cycle ran, summed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SweepCycleResult {
    pub rows_examined: i64,
    pub rows_deleted: i64,
    pub duration_ms: u128,
}

/// Run one full sweep cycle: repeat the batched delete until it deletes
/// zero rows, summing the total across every batch (`performance-design.md`
/// PD-3 — each batch is its own short statement, never one unbounded
/// `DELETE`).
pub async fn run_cycle(
    pool: &sqlx::PgPool,
    batch_size: i64,
) -> Result<SweepCycleResult, StorageFailure> {
    let start = Instant::now();
    let mut total_deleted: i64 = 0;
    loop {
        let deleted = repository::delete_expired_batch(pool, batch_size).await?;
        total_deleted += deleted;
        if deleted == 0 {
            break;
        }
    }
    Ok(SweepCycleResult {
        // Every row this predicate finds in a batch it deletes in the same
        // statement (`repository::delete_expired_batch`'s own doc), so
        // "examined" and "deleted" are the same total for a cycle.
        rows_examined: total_deleted,
        rows_deleted: total_deleted,
        duration_ms: start.elapsed().as_millis(),
    })
}

/// The in-process background task the composition root spawns once
/// (`logical-components.md` LC-3): run a cycle every `config.interval`,
/// emitting exactly one `observability-design.md` OD-3 row per cycle. A
/// cycle's own failure is logged and never panics the task — the next
/// tick tries again (`reliability-design.md` RD-3's "a skipped cycle has
/// exactly one consequence: a larger backlog next time").
///
/// `counters` is the same `Counters` instance handlers write to
/// (`handlers::AppState::counters_arc`) — after a successful cycle, this
/// also samples `stored_rows_count{ownership="anonymous"|"account"}`
/// (`observability-design.md` OD-2), per OD-3/RD-3's "sampled on the
/// sweep's own cycle, not per-request."
pub async fn run_forever(pool: sqlx::PgPool, config: SweepConfig, counters: Arc<Counters>) {
    let mut interval = tokio::time::interval(config.interval);
    loop {
        interval.tick().await;
        match run_cycle(&pool, config.batch_size).await {
            Ok(result) => {
                observability::record_sweep_cycle(
                    result.rows_examined,
                    result.rows_deleted,
                    result.duration_ms,
                );
                sample_stored_rows_count(&pool, &counters).await;
            }
            Err(failure) => {
                tracing::error!(
                    event = "sweep_cycle_failed",
                    reason = failure.reason.as_str(),
                );
            }
        }
    }
}

/// OD-2 — sample the current anonymous/account-owned row split into
/// `counters`. A sampling failure is logged and never panics the sweep
/// task — the next cycle's sample tries again, matching RD-3's stated
/// tolerance for a skipped/delayed reading.
async fn sample_stored_rows_count(pool: &sqlx::PgPool, counters: &Counters) {
    match repository::count_stored_rows_by_ownership(pool).await {
        Ok((anonymous, account)) => counters.set_stored_rows_count(anonymous, account),
        Err(failure) => {
            tracing::error!(
                event = "stored_rows_count_sample_failed",
                reason = failure.reason.as_str(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{self, InsertOutcome};
    use crate::test_support::{test_pool, unique_id};
    use time::OffsetDateTime;

    async fn insert_expired(pool: &sqlx::PgPool, prefix: &str, days_ago: i64) -> String {
        let id = unique_id(prefix);
        let outcome = repository::insert_anonymous(
            pool,
            &id,
            "{}",
            OffsetDateTime::now_utc() - time::Duration::days(days_ago + 1),
            OffsetDateTime::now_utc() - time::Duration::days(days_ago),
        )
        .await
        .unwrap();
        assert_eq!(outcome, InsertOutcome::Inserted);
        id
    }

    async fn insert_not_yet_expired(pool: &sqlx::PgPool, prefix: &str) -> String {
        let id = unique_id(prefix);
        let outcome = repository::insert_anonymous(
            pool,
            &id,
            "{}",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc() + time::Duration::days(30),
        )
        .await
        .unwrap();
        assert_eq!(outcome, InsertOutcome::Inserted);
        id
    }

    // AC11.3.2 — a sweep cycle deletes expired anonymous rows and never
    // touches an account-owned row (which never sets expires_at, so it
    // never matches the sweep's own predicate — a defensive test anyway).
    #[test]
    fn sweep_deletes_expired_anonymous_rows_and_never_deletes_unexpired_ones() {
        crate::test_support::runtime().block_on(async {
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            let expired_id = insert_expired(&pool, "sweep-expired", 1).await;
            let fresh_id = insert_not_yet_expired(&pool, "sweep-fresh").await;

            let result = run_cycle(&pool, 500).await.unwrap();
            assert!(result.rows_deleted >= 1);

            assert!(
                repository::select_by_anonymous_id(&pool, &expired_id)
                    .await
                    .unwrap()
                    .is_none(),
                "the expired row must be gone"
            );
            assert!(
                repository::select_by_anonymous_id(&pool, &fresh_id)
                    .await
                    .unwrap()
                    .is_some(),
                "the not-yet-expired row must never be deleted early"
            );
        });
    }

    // observability-design.md OD-3 — exactly one aggregate row per cycle
    // with the three named fields; asserted at the type level here (the
    // handlers-level log-capture test asserts the emitted JSON shape).
    #[test]
    fn a_cycle_reports_the_three_named_aggregate_fields() {
        crate::test_support::runtime().block_on(async {
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            insert_expired(&pool, "sweep-agg", 1).await;

            let result = run_cycle(&pool, 500).await.unwrap();
            // The three fields observability-design.md OD-3 names, and only
            // those — the type itself has exactly these three fields.
            let _: (i64, i64, u128) = (
                result.rows_examined,
                result.rows_deleted,
                result.duration_ms,
            );
            assert_eq!(result.rows_examined, result.rows_deleted);
        });
    }

    // performance-design.md PD-3 — a sweep processes more than one batch
    // correctly when the expired set exceeds the batch size.
    #[test]
    fn a_sweep_processes_more_than_one_batch_when_the_expired_set_exceeds_it() {
        crate::test_support::runtime().block_on(async {
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            let batch_size = 2;
            let mut expired_ids = Vec::new();
            for i in 0..5 {
                expired_ids.push(insert_expired(&pool, &format!("sweep-batch-{i}"), 1).await);
            }

            let result = run_cycle(&pool, batch_size).await.unwrap();
            assert!(
                result.rows_deleted >= 5,
                "all 5 expired rows must be deleted across multiple batches, got {}",
                result.rows_deleted
            );

            for id in &expired_ids {
                assert!(
                    repository::select_by_anonymous_id(&pool, id)
                        .await
                        .unwrap()
                        .is_none()
                );
            }
        });
    }

    // NFR7.4.2/observability-design.md OD-2 — a sweep cycle samples
    // stored_rows_count into the shared Counters, split anonymous/account
    // (`repository::count_stored_rows_by_ownership`'s own test covers the
    // split itself; this covers the wiring from a cycle into the gauge).
    #[test]
    fn a_sweep_cycle_samples_stored_rows_count_into_the_shared_counters() {
        crate::test_support::runtime().block_on(async {
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            insert_not_yet_expired(&pool, "sweep-count-sample").await;
            let counters = std::sync::Arc::new(crate::observability::Counters::new());

            run_cycle(&pool, 500).await.unwrap();
            sample_stored_rows_count(&pool, &counters).await;

            let snapshot = counters.snapshot();
            assert!(
                snapshot.stored_rows_anonymous >= 1,
                "expected the not-yet-expired anonymous row to be counted, got {}",
                snapshot.stored_rows_anonymous
            );
        });
    }

    // reliability-design.md RD-3 — a skipped/delayed cycle still deletes
    // correctly and never deletes a row early: a row that expires two days
    // from now is untouched by a cycle run "late" (i.e. now), and a row
    // that expired before now is still cleanly removed by that same
    // (idempotent, stateless) cycle.
    #[test]
    fn a_delayed_cycle_still_deletes_correctly_and_never_early() {
        crate::test_support::runtime().block_on(async {
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = test_pool().await;
            let overdue_id = insert_expired(&pool, "sweep-overdue", 3).await;
            let not_due_yet_id = unique_id("sweep-not-due-yet");
            repository::insert_anonymous(
                &pool,
                &not_due_yet_id,
                "{}",
                OffsetDateTime::now_utc(),
                OffsetDateTime::now_utc() + time::Duration::days(2),
            )
            .await
            .unwrap();

            // Simulates a cycle that was skipped/delayed by running the cycle
            // once "late" — the sweep never tracks a last-run timestamp of its
            // own (RD-3), so this is exactly equivalent to any other cycle.
            let result = run_cycle(&pool, 500).await.unwrap();
            assert!(result.rows_deleted >= 1);

            assert!(
                repository::select_by_anonymous_id(&pool, &overdue_id)
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                repository::select_by_anonymous_id(&pool, &not_due_yet_id)
                    .await
                    .unwrap()
                    .is_some(),
                "a row not yet due must never be deleted early"
            );
        });
    }
}
