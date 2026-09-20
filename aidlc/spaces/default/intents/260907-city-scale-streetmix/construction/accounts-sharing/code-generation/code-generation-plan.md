# Code Generation Plan — `accounts-sharing` (U11)

Upstream inputs: `entities.md`, `rules.md`, `functional-spec.md`
(functional-design, this Unit); `performance-design.md`,
`security-design.md`, `scalability-design.md`, `reliability-design.md`,
`observability-design.md`, `logical-components.md` (nfr-design, this
Unit); `tech-stack-decisions.md` (nfr-requirements, this Unit);
`infrastructure-specification.md`, `cicd-pipeline.md` (infrastructure-design,
this Unit); `components.md` (`AccountService`, `SharingService`);
`contract-summary.md` Contracts 4/5; `unit-of-work.md` U11;
`requirements.md` (inception); `crates/cityloom-design-storage/` (U10, the
already-built sibling crate this Unit calls in-process).

This Unit is a new workspace crate, `cityloom-accounts-sharing`, a server
binary (not compiled to WASM) exposing an `axum::Router` merged into the
same shared Railway service's composition root design-storage's own
`cityloom-design-storage::build()` already establishes the pattern for
(`logical-components.md` LC-3). It owns two components, `AccountService`
and `SharingService`. Applicable Testing Contract layers: "Data model /
database behavior" (five tables: `account`, `session`, `access_grant`,
`share_link`, `saved_design`), "Repository / data access" (the
`repository` module's `sqlx` queries), "Business logic" (session/password
handling, the access gate, the deletion cascade, sharing-state
computation), and "API / endpoint" (the `handlers` module's routes). No
"Frontend behavior" layer applies — this Unit has no UI.

**Table name note**: `access_grant`, not `grant` — `grant` is a reserved
PostgreSQL keyword (`infrastructure-specification.md`'s review R-01
finding). Every migration, query, and Rust identifier in this plan uses
`access_grant` for the SQL table; the design-level `Grant` entity name is
unaffected (Rust/design identifiers aren't SQL-reserved).

## ⚠ Flagged cross-unit gap — needs Plan Approval attention

`design-storage`'s (U10) `repository` module exposes exactly one read
function, `select_by_anonymous_id(pool, anonymous_design_id)`
(`crates/cityloom-design-storage/src/repository.rs:76`) — keyed on the
*anonymous* identifier from Contract 2's public upload/fetch path. This
Unit's `BR8.1` design (`SharingService` reads `DesignRepository` directly,
in-process, after authorizing a Grant/ShareLink) needs to fetch a
`StoredDesign` by its **`stored_design_id`** (the opaque primary key
`SavedDesign`/`Grant`/`ShareLink` all reference) — no such lookup exists
in U10's crate today.

U10's `code-generation` stage is already terminal (READY, reviewed,
`unit complete`d). Adding a new function to its `repository.rs` from
*this* Unit's code-generation would cross this project's per-unit source
ownership convention (`source-manifest.json` claims one Unit's paths only)
and could invalidate U10's own review receipt for that file.

**This plan proceeds on the assumption that adding one small, additive,
read-only function to `cityloom-design-storage::repository`
(`select_by_stored_design_id`, mirroring `select_by_anonymous_id`'s exact
shape — same expiry-exclusion logic, same error mapping) is in scope for
this Unit's code-generation, since it is the one piece of new surface
U10's own crate must expose for U11 to exist at all** — the alternative
(U10 re-opening its own already-reviewed code-generation stage to add it
itself) is a bigger, stage-crossing operation for a two-function addition.
This needs explicit human confirmation at Plan Approval; if declined, the
plan changes to Option B (U10's code-generation is reopened first,
out of this Unit's scope) before Step 1 begins.

## Story/AC traceability

| Plan step | Story / AC | Rule |
|---|---|---|
| Steps 3-5 | (schema only; no direct AC) | `entities.md` `Account`/`Session`/`SavedDesign`/`Grant`/`ShareLink` shapes |
| Steps 6-8 | (repository only; no direct AC) | Repository functions backing BR1-BR9 |
| Steps 9-11 | AC9.1.1, AC9.1.2 | BR1.1 |
| Steps 9-11 | AC9.1.3 | BR2.1 |
| Steps 9-11 | AC9.1.4 | BR3.1 |
| Steps 9-11 | AC9.2.1, AC9.2.2, AC9.2.3 | BR4.1, BR4.2 |
| Steps 9-11 | AC9.3.1, AC9.3.3, AC9.3.4 | BR5.1, BR5.2, BR5.4 |
| Steps 12-14 | AC9.3.2 | BR5.3 |
| Steps 12-14 | AC10.1.1 | BR6.1 |
| Steps 12-14 | AC10.2.1, AC10.2.2, AC10.2.3, AC10.2.4 | BR7.1, BR7.2, BR7.3 |
| Steps 12-14 | AC10.1.2, AC10.1.3 | BR8.1 (this Unit decides AND enforces — see flagged gap above for the `DesignRepository` read) |
| Steps 12-14 | AC10.3.1, AC10.3.2, AC10.3.3 | BR9.1, BR9.2 |
| Steps 15-17 | NFR3.5.1 (account-deletion cascade, no direct AC — U12 `data-rights` triggers it) | `reliability-design.md` RD-2 |
| All | NFR6.1.2, NFR6.2.1-NFR6.2.8, NFR4 | `security-design.md` SD-1 through SD-8 |
| All | NFR1.5, NFR2.4, NFR3.2.2 | `performance-design.md` PD-1 through PD-3, `scalability-design.md` SC-1 through SC-3 |
| All | NFR7.5 | `observability-design.md` OD-1 through OD-3 |

## Steps

- [ ] Step 1: Project structure and production configuration skeleton.
      Create `crates/cityloom-accounts-sharing/` as a new workspace
      member: `Cargo.toml` (name `cityloom-accounts-sharing`;
      dependencies matching `cityloom-design-storage`'s exact pins —
      `axum =0.8.9`, `tokio =1.53.1`, `tower-http =0.7.1`,
      `sqlx =0.9.0` (postgres/runtime-tokio/tls-rustls/macros/migrate/time
      features, no ORM), `serde =1.0.229`, `serde_json =1.0.151`,
      `thiserror =2.0.20`, `tracing =0.1.44`,
      `tracing-subscriber =0.3.23`, `time =0.3.55` — plus `argon2`
      (password hashing, `tech-stack-decisions.md`) and a CSPRNG source
      for session/link tokens (`rand`, OS-seeded); `cityloom-api-types`
      (workspace path); `cityloom-design-storage` (workspace path — the
      new in-process dependency, `logical-components.md` LC-2, already
      declared in `unit-of-work-dependency.md`)), `src/lib.rs` with the
      module layout (`handlers`, `repository`, `account`, `sharing`,
      `access_gate`, `failure`, `observability`, per
      `logical-components.md` LC-1), and a `migrations/` directory. Add
      the crate to the workspace root `Cargo.toml`'s `members` list.
      Create the initial migration file
      (`migrations/0001_create_accounts_sharing_tables.sql`) creating the
      five tables (`account`, `session`, `access_grant`, `share_link`,
      `saved_design`) with exactly the columns `entities.md` names, plus
      the unique index on `account.email`, `saved_design (account_id,
      name)` (AC9.3.3), and `share_link.token`
      (`infrastructure-specification.md` ID-21).
      **First: add `select_by_stored_design_id` to
      `crates/cityloom-design-storage/src/repository.rs`** per the
      flagged cross-unit gap above (mirrors `select_by_anonymous_id`
      exactly: same expiry-exclusion `WHERE` clause, same
      `StorageFailure` error mapping, same `Option<StoredDesign>`
      return type) — this is the one piece of U10's crate this Unit's
      design requires and U10's crate does not yet expose.
- [ ] Step 2: Bootstrap the minimal test runner/configuration and record
      the exact unit-scoped command. `cargo test -p cityloom-accounts-sharing`
      is the unit-scoped command. Reuse the same local Postgres instance
      `cityloom-design-storage`'s own test setup already established
      (`team.md`'s TDD posture — no mocking the database); this Unit's
      migration runs against the same database, a separate schema
      namespace is not needed since table names don't collide. Run
      `cargo sqlx prepare` once the first query exists. Record both the
      test command and the (already-running, shared) database setup in
      `unit-test-instructions.md`.
- [ ] Step 3: Data model / database behavior — Red: write the failing
      tests for `Account`, `Session`, `SavedDesign`, `Grant`, `ShareLink`
      (per `entities.md`): construction, and a schema-shape test per
      table asserting the migrated columns match `entities.md` exactly.
      Record the failing command output.
- [ ] Step 4: Data model / database behavior — Green: run the migration
      against the local test database and implement the five types.
      Pass Step 3's tests.
- [ ] Step 5: Data model / database behavior — Refactor: tidy while green.
- [ ] Step 6: Repository / data access — Red: write the failing tests for
      the `repository` module's query functions (account insert/select by
      email, session insert/select-by-hash/delete, saved_design
      insert/select-by-account/select-by-id with duplicate-name conflict
      as zero-rows-affected per SD-6's precedent, access_grant
      insert/select-by-design/select-by-grantee, share_link
      insert/select-by-token/toggle-enabled, and the account-deletion
      cascade as one transaction across all five tables) against the
      real local database. Record the failing command output.
- [ ] Step 7: Repository / data access — Green: implement the `repository`
      module's query functions using `sqlx::query!`/`query_as!`
      compile-time-checked macros with bound parameters only
      (`security-design.md` SD-7). Pass Step 6's tests.
- [ ] Step 8: Repository / data access — Refactor: tidy while green.
- [ ] Step 9: Business logic — Red: write the failing tests for: password
      hashing (`argon2`, constant-time verify, NFR6.2.1), session token
      generation/hashing (SHA-256 of a 256-bit random value, NFR6.2.1),
      the fail-closed access-gate check (`config.access_gate_enabled
      .unwrap_or(false)` — unset AND explicit-false both refuse,
      NFR6.2.5/NFR6.2.7 — SD-1), `ShareLink` token entropy (128-bit
      CSPRNG, statistically-unrelated-tokens test, NFR6.2.4), and the
      `failure` module's typed-error mapping (no driver text leaked,
      NFR6.2.6). Record the failing command output.
- [ ] Step 10: Business logic — Green: implement the `account`, `sharing`,
      `access_gate`, and `failure` modules. Pass Step 9's tests.
- [ ] Step 11: Business logic — Refactor: tidy while green.
- [ ] Step 12: API / endpoint — Red: write the failing tests for the
      `handlers` module's routes, driven as HTTP requests against the
      merged `axum::Router` with the real local test database:
      `POST /api/accounts` (sign-up, optional device-design migration
      payload, BR1.1/BR4.1/BR4.2, AC9.1.1/AC9.2.1-3) —
      `Set-Cookie` carries `HttpOnly`/`Secure`/`SameSite` (NFR6.2.2);
      `POST /api/sessions` (sign-in, BR1.1, AC9.1.2);
      `DELETE /api/sessions` (sign-out — deletes the `Session` row; the
      very next request with that token is refused, NFR6.2.8);
      `POST /api/saved-designs` (save-with-name, BR5.1/BR5.2,
      AC9.3.1/AC9.3.3); `GET /api/saved-designs` (list, BR5.4/BR9.1/BR9.2,
      AC9.3.4/AC10.3.1-3); `GET /api/saved-designs/{id}` (reopen, BR5.3,
      AC9.3.2); `POST /api/saved-designs/{id}/grants` (named grant,
      BR7.1/BR7.2, AC10.2.1/AC10.2.3); `PUT`/`DELETE
      /api/saved-designs/{id}/share-link` (enable/disable, BR6.1/BR7.2/
      BR7.3, AC10.1.1/AC10.2.2-4); `GET /api/shared/{token-or-session}`
      (Contract 5's public/authenticated read — authorize via Grant or
      ShareLink, THEN call `cityloom_design_storage::repository
      ::select_by_stored_design_id` in-process, never construct the
      response before the authorization check returns `Ok`,
      BR8.1/NFR6.2.3, AC10.1.2/AC10.1.3). Every route first checks
      the access-gate flag (BR2.1/NFR6.2.5/NFR6.2.7, AC9.1.3) — a test
      with the flag unset asserts every route refuses. A log-capture
      test asserts no credential/token/identifier field appears in any
      emitted row. A readiness test simulates a database outage and
      asserts the shared `/readyz` reports not-ready. Record the failing
      command output.
- [ ] Step 13: API / endpoint — Green: implement the `handlers` module and
      the composition-root constructor function (`logical-components.md`
      LC-3 — one public function taking the shared `PgPool` and returning
      the configured `axum::Router`). Pass Step 12's tests.
- [ ] Step 14: API / endpoint — Refactor: tidy while green.
- [ ] Step 15: Business logic (deletion cascade) — Red: write the failing
      tests for the account-deletion cascade function (an internal-only
      capability `data-rights`/U12 will call, not exposed as a route in
      this Bolt — RD-2/NFR3.5.1): a successful cascade removes the
      `Account` row and every `Session`/`SavedDesign`/`Grant`
      (as grantee)/`ShareLink` row referencing it, atomically in one
      transaction; a forced mid-transaction failure leaves every row
      intact (no partial deletion). Record the failing command output.
- [ ] Step 16: Business logic (deletion cascade) — Green: implement the
      cascade function as one `sqlx::Transaction`. Pass Step 15's tests.
- [ ] Step 17: Business logic (deletion cascade) — Refactor: tidy while
      green.
- [ ] Step 18: Environment/build configuration. Confirm
      `cargo build -p cityloom-accounts-sharing` and
      `cargo build -p cityloom-design-storage` (the Step 1 repository
      addition), `cargo clippy --all-targets -- -D warnings`, and
      `cargo fmt --check` are clean across both crates. Confirm
      `cargo sqlx prepare` produces a `.sqlx/` cache with no stale
      entries for both crates. Confirm no WASM-target crate appears in
      this crate's dependency graph (server-only, `logical-components.md`
      LC-2). Confirm `cargo-llvm-cov` for `cityloom-accounts-sharing`
      clears the 80% line-coverage floor.
