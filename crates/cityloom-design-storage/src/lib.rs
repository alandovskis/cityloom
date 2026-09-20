//! `cityloom-design-storage` — Unit U10 of CityLoom.
//!
//! Owns `DesignRepository` (`components.md`): stored designs, anonymous
//! uploads now and account-owned designs from product Stage 2, their
//! retention, object-level authorisation on every read, and rate limiting
//! against identifier guessing (`entities.md`, `rules.md`).
//!
//! Per `logical-components.md` LC-3, this crate exposes exactly one public
//! constructor from this file: [`build`]. It takes the already-constructed
//! `PgPool` (the composition root's job, not this crate's) and returns the
//! configured `axum::Router` for `/api/designs*` and `/readyz`, plus the
//! spawned expiry-sweep task's `JoinHandle`. No other code in this crate
//! constructs a `PgPool` or reaches across to another Unit's crate.
//!
//! Module rings (`logical-components.md` LC-1): `handlers` is the only
//! module depending on `axum`; `repository` is the only module depending on
//! `sqlx`. `rate_limit` and `sweep` are plain Rust state/logic with no
//! dependency on either, so they are unit-testable without an HTTP server.

pub mod failure;
pub mod handlers;
pub mod observability;
pub mod rate_limit;
pub mod repository;
pub mod sweep;

#[cfg(test)]
pub(crate) mod test_support;

use std::time::{Duration, Instant};

use axum::Router;
use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::rate_limit::RateLimiter;
use crate::sweep::SweepConfig;

/// Numeric thresholds and intervals this Unit owns
/// (`infrastructure-specification.md` ID-18, ID-19, ID-20).
#[derive(Clone, Copy, Debug)]
pub struct DesignStorageConfig {
    /// ID-19 — requests per minute allowed against one `anonymousDesignId`.
    pub rate_limit_per_minute: u32,
    /// ID-19 — requests per hour allowed against one `anonymousDesignId`.
    pub rate_limit_per_hour: u32,
    /// The rate limiter's own minute window length. Production code
    /// leaves this at a real minute (`production_defaults`); it exists as
    /// a field (rather than a hardcoded constant) so this crate's own
    /// `build()`-wiring test can shrink it and observe `RateLimiter::sweep`
    /// actually reclaiming memory without waiting out real time (R-01).
    pub rate_limit_minute_window: Duration,
    /// As above, for the hour window.
    pub rate_limit_hour_window: Duration,
    /// R-01 / `security-design.md` SD-2 — how often this crate's own
    /// composition root ([`build`]) sweeps the rate limiter's in-memory
    /// identifier map, so it never grows unbounded for the life of the
    /// process.
    pub rate_limiter_sweep_interval: Duration,
    /// ID-20 — the raw upload byte-length cap, checked before
    /// deserialization (`security-design.md` SD-3).
    pub max_payload_bytes: usize,
    /// ID-18 — how often the expiry sweep runs.
    pub sweep_interval: Duration,
    /// ID-18 — how many expired rows the sweep removes per batch
    /// (`performance-design.md` PD-3).
    pub sweep_batch_size: i64,
}

impl DesignStorageConfig {
    /// The numbers `infrastructure-specification.md` fixes: 10/minute,
    /// 100/hour (ID-19), 5 MiB (ID-20), a 6-hour sweep interval in
    /// 500-row batches (ID-18); the rate limiter's own windows are a real
    /// minute and a real hour, swept once an hour (R-01).
    pub fn production_defaults() -> DesignStorageConfig {
        DesignStorageConfig {
            rate_limit_per_minute: 10,
            rate_limit_per_hour: 100,
            rate_limit_minute_window: Duration::from_secs(60),
            rate_limit_hour_window: Duration::from_secs(60 * 60),
            rate_limiter_sweep_interval: Duration::from_secs(60 * 60),
            max_payload_bytes: 5 * 1024 * 1024,
            sweep_interval: Duration::from_secs(6 * 60 * 60),
            sweep_batch_size: 500,
        }
    }
}

/// The composition root's one entry point into this crate
/// (`logical-components.md` LC-3): construct the router and spawn the
/// expiry-sweep and rate-limiter-sweep tasks from an already-open `PgPool`.
pub struct DesignStorage {
    pub router: Router,
    /// The batched database expiry-sweep task (`sweep::run_forever`).
    pub sweep_handle: JoinHandle<()>,
    /// R-01 — the periodic in-memory rate-limiter sweep task
    /// (`security-design.md` SD-2): without this, `RateLimiter`'s own
    /// per-identifier map would grow for the life of the process.
    pub rate_limiter_sweep_handle: JoinHandle<()>,
    /// Test-only: the same `AppState` mounted into `router`, kept here so
    /// this crate's own `build()`-wiring test can observe the rate
    /// limiter's tracked-identifier count directly, through the actual
    /// composition-root path, rather than reconstructing pieces by hand.
    #[cfg(test)]
    pub(crate) test_state: handlers::AppState,
}

