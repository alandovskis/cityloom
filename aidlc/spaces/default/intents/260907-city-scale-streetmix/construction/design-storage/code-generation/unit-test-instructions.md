# Unit Test Instructions — `design-storage` (U10)

## Framework and setup

Standard `cargo test`, reusing `rust-toolchain.toml` (rustc/cargo 1.97.1).
No new test framework crate. Unlike every prior Unit in this workspace,
this crate's repository and handler tests need a **real local PostgreSQL
database** — `team.md`'s TDD posture treats mocking the database as a
practice this project does not use, and `security-design.md` SD-5's
compile-time-checked `sqlx` queries need a real schema to check against
in any case. Bootstrap a local Postgres instance (Docker or a locally
installed server — the developer agent's choice, matching
`code-generation-plan.md`'s stated assumption), point `DATABASE_URL` at
it, and run this crate's migration (`migrations/0001_create_stored_designs.sql`)
before the first Red test. Once the first real query exists, run
`cargo sqlx prepare -p cityloom-design-storage` to generate the committed
`.sqlx/` offline query cache (`infrastructure-specification.md` ID-16),
so CI's fast tier can compile this crate without a reachable database.

## Exact unit-scoped run command

```bash
cargo test -p cityloom-design-storage
```

Scoped to only this crate's tests. Requires `DATABASE_URL` pointed at a
reachable Postgres instance with this crate's migration applied — record
the exact local setup commands used (e.g. a `docker run postgres:16` line
and the migration-apply command) in this Unit's own README or
`code-summary.md` if a step beyond the bare `cargo test` invocation is
needed to make the command runnable, since `cicd-pipeline.md`'s fast
tier automates the equivalent setup as a service container.

## Test scope (Standard strategy: 5-8 tests per component, 4 modules)

**`StoredDesign`/`UploadReceipt`/`StorageFailure` (data model):**
1. `StoredDesign` construction round-trips every field named in
   `entities.md`.
2. `StorageFailure`'s `reason` enum has exactly the six values Contract
   2's `ApiError` reason enum names — no extra, no missing.
3. A migrated `stored_designs` table has exactly the columns
   `entities.md` names for `StoredDesign` and no others — a schema
   introspection test (NFR6.3.1, `security-design.md` SD-7).
4. `UploadReceipt`'s `expires_at` is exactly 30 days after `uploaded_at`
   when constructed for an anonymous upload (AC11.3.1).

**`repository` module (repository / data access, against the real local
database):**
5. Insert then select-by-id round-trips the payload unchanged.
6. Select-by-id for a row whose `expires_at` is in the past returns
   not-found, even though the row still physically exists (BR2.1).
7. Delete-by-id for a non-existent id succeeds without error
   (idempotent, BR5.1, AC8.3.4).
8. A second insert with a conflicting `anonymous_design_id` reports zero
   rows affected, not an error (`security-design.md` SD-6,
   `performance-design.md` PD-1's `ON CONFLICT DO NOTHING`).
9. Every query is issued through a `sqlx::query!`/`query_as!` macro with
   bound parameters — a code-inspection assertion (or a test with a
   request-derived value containing SQL metacharacters, asserting no
   injection occurs) for `security-design.md` SD-5.

**`rate_limit` + `failure` + validation logic (business logic):**
10. Under both the 10/minute and 100/hour thresholds, a request against
    one identifier passes.
11. At the 10/minute threshold, the 11th request within the window
    returns `rate_limited` with a computed `Retry-After`
    (`security-design.md` SD-2, NFR5.3.1).
12. Every `sqlx::Error` variant the repository can return maps to
    `StorageFailure { reason: internal }` with no driver error text in
    `detail` (`security-design.md` SD-6, T4).
13. A payload exceeding the 5 MiB cap is rejected (`413`) before any
    database call — asserted by a spy/counter on the repository's insert
    function never being invoked (BR6.1, NFR6.4.3).
14. A payload with an invalid `payloadVersion` is rejected (`400`) via
    `cityloom-design-payload::DesignPayload::from_json`'s existing
    fail-closed check, before any database call.

**`handlers` module (API / endpoint, against the merged router with the
real local database):**
15. `POST /api/designs` with a valid payload returns `201` with an
    `UploadReceipt` (AC8.3.1).
16. `POST /api/designs` with a conflicting identifier returns `409`.
17. `GET /api/designs/{id}` for a valid, unexpired, matching id returns
    `200` with the unchanged payload.
18. `GET /api/designs/{id}` for a missing or expired id returns `404`,
    and the response body contains no payload field (BR3.1, AC10.1.2,
    AC10.1.3, NFR6.4.2).
19. `DELETE /api/designs/{id}` always returns `204`, whether the row
    existed or not (BR5.1, AC8.3.4).
20. A log-capture test across one upload failure, one fetch-404, and one
    rate-limit-429 asserts no identifier, payload, or requester-identifying
    field appears in any emitted row (NFR7.4.1, `security-design.md` SD-7).
21. A simulated database outage causes the shared `/readyz` endpoint to
    report not-ready while the process keeps running (NFR7.4.4,
    `reliability-design.md` RD-2).

**`sweep` module (business logic — expiry):**
22. A sweep cycle deletes an expired anonymous row and never touches an
    account-owned row with a past-dated `expires_at`-equivalent state
    (there is none to have, since account-owned rows never set
    `expires_at` — constructed as a defensive test anyway, AC11.3.2).
23. A sweep cycle emits exactly one aggregate row with the three fields
    `rows_examined`, `rows_deleted`, `duration_ms` — no row identifiers
    (NFR7.4.3, `observability-design.md` OD-3).
24. A sweep processes more than one batch (500-row limit) correctly when
    the expired set exceeds one batch, without holding one lock across
    the whole set (`performance-design.md` PD-3, NFR1.4.4).

24 tests total across four modules (4 data model, 5 repository, 5
business-logic/validation, 7 API/endpoint, 3 sweep), within the Standard
strategy's 5-8-per-component band (the data-model and sweep groups run
slightly under 5 because their scope is narrower than a full component,
matching the plan's own note that "business logic" splits across two
genuinely distinct modules for this Unit).

## Coverage target

80% line coverage floor — this crate is not one of `team.md`'s five
explicitly named client-side crates/modules (it is server-only), but the
same org-default 80% floor for `feature` scope applies to every crate in
the workspace per `team.md`'s stated merge gate item 5
(`cargo-llvm-cov` over "the measured set," which `logical-components.md`
LC-1 extends to include this Unit's own modules per NFR7.1.1's
precedent-setting reading, matching how `street-import` and
`design-editing` each extended the measured set for their own crates).

## Mocking/stubbing guidance

The database is **never mocked** — every repository and handler test
runs against a real local Postgres instance, per `team.md`'s TDD posture
and this Unit's own `security-design.md` SD-5 (a compile-time-checked
query needs a real schema to check against). The only thing injected for
testability is time, where a sweep test needs to simulate an interval
having elapsed (e.g. a clock abstraction or directly manipulating
`expires_at` values in test rows rather than waiting real wall-clock
time) — never a mock of the database connection itself.

## Test data management

Test rows are constructed inline per test via the repository's own
insert function (or a direct `sqlx::query!` in test setup code) — no
external fixture files, unlike `street-import`'s committed OSM fixtures,
since this Unit's test data is synthetic application data, not a
third-party dependency's characterised output. Each test that needs an
expired row constructs one directly with a past `expires_at`, never by
waiting for real time to pass.
