# Code Summary — `design-storage` (U10)

## Status — repaired (review iteration 1 of 2 findings addressed)

The architecture reviewer's adversarial review (iteration 1 of 2) returned
**NOT-READY** with 2 Major findings and 1 Minor finding (verdict and
findings preserved, unmodified, as `## Review` in `code-generation-plan.md`
— reviewer-owned, not edited by this repair). This repair pass addresses
all three:

- **R-01 — rate-limiter memory leak in production — fixed.**
  `RateLimiter::sweep()` is now wired into a periodic task
  `lib.rs::build()` itself spawns (`rate_limiter_sweep_handle`), sharing
  the exact same `RateLimiter` instance the handlers check against
  (`handlers::AppState::limiter_arc`) rather than a second, disconnected
  one. The sweep interval and the limiter's own minute/hour window
  durations are now `DesignStorageConfig` fields (previously hardcoded
  constants) — production code keeps the real minute/hour/1-hour-sweep
  values (`DesignStorageConfig::production_defaults`), and this crate's
  own end-to-end test
  (`tests::build_wires_a_periodic_rate_limiter_sweep_that_keeps_the_map_bounded`
  in `lib.rs`) shrinks them to observe the wired-up `build()` path
  actually evicting stale identifiers within a fast test, rather than
  only exercising the bare `RateLimiter` type in isolation.
- **R-02 — NFR7.4.2 implemented; traceability corrected.**
  `observability.rs::Counters` now carries the full `observability-design.md`
  OD-2 shape: `uploads_total{result="success"}` plus one failure series per
  `StorageFailure` reason, `fetches_total{result="success"|"403"|"404"|"429"}`,
  `deletes_total{result="success"|"429"}`, and a `stored_rows_count`
  gauge split anonymous/account, sampled once per expiry-sweep cycle
  (`sweep.rs::run_forever` → `repository::count_stored_rows_by_ownership`)
  rather than per-request, per OD-2/OD-3. Every counter is wired into
  `handlers.rs` at its actual outcome point. Tested the same way
  `crates/osm-extract-proxy/src/counters.rs` already established for this
  workspace: direct tests against `Counters`/`AppState`, no `/metrics` HTTP
  route (matching that precedent, which has none either). `NFR7.4.2` is
  now in `code-generation-plan.md`'s Story/AC traceability table and
  `traceability.json`'s `upstream_ids`/`coverage` with `status: "OK"`
  pointing at `src/observability.rs` (previously `status: "GAP"`).
