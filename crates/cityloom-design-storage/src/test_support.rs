//! Test-only support: a real local Postgres pool with this crate's
//! migration applied (`team.md`'s TDD posture — the database is never
//! mocked; `unit-test-instructions.md`), and small helpers for building
//! unique test identifiers and timestamps. Compiled only under `#[cfg(test)]`.
//!
//! Every async test in this crate runs on [`runtime`]'s single, shared,
//! process-lifetime `tokio::runtime::Runtime` — via `#[test] fn name() {
//! crate::test_support::runtime().block_on(async { ... }) }` rather than
//! `#[tokio::test]`, which would give each test its *own*, independent
//! runtime. `sqlx::PgPool` spawns a background maintenance task tied to
//! whichever runtime is active when the pool is created; sharing one pool
//! (via [`test_pool`]) across many independent per-test runtimes leaves
//! that task orphaned the moment the first test's own runtime shuts down,
//! and the pool degrades from there (observed here as sporadic, and then
//! not-so-sporadic, `PoolTimedOut` errors under `cargo test`'s default
//! parallelism). One shared runtime for the whole test binary — same
//! shape as a real production process, which also has exactly one — is
//! the standard fix, not a smaller pool or less parallelism.

use std::sync::OnceLock;
use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, MutexGuard, OnceCell};

/// The one Tokio runtime every test in this crate drives its async work
/// through — see the module doc for why a shared runtime, not one per
/// test, is required here.
pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        // `tracing`'s per-callsite `Interest` cache is process-global, not
        // per-thread: the first time any tracing macro fires on a thread
        // with no subscriber installed, that callsite is cached as
        // `Interest::never()` for the rest of the process — permanently
        // bypassing dispatch, even for a later thread-local override via
        // `tracing::subscriber::with_default` (as `handlers`' log-capture
        // test does). Every test calls `runtime()` first, so installing a
        // real (discarding) global default here, before any other test
        // thread can exercise a `record_failure`/`record_sweep_cycle`
        // callsite, keeps every callsite's interest live for that later
        // scoped override to actually receive events.
        let _ = tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_writer(std::io::sink)
                .finish(),
        );
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build this crate's shared test runtime")
    })
}

/// `DATABASE_URL` for tests. `sqlx`'s own macros read `.env` at compile
/// time (via `dotenvy`, `sqlx`'s own dependency) for query-checking; this
/// reads the process environment at *run* time for the pool tests actually
/// connect with, falling back to the same local Postgres instance
/// `unit-test-instructions.md` documents for development.
fn database_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://cityloom:cityloom@localhost:5432/cityloom_design_storage_test".to_string()
        })
    })
}

/// A shared pool for the whole test binary — `sqlx::PgPool` is cheaply
/// cloneable and safe to share across concurrent tests (all now running
/// on the one shared [`runtime`]), and creating one per test would exhaust
/// connections under the default pool size.
static POOL: OnceCell<PgPool> = OnceCell::const_new();

/// Connect (once) to the local test database and run this crate's
/// migration against it. Every repository/handler/sweep test calls this
/// rather than mocking the database.
pub async fn test_pool() -> PgPool {
    POOL.get_or_init(|| async {
        // A ceiling for this test binary's own concurrent tests — not to
        // be confused with `infrastructure-specification.md` ID-17's
        // production `max_connections = 10` (`DesignStorageConfig`, not
        // this test-only pool).
        let pool = PgPoolOptions::new()
            .max_connections(40)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url())
            .await
            .expect("connect to the local test Postgres instance — see unit-test-instructions.md");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run this crate's migration against the local test database");
        pool
    })
    .await
    .clone()
}

/// `sweep::run_cycle` scans the *whole* `stored_designs` table — it has no
/// per-test scoping, by design (a real sweep never would). Any test that
/// calls it must therefore run exclusive of every other test doing the
/// same, or two sweep tests running concurrently can delete rows out from
/// under each other. This lock is that exclusion; every sweep test (and
/// the one repository test that depends on the table having no other
/// sweep running concurrently) acquires it for the duration of the test.
static SWEEP_LOCK: Mutex<()> = Mutex::const_new(());

/// Acquire exclusive access to the shared table for the duration of a
/// sweep-sensitive test. Hold the returned guard until the test's
/// assertions are done.
pub async fn exclusive_table_access() -> MutexGuard<'static, ()> {
    SWEEP_LOCK.lock().await
}

/// A unique-enough test identifier: a fixed prefix (for readability in a
/// failed assertion) plus a monotonically increasing counter, so
/// concurrently run tests never collide on `anonymous_design_id`'s unique
/// index.
pub fn unique_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("{prefix}-{pid}-{n}")
}
