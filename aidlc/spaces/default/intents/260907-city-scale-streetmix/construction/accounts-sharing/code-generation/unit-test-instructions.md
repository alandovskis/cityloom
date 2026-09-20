# Unit Test Instructions — `accounts-sharing` (U11)

## Framework and setup

Standard `cargo test`, reusing `rust-toolchain.toml`. No new test
framework crate. Like `design-storage` (U10), this crate's repository and
handler tests need a **real local PostgreSQL database** — `team.md`'s TDD
posture treats mocking the database as a practice this project does not
use. Reuse the same local Postgres instance U10's own test setup already
established (`DATABASE_URL`), and run this crate's own migration
(`migrations/0001_create_accounts_sharing_tables.sql`) alongside U10's
before the first Red test — the two migration sets are independent (no
foreign key between them) so ordering between them doesn't matter, only
that both are applied. Once the first real query exists, run
`cargo sqlx prepare -p cityloom-accounts-sharing` to generate this crate's
own committed `.sqlx/` offline query cache
(`infrastructure-specification.md` ID-22).

## Exact unit-scoped run command

```bash
cargo test -p cityloom-accounts-sharing
```

Scoped to only this crate's tests. Requires `DATABASE_URL` pointed at the
same reachable Postgres instance `cityloom-design-storage`'s tests use,
with both crates' migrations applied.

**Note**: Step 1 of `code-generation-plan.md` also adds one function
(`select_by_stored_design_id`) to `cityloom-design-storage`'s own
`repository` module. Run `cargo test -p cityloom-design-storage` too
after that change, to confirm the addition didn't regress U10's own
existing 46-test suite.

## Test scope (Standard strategy: 5-8 tests per component, 4 modules)

**`Account`/`Session`/`SavedDesign`/`Grant`/`ShareLink` (data model):**
1. Each type's construction round-trips every field named in
   `entities.md`.
2. A migrated table has exactly the columns `entities.md` names for that
   entity and no others, for all five tables (schema-introspection test).

**`repository` module (repository / data access, against the real local
database):**
3. Account insert then select-by-email round-trips.
4. Session insert then select-by-token-hash round-trips; delete then
   select-by-token-hash returns not-found (NFR6.2.8's underlying query
   behavior).
5. SavedDesign insert then select-by-account lists it; a second insert
   with a duplicate `(account_id, name)` pair reports zero rows affected,
   not an error (matching U10's `security-design.md` SD-6 precedent).
6. `AccessGrant`/`ShareLink` insert/select/toggle round-trip independently
   of each other (BR7.2 — enabling a link does not touch existing
   grants and vice versa, asserted at the query layer).
7. The account-deletion cascade, run as one transaction: deletes the
   `Account` and every referencing row across all five tables; a forced
   mid-transaction failure (e.g. a constraint violation injected on the
   last delete) leaves every row from all five tables unchanged.
8. `select_by_stored_design_id` (the new `cityloom-design-storage`
   function) returns the stored design for a valid id and excludes an
   expired one, mirroring `select_by_anonymous_id`'s own existing test 6.

**`account` + `sharing` + `access_gate` + `failure` modules (business
logic):**
9. Password hashing: the stored value differs from the input password,
   and verification uses the `argon2` crate's constant-time verify
   function (NFR6.2.1).
10. Session token: a 256-bit random value; only its SHA-256 hash is
    stored, never the raw value (NFR6.2.1).
11. Access gate: `unset` and explicit `false` both compute to "disabled";
    only explicit `true` enables (NFR6.2.5, NFR6.2.7 — the fail-closed
    default).
12. `ShareLink` token: two tokens for different designs by the same
    account are statistically unrelated (NFR6.2.4).
13. Every failure variant this crate's repository can return maps to a
    closed `AccountsSharingFailure` enum with no driver error text
    (NFR6.2.6).

**`handlers` module (API / endpoint, against the merged router with the
real local database):**
14. Sign-up (`POST /api/accounts`) with a device-design payload attaches
    it as a `SavedDesign` unchanged (AC9.2.1, AC9.2.2); a failed migration
    leaves the device copy untouched and reports failure (AC9.2.3).
15. Sign-in (`POST /api/sessions`) issues a session reaching the same
    account regardless of which device signs in (AC9.1.2).
16. Sign-out (`DELETE /api/sessions`) deletes the session; the very next
    request with that token is refused (NFR6.2.8, no grace window).
17. Save-with-name (`POST /api/saved-designs`) with a duplicate name for
    the same account is refused, existing row untouched (AC9.3.3).
18. List (`GET /api/saved-designs`) with zero rows returns an explicit
    empty-state indicator (AC9.3.4); a non-empty list's sharing-state
    field reflects current Grant/ShareLink rows, computed at request
    time (AC10.3.1, AC10.3.2).
19. Grant issuance (`POST /api/saved-designs/{id}/grants`) to a
    non-account-holder is refused (BR7.1); enabling/disabling a
    `ShareLink` does not change existing grants and vice versa
    (AC10.2.1-AC10.2.4).
20. Shared-design read (`GET /api/shared/{...}`): an unauthorized caller
    gets a response with no payload data and no `DesignRepository` call
    is attempted (a spy/call-count assertion, NFR6.2.3, AC10.1.2,
    AC10.1.3); an authorized caller (via Grant or enabled ShareLink) gets
    the payload.
21. Every route with the access-gate flag unset returns a refusal,
    including a route a client would never normally be linked to
    (NFR6.2.5, NFR6.2.7, AC9.1.3).
22. A log-capture test across one of each failure path asserts no
    credential, session token, or account-identifying field appears in
    any emitted row.
23. A simulated database outage causes the shared `/readyz` endpoint to
    report not-ready while the process keeps running.

23 tests total across four modules (7 data model, 6 repository, 5
business-logic, 10 API/endpoint — endpoint count runs slightly over the
5-8 band because this Unit's route surface is wider than U10's three
routes; still within the Standard strategy's overall spirit given the
number of distinct business rules (9 BR groups) this Unit owns).

## Coverage target

80% line coverage floor — same org-default `feature`-scope floor every
crate in this workspace meets, measured with `cargo-llvm-cov -p
cityloom-accounts-sharing`. The one added `cityloom-design-storage`
function (`select_by_stored_design_id`) is covered under U10's own
existing `cargo-llvm-cov -p cityloom-design-storage` measurement (test
23 above, added to that crate's suite).

## Mocking/stubbing guidance

The database is **never mocked** — every repository and handler test runs
against a real local Postgres instance (`team.md`'s TDD posture,
`security-design.md` SD-7). The only thing injected for testability is
time (session `expires_at`, a forced mid-transaction failure for the
cascade test) — constructed directly in test setup, never a mock of the
database connection.

## Test data management

Test rows are constructed inline per test via the repository's own insert
functions — no external fixture files, matching `design-storage`'s own
convention (this Unit's test data is synthetic application data, not a
third-party dependency's characterised output).
