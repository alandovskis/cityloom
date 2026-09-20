# Code Generation Plan — `design-storage` (U10)

Upstream inputs: `entities.md`, `rules.md`, `functional-spec.md`
(functional-design, this Unit); `performance-design.md`,
`security-design.md`, `scalability-design.md`, `reliability-design.md`,
`observability-design.md`, `logical-components.md` (nfr-design, this
Unit); `tech-stack-decisions.md` (nfr-requirements, this Unit);
`infrastructure-specification.md`, `cicd-pipeline.md` (infrastructure-design,
this Unit); `components.md` (`DesignRepository`); `contract-summary.md`
Contract 2; `unit-of-work.md` U10; `requirements.md` (inception).

This Unit is a new workspace crate, `cityloom-design-storage`, a server
binary (not compiled to WASM) exposing an `axum::Router` merged into the
shared Railway service's entry point (`logical-components.md` LC-3's
composition root). It owns one component, `DesignRepository`. Applicable
Testing Contract layers: "Data model / database behavior" (the
`stored_designs` schema and migration), "Repository / data access" (the
`repository` module's `sqlx` queries), "Business logic" (`rate_limit` and
`sweep` modules, and the failure-mapping logic), and "API / endpoint"
(the `handlers` module's three routes). No "Frontend behavior" layer
applies — this Unit has no UI.

## Story/AC traceability

| Plan step | Story / AC | Rule |
|---|---|---|
| Steps 3-5 | (schema only; no direct AC) | `entities.md` `StoredDesign`/`UploadReceipt`/`StorageFailure` shapes |
| Steps 6-8 | AC8.3.3 (atomic failure), NFR3.4.1 | BR6.1 |
| Steps 9-11 | AC8.3.1, AC8.3.2, AC8.3.5 | BR1.1, BR7.1 |
| Steps 9-11 | AC11.3.1, AC11.3.2, AC11.3.3 | BR2.1 |
| Steps 9-11 | (rate limiting, no direct AC) NFR5.3.1, NFR6.4.1 | BR4.1 |
| Steps 12-14 | AC8.3.4 | BR5.1 |
| Steps 12-14 | AC10.1.2, AC10.1.3 | BR3.1 |
| Steps 12-14 | AC8.3.1 (the 201/409 upload responses) | BR1.1, `security-design.md` SD-6 (409 via rows-affected, not a caught error) |
| All | NFR6.4.1-6.4.4, NFR6.3.1, NFR5.3.1 | `security-design.md` SD-1 through SD-9 |
| All | NFR1.4.1-1.4.4, NFR3.4.1-3.4.2 | `performance-design.md` PD-1 through PD-5, `reliability-design.md` RD-1 through RD-4 |
| Steps 13-17 (repair, R-02) | NFR7.4.2 | `observability-design.md` OD-2 — `uploads_total` split success/per-`StorageFailure`-reason, `fetches_total` split success/403/404/429, `deletes_total` split success/429, `stored_rows_count` gauge split anonymous/account, sampled on the sweep's own cycle. Implemented in `src/observability.rs::Counters`, wired into `src/handlers.rs` at each outcome point and into `src/sweep.rs::run_forever`'s cycle; tested directly against `Counters`/`AppState`, the same pattern `crates/osm-extract-proxy/src/counters.rs` already established for this workspace (no `/metrics` HTTP route, matching that precedent). |
| Step 13 (repair, R-01) | (rate limiting resource-exhaustion, no direct AC) NFR6.4.1 | `security-design.md` SD-2, BR4.1 — `RateLimiter::sweep()` is now wired into a periodic task `lib.rs::build()` spawns, so the in-memory identifier map no longer grows unbounded for the life of the process; tested end-to-end through the wired-up `build()` path (`src/lib.rs`'s own `tests` module), not only against the bare `RateLimiter` type. |

## Steps

- [x] Step 1: Project structure and production configuration skeleton.
      Create `crates/cityloom-design-storage/` as a new workspace member:
      `Cargo.toml` (name `cityloom-design-storage`; dependencies: `axum`
      pinned `=0.8.9`, `tokio` pinned `=1.53.1` (both matching
      `osm-extract-proxy`'s exact pins, `tech-stack-decisions.md`), `sqlx`
      (`postgres`, `runtime-tokio`, `tls-rustls` features, no ORM),
      `cityloom-api-types` (workspace path, extended with this Unit's
      request/response types), `cityloom-design-payload` (workspace path,
      for `DesignPayload::from_json`'s version check), `thiserror`,
      `tracing` + `tracing-subscriber` (pinned versions matching
      `osm-extract-proxy`)), `src/lib.rs` with the six-module layout
      (`handlers`, `repository`, `rate_limit`, `sweep`, `failure`,
      `observability`, per `logical-components.md` LC-1), and a
      `migrations/` directory. Add the crate to the workspace root
      `Cargo.toml`'s `members` list (confirm `["crates/*"]` already covers
      it by inspection). Create the initial migration file
      (`migrations/0001_create_stored_designs.sql`) creating the
      `stored_designs` table with exactly the columns `entities.md`
      names for `StoredDesign` (`stored_design_id`, `anonymous_design_id`
      UNIQUE, `owner_account_id` nullable, `payload`, `created_at`,
      `expires_at`) plus a unique index on `anonymous_design_id`
      (`infrastructure-specification.md` ID-15, additive-only).
- [x] Step 2: Bootstrap the minimal test runner/configuration and record
      the exact unit-scoped command. `cargo test -p cityloom-design-storage`
      is the unit-scoped command. Since `sqlx`'s compile-time query
      checking and this Unit's own repository tests need a real database
      (`team.md`'s TDD posture — no mocking the database), set up a local
      Postgres instance for development/test (matching
      `cicd-pipeline.md`'s fast-tier Postgres service container) and run
      `cargo sqlx prepare` to generate the initial `.sqlx/` offline cache
      once the first query exists (`infrastructure-specification.md`
      ID-16). Record both the test command and the local-database setup
      steps in `unit-test-instructions.md`.
- [x] Step 3: Data model / database behavior — Red: write the failing
      tests for `StoredDesign`, `UploadReceipt`, and `StorageFailure`
      (per `entities.md`): construction, `StorageFailure`'s `reason` enum
      matching Contract 2's `ApiError` reason values exactly, and a
      schema-shape test asserting the migrated table has exactly the
      columns `entities.md` names and no more (NFR6.3.1's "no
      identifying field" as a structural assertion, `security-design.md`
      SD-7). Record the failing command output.
- [x] Step 4: Data model / database behavior — Green: run the migration
      against the local test database and implement the three types.
      Pass Step 3's tests.
- [x] Step 5: Data model / database behavior — Refactor: tidy while green.
- [x] Step 6: Repository / data access — Red: write the failing tests for
      the `repository` module's three query functions (insert, select-by-id
      excluding expired rows, delete-by-id) against the real local
      database: an insert then select round-trips the payload unchanged; a
      select for an expired row (`expires_at` in the past) returns
      not-found even though the row still physically exists (BR2.1's
      "removed once `expires_at` passes" — read as "never returned" at
      the query level, with the sweep as the eventual physical cleanup,
      `performance-design.md` PD-1); a delete of a non-existent id
      succeeds without error (idempotent delete, BR5.1); a second insert
      with a conflicting `anonymous_design_id` reports zero rows affected
      rather than an error (`security-design.md` SD-6, matching
      `performance-design.md` PD-1's `ON CONFLICT DO NOTHING` mechanism).
      Record the failing command output.
- [x] Step 7: Repository / data access — Green: implement the `repository`
      module's three query functions using `sqlx::query!`/`query_as!`
      compile-time-checked macros with bound parameters only
      (`security-design.md` SD-5). Pass Step 6's tests.
- [x] Step 8: Repository / data access — Refactor: tidy while green.
- [x] Step 9: Business logic — Red: write the failing tests for the
      `rate_limit` module (the two-fixed-window map keyed on
      `anonymousDesignId`, 10/minute and 100/hour per
      `infrastructure-specification.md` ID-19: under threshold passes,
      at threshold returns `rate_limited` with a computed `Retry-After`,
      `security-design.md` SD-2), the `failure` module's `StorageFailure`
      mapping (every `sqlx::Error` variant maps to `internal` with no
      driver text leaked, per SD-6's first half), and the payload
      validation logic (`payloadVersion` check via
      `cityloom-design-payload::DesignPayload::from_json`, and the
      byte-length size cap check against `infrastructure-specification.md`
      ID-20's 5 MiB value, both before any database call — BR6.1,
      `security-design.md` SD-3). Record the failing command output.
- [x] Step 10: Business logic — Green: implement `rate_limit`, `failure`'s
      mapping function, and the validation logic. Pass Step 9's tests.
- [x] Step 11: Business logic — Refactor: tidy while green.
- [x] Step 12: API / endpoint — Red: write the failing tests for the
      `handlers` module's three routes, driven as HTTP requests against
      the merged `axum::Router` with the real local test database:
      `POST /api/designs` — a valid upload returns `201` with an
      `UploadReceipt` (AC8.3.1); a conflicting identifier returns `409`
      (BR1.1); an oversized or `payloadVersion`-invalid payload returns
      `413`/`400` and leaves no row (BR6.1, AC8.3.3); rate-limited
      returns `429` with `Retry-After`. `GET /api/designs/{id}` — a
      valid, unexpired, matching id returns `200` with the unchanged
      payload; a missing or expired id returns `404` (BR3.1, AC10.1.2);
      the response body of a `404` contains no payload field
      (AC10.1.3, NFR6.4.2); rate-limited returns `429`.
      `DELETE /api/designs/{id}` — always returns `204`, whether the row
      existed or not (BR5.1, AC8.3.4); rate-limited still applies
      (BR4.1). A log-capture test across one of each failure reason
      asserts no identifier, payload, or requester-identifying field
      appears in any emitted row (NFR7.4.1, `security-design.md` SD-7).
      A readiness test simulates a database outage and asserts `/readyz`
      (the shared endpoint, extended per `reliability-design.md` RD-2)
      reports not-ready while the process keeps running (NFR7.4.4).
      Record the failing command output.
- [x] Step 13: API / endpoint — Green: implement the `handlers` module and
      the composition-root constructor function (`logical-components.md`
      LC-3 — the one public function taking a `PgPool` and returning the
      configured `axum::Router` plus the spawned sweep task's
      `JoinHandle`). Pass Step 12's tests.
- [x] Step 14: API / endpoint — Refactor: tidy while green.
- [x] Step 15: Business logic (sweep) — Red: write the failing tests for
      the `sweep` module: a sweep cycle deletes expired anonymous rows in
      bounded batches (`performance-design.md` PD-3's 500-row batches,
      `infrastructure-specification.md` ID-18) and never touches an
      account-owned row (`owner_account_id` set, AC11.3.2); a sweep
      emits exactly one aggregate row per cycle with the three fields
      `rows_examined`/`rows_deleted`/`duration_ms` and no row identifiers
      (NFR7.4.3, `observability-design.md` OD-3); a skipped/delayed sweep
      cycle (simulated by advancing past two intervals before the first
      run) still deletes correctly and never deletes a row early
      (NFR3.4.2, `reliability-design.md` RD-3). Record the failing
      command output.
- [x] Step 16: Business logic (sweep) — Green: implement the `sweep`
      module as an in-process `tokio` background task (6-hour interval,
      `infrastructure-specification.md` ID-18), spawned once by the
      composition-root constructor from Step 13. Pass Step 15's tests.
- [x] Step 17: Business logic (sweep) — Refactor: tidy while green.
- [x] Step 18: Environment/build configuration. Confirm
      `cargo build -p cityloom-design-storage`,
      `cargo clippy -p cityloom-design-storage --all-targets -- -D warnings`,
      and `cargo fmt --check` are clean. Confirm `cargo sqlx prepare`
      produces a `.sqlx/` cache with no stale entries (every query
      checked against the current migration). Confirm no
      `cityloom-street-import`, `cityloom-design-editing`, or any
      WASM-target crate appears in this crate's dependency graph (this
      Unit is server-only, per `logical-components.md` LC-2).
- [x] Step 19: Documentation and traceability. Write `code-summary.md`,
      `source-manifest.json`, and `traceability.json`. Update
      `docs/dependencies.md` with the new crate's dependencies (`sqlx`
      plus the reused `axum`/`tokio`/`thiserror`/`tracing` pins already
      documented for `osm-extract-proxy`) and their licences.

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
      "text": "- **Methodology**: tdd\n- **Ordering**: for every unit of work, write the failing test — or the\n  executable form of the stated acceptance criterion — before writing the\n  implementation that satisfies it, including at the osm2streets adapter\n  boundary, where the expected output is committed as a fixture (see below)\n  before the adapter code that must reproduce it is written against that\n  fixture. This is the strictest of the options put to the interview and was\n  chosen deliberately over the mixed test-first-for-the-model /\n  spike-then-characterise-for-the-boundary approach the quality review\n  proposed; see `evidence.md` for that tension and why the human's explicit\n  choice stands.\n- **Coverage floor**: the org default's 80% line-coverage floor for `feature`\n  scope applies, measured with **`cargo-llvm-cov`** over a **stated set** of\n  Rust crates/modules in the Cargo workspace: the street/lane domain model\n  crate, the provenance model, the osm2streets adapter module, the\n  persistence layer, and the editing state machine. Because the client\n  itself compiles to WASM, there is no separate \"WASM glue\" category to\n  exclude — the whole client is WASM, and the denominator is simply the set\n  of crates/modules named above. The view/rendering layer (whichever DOM\n  framework Domain Design selects — see the open constraint below) and\n  visual-identity assets leave the denominator through an explicit,\n  committed `cargo-llvm-cov` ignore pattern — never by lowering the number.\n  Branch coverage on the adapter and the lane/provenance model is\n  **reported**, not gated, until a real number exists to set a floor against\n  at `nfr-requirements`.\n- **Every defect gets a failing regression test that reproduces it before the\n  fix lands.** Worth more here than on a team: it is what stops the same\n  osm2streets edge case returning in three months once the context that found\n  it the first time is gone.\n- **Merge gate — all of the following pass in CI before squash-merge to\n  `main`**, run through the `justfile`/`scripts/verify.sh` gate above:\n  1. `cargo build --workspace` and `cargo check --workspace` clean.\n  2. `cargo clippy --workspace --all-targets -- -D warnings` clean (clippy\n     denies warnings; nothing merges with an unresolved lint).\n  3. `cargo fmt --all -- --check` clean.\n  4. Unit and component tests green (`cargo test --workspace`).\n  5. Coverage floor met on the measured set above (`cargo-llvm-cov`).\n  6. The osm2streets adapter's golden-fixture suite green (see below).\n  7. Keyboard-path and accessibility tests green, for any Bolt touching the\n     editing surface.\n- **Two CI tiers, so the gate stays fast**: a fast tier on every push runs\n  1–6; a slow tier on PRs to `main` and nightly runs the browser-mode\n  keyboard/accessibility tests (item 7) plus the full release-mode WASM\n  build of the workspace — which now includes compiling the pinned\n  osm2streets crate dependency from source as an ordinary part of the Cargo\n  build, not a separate binding step. The build is cached by Cargo's own\n  incremental/target-directory caching keyed on `Cargo.lock`, never rebuilt\n  from scratch per push.\n- **The osm2streets adapter is characterised, not trusted.** Capture real\n  osm2streets output for a small, committed set of real OSM extracts (a\n  well-tagged street, a thinly-tagged street, a one-way, a street with a\n  separately mapped cycleway, one intersection) as committed fixture files,\n  and test everything above the adapter against those fixtures — never\n  against a live OSM fetch at test time, which would make the suite\n  non-deterministic and\n  impolite to a free public API. This suite's job is to tell the builder the\n  day the dependency's behaviour moves underneath them, not to prove\n  osm2streets correct.\n- **Provenance is tested, not just rendered.** For any Bolt touching import or\n  the editor, the test floor includes at least one test asserting that a\n  thinly-tagged fixture yields an `inferred` value, and one asserting the UI\n  renders it distinguishably from a `mapped` one. This is what makes Q3's\n  success criterion checkable rather than aspirational.\n- **Accessibility testing is two distinct things, not one axe-core check.**\n  Automated tooling (axe-core, driven from Playwright) is the floor for\n  contrast, names, roles and landmarks in the surrounding UI — it cannot see\n  inside a `<canvas>` element at all, so it is not evidence about the editing\n  surface itself if that surface turns out to be canvas-rendered. The\n  per-action check is a keyboard-only interaction test for every editing\n  action (select a lane, change its type, change its width, extend along the\n  corridor, undo), asserting both the resulting model state and the announced\n  accessible name/state. A defined manual screen-reader walkthrough of the\n  full edit path happens before each stage's release. **WCAG 2.1 AA is not\n  fully CI-verifiable**; a green pipeline is not read as conformance.\n- **Open constraint on Domain Design — more load-bearing now, not less, under\n  Rust/WASM (Q5 defers the editing-surface choice; it does not remove this\n  consequence):** the affirmation-gate decision to build the client in Rust\n  compiled to WebAssembly does not force a raster-canvas editing surface.\n  Rust/WASM UI frameworks split on exactly this axis: Leptos renders real DOM\n  nodes through fine-grained reactivity with no virtual DOM, and Yew and\n  Dioxus also render real DOM nodes, through virtual-DOM diffing — none of\n  the three requires drawing to `<canvas>`. So Q5's choice remains live and\n  is now a framework-selection question for Domain Design as much as a\n  rendering-technique one. If Domain Design nonetheless selects a raster\n  canvas for the interactive lane elements — in any of these frameworks, or\n  by hand-rolling WASM-driven canvas drawing — automated accessibility\n  verification of the editing surface itself is not available: axe-core\n  cannot inspect canvas contents, and WCAG 2.1 AA conformance there can only\n  be checked by the manual screen-reader pass, every release, indefinitely.\n  A DOM-rendering choice (Leptos, Yew, or Dioxus used in its normal mode)\n  makes each lane a real, focusable, announceable element and keeps the\n  automated floor meaningful; a canvas choice does not, regardless of which\n  language drew the canvas. This is not a decision made here; it is the cost\n  Domain Design is choosing against if it picks canvas.\n- No snapshot tests of rendered geometry — pure maintenance cost for one\n  builder, and they fail on every legitimate visual change, training the\n  habit of regenerating snapshots without reading them.\n- No performance/load testing without a stated NFR target — none is fixed yet\n  (`constraint-register.md`), and load-testing a single-instance $5/month app\n  against no target is waste. One cheap budget guard instead: assert a\n  ceiling on the built client bundle and the WASM artifact size in CI, which\n  protects the hosting budget (OC-4) and phone usability at once.\n- The Test Strategy Standard's \"E2E skipped unless NFR requirements exist\"\n  does **not** apply to the keyboard/accessibility browser tests above — WCAG\n  2.1 AA is itself a committed non-functional requirement, and a real browser\n  is its only validation method.\n- **Not yet resolved, carried to Requirements Analysis rather than decided\n  here**: whether the \"week to under a day\" workflow-time claim\n  (`initiative-brief.md`) becomes an acceptance criterion. `raid-log.md` R-4\n  records it as unvalidated, with no baseline and no user timed yet. It stays\n  a hypothesis until a real baseline exists; a later stage should not be able\n  to invent a threshold nobody measured."
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
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:ed4bdcf58363e72548317cd6aa9374f8d2b8448e61759898d0eacfa80c588f09",
  "contract_sha256": "sha256:154405be1006cdb8702ac3d4735e1c9938381c308660d7eb2fc281364e7e3f86"
}
```

**Applicable layers for this Unit**: "Data model / database behavior",
"Repository / data access", "Business logic" (split across two plan
segments — validation/rate-limiting/failure-mapping, and the sweep task —
both genuinely "business logic" per `logical-components.md`'s module
split), and "API / endpoint". No "Frontend behavior" layer applies —
this crate is server-only and has no UI.

## Assumptions & Open Questions

- **[assumption]** The exact local-database bootstrap mechanism for
  development/test (a Docker Compose Postgres, a locally installed
  Postgres, or a lightweight embedded alternative) is left to the
  developer agent's judgment at Step 2 — this plan only requires that
  whichever mechanism is chosen produces a reachable database before the
  first Red test, consistent with `team.md`'s "no mocking the database"
  practice and `cicd-pipeline.md`'s own CI-side Postgres service
  container.
- **[assumption]** `cargo sqlx prepare`'s exact invocation and the
  `.sqlx/` cache's initial commit happen once real queries exist (Step 7
  onward) — Step 1 only creates the migration and module skeleton, not
  the offline cache itself, since there are no queries yet to prepare
  against.

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-20T03:15:00Z
**Iteration:** 2
**Request Challenge:** review:34c52a4ec7bd5343affab69c3ace5868

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `crates/cityloom-design-storage/src/rate_limit.rs` vs `src/lib.rs` `build()` and `src/sweep.rs` | `RateLimiter::sweep()` was never called from production code — only from its own test module — so the in-memory rate-limiter map grew unbounded for the process lifetime, exactly the resource-exhaustion risk BR4.1/SD-2 exist to prevent. | Verified fixed. `lib.rs::build()` now spawns a dedicated `tokio::time::interval` task (`rate_limiter_sweep_handle`) that calls `limiter.sweep(Instant::now())` on `config.rate_limiter_sweep_interval` (production default: hourly). The task closes over `state.limiter_arc()` — an `Arc` clone of the exact same `RateLimiter` the `upload`/`fetch`/`remove` handlers check against (`AppState::limiter_arc`), confirmed by reading `handlers.rs::AppState` — not a second, disconnected instance. `grep -rn "\.sweep("` finds exactly one production call site (`lib.rs:141`) plus three test-only call sites in `rate_limit.rs`'s own `#[cfg(test)]` module. `lib.rs::tests::build_wires_a_periodic_rate_limiter_sweep_that_keeps_the_map_bounded` drives ~40 distinct identifiers through `AppState`'s real check path (reached via `build()`, not a bare `RateLimiter`), asserts the tracked-identifier count peaks near 40, then asserts it shrinks after the wired-up periodic sweep has had time to run with shrunk test windows. Ran this test directly (`cargo test -p cityloom-design-storage`, live `DATABASE_URL`): passes, alongside all other 45 tests, 46/46, reproduced across multiple full runs. `RateLimiter::with_windows`'s window-duration parameterization (production keeps real minute/hour via `DesignStorageConfig::production_defaults`; only the test shrinks them) is a legitimate testability seam, not a production behavior change. | Resolved |
| R-02 | Major | `code-summary.md` claim vs `src/observability.rs` `Counters` vs NFR7.4.2 / OD-2 | NFR7.4.2 (aggregate counters: uploads success/failure-by-reason, fetches success/403/404/429, deletes success/429, stored_rows_count split anonymous/account) was a confirmed requirement silently dropped from this Unit's traceability and not implemented; `code-summary.md` falsely claimed it was delivered. | Verified fixed. `observability.rs::Counters` now carries `uploads_success`, a `[AtomicU64; 6]` `uploads_failure_by_reason` array indexed by every `FailureReason` variant (plus an additive `uploads_conflict` counter for the BR1.1 409 case, which correctly has no `StorageFailure` reason of its own), `fetches_success`/`fetches_403`/`fetches_404`/`fetches_429`, `deletes_success`/`deletes_429`/`deletes_failure` (R-03), and a `stored_rows_anonymous`/`stored_rows_account` gauge pair. Read `handlers.rs` line by line: every outcome branch in `upload`, `fetch`, and `remove` calls the matching `state.counters.record_*` before returning — confirmed these are live call sites, not dead code, by exercising them through `cargo test` (the `observability.rs` unit tests plus `handlers.rs`'s HTTP-level tests both pass). `stored_rows_count` is sampled once per expiry-sweep cycle via `sweep.rs::sample_stored_rows_count`, which calls the new `repository::count_stored_rows_by_ownership` (confirmed both `count(*)` queries use no request-derived interpolation — `WHERE anonymous_design_id IS NOT NULL` / `WHERE owner_account_id IS NOT NULL`, no bound parameters needed since neither predicate takes external input) and is itself called from `run_forever` after every successful cycle — not a dead/unused function, confirmed by `sweep::tests::a_sweep_cycle_samples_stored_rows_count_into_the_shared_counters` passing against a live database. `traceability.json`'s `upstream_ids`/`coverage` now list `NFR7.4.2` with `status: "OK"` pointing at `src/observability.rs`, and the plan's Story/AC table (line 37) documents the implementation and its precedent (`osm-extract-proxy/src/counters.rs`'s no-`/metrics`-route pattern). `code-summary.md` accurately describes what was built (no overstated claims found on inspection). | Resolved |
| R-03 | Minor | `crates/cityloom-design-storage/src/observability.rs` `Counters::record_delete` | `deletes_success` was incremented on both genuine success and real repository failure, hiding a healthy-vs-failing delete rate. | Verified fixed. A new `deletes_failure: AtomicU64` field and `record_delete_failure()` method exist; `handlers.rs::remove`'s `Err(failure)` arm now calls `state.counters.record_delete_failure()` (line 339) instead of folding into success, confirmed by reading the call site and by `observability::tests::delete_counters_track_success_429_and_failure_independently` passing. | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo build -p cityloom-design-storage` | Clean | Confirmed working build. |
| `cargo clippy -p cityloom-design-storage --all-targets -- -D warnings` | Clean | No lint findings. |
| `cargo fmt -p cityloom-design-storage -- --check` | Clean (no output) | Formatting matches `rustfmt`. |
| `DATABASE_URL=... cargo test -p cityloom-design-storage` (multiple runs against a live local Postgres 16 container) | 46 passed, 0 failed, every run, including `tests::build_wires_a_periodic_rate_limiter_sweep_that_keeps_the_map_bounded` | Matches `code-summary.md`'s "46 passed" claim exactly; independently reproduced, not just trusted. |
| `SQLX_OFFLINE=true cargo sqlx prepare --check -- --tests` (run from the crate's own directory, per `cargo-sqlx`'s per-crate cache convention) | Clean | Committed `.sqlx/` cache is current, including the two new `count_stored_rows_by_ownership` queries and the test-only account-row insert. (An earlier attempt using `--workspace` mode from the workspace root reported a stale-cache error — that is a different, workspace-level cache path this crate does not use, not a defect in the committed per-crate cache; the correct per-crate check is clean.) |
| `cargo llvm-cov -p cityloom-design-storage --summary-only` (live `DATABASE_URL`) | TOTAL 96.16% region / 96.02% line / 94.18% function | Matches `code-summary.md`'s reported numbers exactly, digit for digit. Clears `team.md`'s 80% line-coverage floor with wide margin; improved from iteration 1's 93.91%/93.34%/91.89%. |
| `grep -rn "\.sweep(" crates/cityloom-design-storage/src/` | One production call site (`lib.rs:141`, inside the spawned periodic task) plus three test-only sites in `rate_limit.rs` | Confirms R-01's fix wires a real production call site, not only another test. |
| `git status --porcelain -- crates/cityloom-design-storage` | `?? crates/cityloom-design-storage/` (whole directory untracked, consistent with a not-yet-committed new crate) | No drift outside what `source-manifest.json` claims; nothing unexpected. |
| Reading `handlers.rs::AppState` and `lib.rs::build()` together | `state.limiter_arc()` and the router's own `state.0.limiter` are `Arc::clone`s of the one `RateLimiter` constructed in `build()` | Confirms the sweep task and the handlers share state — not two disconnected limiters. |
| `repository.rs::count_stored_rows_by_ownership` source read | Both queries are fixed literal SQL via `sqlx::query_scalar!` with no interpolated or bound request-derived values | Confirms SD-5 is not violated by the new query; no driver-error-text leak risk since `from_sqlx_error` is still the sole error-mapping path. |

### Summary

Both Major findings from iteration 1 are genuinely resolved, not merely asserted: the rate-limiter sweep is wired into `build()`'s composition root, shares the live `RateLimiter` instance with the handlers, and is exercised end-to-end by a new test that passed on independent re-run; NFR7.4.2's full counter shape is implemented, wired into every real outcome point in `handlers.rs`, sampled by the sweep cycle through a new SQL-injection-safe query, reflected accurately in `traceability.json` and `code-summary.md`, and covered by passing tests. The optional R-03 fix is also in place and tested. All validation commands were re-run independently against a live local Postgres instance rather than trusted from the developer's report, and every reported number (46 tests, coverage percentages) matched exactly. No new defects were found in the changed surface (rate limiter wiring, counters, sweep-cycle sampling, the new repository query). Zero Critical, zero Major, zero unresolved Minor findings remain.