- **R-03 — delete-failure blind spot — fixed (optional, taken).** Added a
  `deletes_failure` counter, additive to OD-2's confirmed success/429
  series, so a genuine repository/database failure on delete
  (`handlers::remove`'s `Err(failure)` arm) is never folded into
  `deletes_success` again.

No other change was made to this Unit's scope or design: the plan's other
steps, the schema, and every previously-passing test are unchanged.

## What was built

A new server-only Cargo workspace crate, `crates/cityloom-design-storage/`,
implementing the `DesignRepository` component (`components.md`): anonymous
design upload/fetch/delete, per-identifier rate limiting, and an in-process
expiry sweep. Six modules per `logical-components.md` LC-1's ring split:

- `failure.rs` — `StorageFailure` (typed via `thiserror`), the closed
  `FailureReason`-mapping function for every `sqlx::Error` variant, never
  leaking driver text (`security-design.md` SD-6).
- `repository.rs` — the only module depending on `sqlx`; insert,
  select-by-id (excluding expired rows), delete-by-id, all through
  `sqlx::query!`/`query_as!` compile-time-checked macros with bound
  parameters only (SD-5).
- `rate_limit.rs` — a plain in-memory two-fixed-window limiter (10/minute,
  100/hour per identifier, ID-19), no external dependency.
- `handlers.rs` — the only module depending on `axum`; the three
  `/api/designs*` routes plus this Unit's contribution to `/readyz`,
  orchestrating rate limiting → validation → the single repository call
  per operation (PD-1).
- `sweep.rs` — the in-process `tokio` background task, 6-hour interval,
  500-row batches (ID-18), spawned once by the composition root; each
  cycle also samples `stored_rows_count` into the shared `Counters`
  (OD-2, repair R-02).
- `observability.rs` — the `tracing` JSON subscriber shape (matching
  `osm-extract-proxy`'s own `emit::subscriber`) and the fixed-field-set
  failure/sweep-cycle log rows (OD-1, OD-3), plus `Counters` — the full
  OD-2 aggregate-counter shape (repair R-02): per-`StorageFailure`-reason
  upload failures, 403/404/429 fetch results, success/429/failure delete
  results (the last one additive, repair R-03), and the anonymous/account
  `stored_rows_count` gauge.
- `lib.rs` — the crate's one public constructor, `build(pool, config)`
  (LC-3), returning the configured `axum::Router`, the spawned database
  expiry-sweep task's `JoinHandle`, and (repair R-01) the spawned
  rate-limiter-sweep task's own `JoinHandle`, sharing the same
  `RateLimiter`/`Counters` instances the router's handlers use. Not yet
  wired into the shared Railway service's own `main` — that
  composition-root integration is this Unit's declared boundary
  (`tech-stack-decisions.md`'s "surrounding binary is not this crate's to
  resolve") and belongs to whichever Unit owns the shared entry point.

`migrations/0001_create_stored_designs.sql` creates the `stored_designs`
table with exactly the columns `entities.md` names, plus the unique index
on `anonymous_design_id` (`infrastructure-specification.md` ID-15).

## Key implementation decisions

- **TDD Red/Green/Refactor followed exactly as planned**, layer by layer
  (data model → repository → business logic → API/endpoint → sweep),
  against a real local PostgreSQL instance for every repository/handler/
  sweep test — the database is never mocked (`team.md`).
- **Database-outage `/readyz` test uses a deliberately-closed pool**
  (a `PgPool` connected then immediately closed) rather than an external
  fault injector, keeping the "database unreachable" case deterministic
  and hermetic.
- **The log-capture test (`handlers::tests::
  failure_logs_never_carry_an_identifier_payload_or_requester_field`) was
  flaky under `cargo test`'s default parallelism** — traced to `tracing`'s
  per-callsite `Interest` cache being process-global, not thread-scoped:
  the first time any test thread called `tracing::warn!`/`error!` with no
  subscriber installed on that thread, the callsite got cached as
  `Interest::never()` for the rest of the process, permanently bypassing
  dispatch even for this test's later thread-local
  `tracing::subscriber::with_default` override. Fixed by installing a
  discarding (`std::io::sink`) global default subscriber once, piggy-backed
  on `test_support::runtime()`'s `OnceLock` initializer that every test
  calls first — this keeps every tracing callsite's interest live so the
  scoped override actually receives events. Verified stable across 8
  consecutive full-parallelism runs after the fix (previously 5/5 failing
  under parallelism, 0/5 failing serially — a deterministic ordering bug,
  not a timing race).
- **`abstutil`'s unpinned-dependency deviation does not apply to this
  Unit** — `cityloom-design-storage` has no osm2streets/abstutil
  dependency at all; that concern is scoped to `cityloom-street-import`
  only (see `docs/dependencies.md`).
- **Repair R-01 — the rate limiter's window durations became
  `DesignStorageConfig` fields, not just its request thresholds.**
  Previously only `rate_limit_per_minute`/`rate_limit_per_hour` were
  configurable; the minute/hour window lengths themselves were hardcoded
  `MINUTE`/`HOUR` constants in `rate_limit.rs`. Making them explicit
  constructor parameters (`RateLimiter::with_windows`, with `new` as the
  real-minute/real-hour convenience wrapper) is what makes the R-01 fix
  actually verifiable in a fast test: production keeps the real values,
  while `lib.rs`'s own wiring test shrinks them to observe real,
  time-bounded eviction rather than waiting out a real hour.
- **Repair R-01 — `tokio::time::interval`'s "first tick fires
  immediately" behavior required a test-only `exclusive_table_access`
  guard.** `build()`'s spawned database expiry-sweep task runs one real
  cycle against the shared test database almost immediately, regardless
  of `sweep_interval`'s length (a well-known `tokio` interval property,
  not a bug introduced here) — the new `lib.rs` wiring test takes the
  same `exclusive_table_access()` lock every `sweep.rs`/`repository.rs`
  whole-table-scanning test already takes, so that immediate cycle can
  never race one of them.
- **Repair R-02 — `stored_rows_count`'s test asserts a lower bound for
  the anonymous side, not exact equality.** The gauge's own repository
  query (`count_stored_rows_by_ownership`) scans the *whole* table, and
  every other test in this suite inserts anonymous rows outside the
  `exclusive_table_access` lock (by design — only whole-table-scanning
  tests take it), running concurrently under `cargo test`'s default
  parallelism. Its test therefore asserts "at least this test's own row"
  for the anonymous count and exact equality for the account count (which
  no other test in the suite ever touches).

## Test results

`cargo test -p cityloom-design-storage`: **46 passed, 0 failed** (37 from
the prior review iteration, plus 9 added by this repair: `with_windows`'s
own unit test in `rate_limit.rs`; six new `observability.rs` tests
covering the full OD-2 counter shape; `repository.rs`'s
`count_stored_rows_by_ownership` test; `sweep.rs`'s
sample-into-shared-counters test; and `lib.rs`'s own end-to-end
`build()`-wiring test for R-01). Stable across 5 consecutive full-
parallelism runs.

`cargo clippy -p cityloom-design-storage --all-targets -- -D warnings`:
clean. `cargo fmt -p cityloom-design-storage -- --check`: clean.
`cargo build -p cityloom-design-storage` / `cargo check -p
cityloom-design-storage`: clean.

## Coverage

`cargo-llvm-cov` (with a live `DATABASE_URL`, the required mode for this
crate's macro-checked queries): **96.16% region / 96.02% line / 94.18%
function** coverage overall — above `team.md`'s 80% line-coverage floor,
and improved from the prior iteration's 93.91%/93.34%/91.89%. Per-file
line coverage: `failure.rs` 100%, `handlers.rs` 95.20%, `lib.rs` 100%
(the new `build()`-wiring test now exercises the composition root
directly), `observability.rs` 93.21%, `rate_limit.rs` 97.74%,
`repository.rs` 97.31%, `sweep.rs` 96.53%, `test_support.rs` 94.23%.

## Local database setup used

A local PostgreSQL instance is reachable at
`postgres://cityloom:cityloom@localhost:5432/cityloom_design_storage_test`
(this crate's exact bootstrap mechanism — Docker vs. a locally installed
server — was left to the developer agent's judgment per the plan's own
assumption; whichever was used, the credentials and database name above
are what this crate's `.env` and tests connect with). Recorded here as
`DATABASE_URL=postgres://cityloom:cityloom@localhost:5432/cityloom_design_storage_test`,
recorded in the crate-local (gitignored) `.env` for `sqlx`'s compile-time
macro checking, and read at test/run time the same way. Migration applied
automatically by `test_support::test_pool()` via `sqlx::migrate!` on first
use; no separate manual migration step is needed for local development or
CI's equivalent service-container setup (`cicd-pipeline.md`).

`.sqlx/` offline query cache generated with `cargo sqlx prepare -- --tests`
(covering both the library and the `#[cfg(test)]` query sites, e.g. the
schema-introspection test below) and verified current with
`cargo sqlx prepare --check -- --tests` under `SQLX_OFFLINE=true`, so
CI's fast tier can compile this crate without a reachable database
(`infrastructure-specification.md` ID-16). Regenerated by this repair for
the two new queries `repository::count_stored_rows_by_ownership` adds
(R-02) and the test-only account-row insert its own test uses.

## Deviations from the plan

- **Test count**: 37 tests were written against the planned 24 — every
  planned test survived, several split further where doing so kept each
  test's failure message unambiguous (e.g. the planned single
  "every `sqlx::Error` variant maps to `internal`" test became one test
  per variant group), plus one test (below) added on post-generation
  review. This still sits within `team.md`'s Standard strategy floor per
  component; see `unit-test-instructions.md`'s own note on the data-model/
  sweep groups running under the 5-8 band by design.
- **Step 3's schema-shape test was missing on first generation, then
  added.** Step 3 calls for "a schema-shape test asserting the migrated
  table has exactly the columns `entities.md` names and no more"
  (NFR6.3.1, `security-design.md` SD-7); every other Step 3 test was
  written, but this one was not. Caught at post-generation review and
  added as
  `repository::tests::the_migrated_table_has_exactly_the_columns_entities_md_names`,
  which queries `information_schema.columns` and asserts the exact
  6-column set. All Step 3 tests are now present.