- [ ] Step 19: Documentation and traceability. Write `code-summary.md`,
      `source-manifest.json` (claiming this Unit's own crate paths AND
      the one added `cityloom-design-storage/src/repository.rs` function
      per the flagged gap — a cross-unit write, disclosed explicitly),
      and `traceability.json`. Update `docs/dependencies.md` with the new
      crate's additional dependencies (`argon2`, `rand`) and their
      licences.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "for every unit of work, write the failing test — or the",
  "scope": "feature",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: for every unit of work, write the failing test — or the\n  executable form of the stated acceptance criterion — before writing the\n  implementation that satisfies it, including at the osm2streets adapter\n  boundary, where the expected output is committed as a fixture (see below)\n  before the adapter code that must reproduce it is written against that\n  fixture."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Meet an 80% line-coverage floor.",
      "Run the selected tests in CI before merge."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ]
  },
  "input_sha256": "sha256:ed4bdcf58363e72548317cd6aa9374f8d2b8448e61759898d0eacfa80c588f09",
  "contract_sha256": "sha256:154405be1006cdb8702ac3d4735e1c9938381c308660d7eb2fc281364e7e3f86"
}
```

## Assumptions & Open Questions

- **[assumption]** The flagged cross-unit gap above (adding
  `select_by_stored_design_id` to `cityloom-design-storage`) is treated as
  in-scope for this Unit's code-generation. Needs explicit Plan Approval
  confirmation — this is the one point in the plan where a human should
  look twice before generation starts.
- **[assumption]** Session tokens and `ShareLink` tokens both use a
  128-bit-or-greater CSPRNG value (`rand::rngs::OsRng` or equivalent) per
  the infrastructure-design review's R-02 note naming the entropy source
  explicitly at code-generation time.
- **[assumption]** The account-deletion cascade (Step 15-17) is
  implemented as an internal Rust function only, not exposed as an HTTP
  route in this Bolt — `data-rights`/U12 will call it when that Unit's own
  code-generation runs. This carries forward the disclosed functional-design
  gap already accepted at `nfr-requirements` (NFR3.5.1 has no BR behind it
  in this Unit's own `rules.md` — the human already chose "accept and move
  on" for that gap).