/// Build the `/api/designs*` + `/readyz` router and spawn the expiry-sweep
/// and rate-limiter-sweep background tasks, from a `PgPool` the caller
/// already constructed (`tech-stack-decisions.md`'s open assumption on the
/// surrounding binary is not this crate's to resolve).
pub fn build(pool: PgPool, config: DesignStorageConfig) -> DesignStorage {
    let limiter = RateLimiter::with_windows(
        config.rate_limit_per_minute,
        config.rate_limit_per_hour,
        config.rate_limit_minute_window,
        config.rate_limit_hour_window,
    );
    let state = handlers::AppState::new(pool.clone(), limiter, config.max_payload_bytes);
    let router = handlers::router(state.clone());

    let sweep_handle = tokio::spawn(sweep::run_forever(
        pool,
        SweepConfig {
            interval: config.sweep_interval,
            batch_size: config.sweep_batch_size,
        },
        state.counters_arc(),
    ));

    // R-01 — the rate limiter's own in-memory map is per-process state
    // that nothing else ever cleans up; without this task it grows for
    // the life of the process (`security-design.md` SD-2, BR4.1).
    let rate_limiter_sweep_handle = {
        let limiter = state.limiter_arc();
        let sweep_interval = config.rate_limiter_sweep_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sweep_interval);
            loop {
                interval.tick().await;
                limiter.sweep(Instant::now());
            }
        })
    };

    DesignStorage {
        router,
        sweep_handle,
        rate_limiter_sweep_handle,
        #[cfg(test)]
        test_state: state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // R-01 — `build()` itself wires a periodic sweep of the rate
    // limiter's in-memory map, exercised through the actual composition
    // root (not the bare `RateLimiter` type in isolation): under
    // sustained traffic against many distinct identifiers, the
    // tracked-identifier count grows and then, once the wired-up periodic
    // sweep has had a chance to run past the (deliberately shrunk, for
    // this test) hour window, shrinks back down — proving the map does
    // not grow unbounded for the life of the process.
    #[test]
    fn build_wires_a_periodic_rate_limiter_sweep_that_keeps_the_map_bounded() {
        crate::test_support::runtime().block_on(async {
            // `build()` also spawns the real database expiry-sweep task
            // (`sweep::run_forever`) against this same shared test
            // database — and `tokio::time::interval`'s first tick fires
            // immediately, regardless of `sweep_interval`'s length, so
            // that task *will* run one real cycle almost as soon as this
            // test starts. That cycle only ever deletes already-expired
            // anonymous rows, which only the sweep-cycle tests in
            // `sweep.rs` (and one in `repository.rs`) ever create — this
            // guard serialises against exactly those, the same as every
            // one of their own tests already does.
            let _guard = crate::test_support::exclusive_table_access().await;
            let pool = crate::test_support::test_pool().await;
            let mut config = DesignStorageConfig::production_defaults();
            config.rate_limit_minute_window = Duration::from_millis(10);
            config.rate_limit_hour_window = Duration::from_millis(40);
            config.rate_limiter_sweep_interval = Duration::from_millis(15);
            // Kept long — this test exercises the rate-limiter sweep
            // only; the database expiry sweep's own single immediate
            // cycle (see above) is harmless (nothing here is expired) and
            // it must not tick a *second* time mid-test.
            config.sweep_interval = Duration::from_secs(3_600);

            let storage = build(pool, config);

            for i in 0..40 {
                let id = format!("build-wiring-bounded-{i}");
                let _ = storage.test_state.limiter_check_for_test(&id);
            }
            let peak = storage.test_state.limiter_tracked_identifiers();
            assert!(
                peak >= 30,
                "expected close to 40 distinct identifiers tracked immediately after traffic, got {peak}"
            );

            // Give the spawned task, ticking every 15ms, several chances
            // to run past the 40ms hour window.
            tokio::time::sleep(Duration::from_millis(400)).await;

            let after = storage.test_state.limiter_tracked_identifiers();
            assert!(
                after < peak,
                "expected build()'s own wired-up periodic sweep to have shrunk the map (peak {peak}, after {after})"
            );

            storage.sweep_handle.abort();
            storage.rate_limiter_sweep_handle.abort();
        });
    }
}
