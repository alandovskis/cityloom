# CI/CD Pipeline — `accounts-sharing` (U11)

_Confirmed (consolidated summary re-confirmed 2026-09-20 after review-repair)._

Upstream inputs: `security-design.md`, `reliability-design.md`,
`performance-design.md`, `scalability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-specification.md`
(this stage), `design-storage`'s own `cicd-pipeline.md` (U10), `team.md`
and `project.md` (practices).

Design elements are numbered `CP-n` (this Unit's own numbering). `team.md`
already fixed the pipeline's gates; U9/U10's own `cicd-pipeline.md`
already fixed the code-path shape including the Postgres-backed migration
step U10 introduced. This file states what this Unit adds — a second,
independent migration set sharing the same database and the same fast-tier
Postgres container U10 already added, not a second container.

## One path into production, now with a second migration set

```
 code path (shared with U9/U10, extended again)
 -------------------------------------
 feature branch --push--> fast tier (now also includes
        |                             cityloom-accounts-sharing's own
        |                             build/test/clippy/fmt, reusing the
        |                             SAME Postgres service container U10
        |                             already added — not a second one)
 pull request -----> fast + slow tiers
        |               (merge gate)
 squash-merge to main
        |
 push to main --> fast tier (re-run)
        |
 Railway build (Dockerfile, unchanged — no new image stage)
        |
 process starts --> U10's sqlx::migrate! runs, THEN this Unit's own
        |            sqlx::migrate! runs (independent migration sets,
        |            no ordering dependency between them per
        |            infrastructure-design-questions.md Q3 — sequenced
        |            only because they're both calls in the same startup
        |            function, not because one depends on the other)
        |                     |
        |                 either fails? -----> process exits non-zero;
        |                               Railway marks the deploy Failed;
        |                               previous deployment keeps serving
        |                     |
        |                 both succeed
        |                     v
 /readyz health check (unchanged — pings the one shared database both
        |                Units' migrations just ran against) --fail-->
        |                previous deployment keeps serving
        |
 rollover; SIGTERM old (5 s drain, unchanged)
        |
 uptime monitor (unchanged) sees the new deployment as up
```

<!-- Text fallback: the code path is unchanged from U9/U10's own pipeline
up through the squash-merge and Railway build steps; this Unit adds its
own crate to the fast tier's build/test/clippy/fmt, reusing the same
Postgres service container U10 already introduced rather than adding a
second one. After the Railway build, the process now runs two independent
sqlx migration sets at start (U10's, then this Unit's) before the shared
health check is evaluated; either migration failing exits the process
non-zero, which Railway treats as a failed deployment, leaving the
previous deployment serving. The health check itself is unchanged, since
it already pings the one shared database. Rollover and the uptime monitor
are unchanged. -->

## Stage-to-gate mapping (CP-1)

| Stage | Runs on | Gate (what must pass) | On failure |
|---|---|---|---|
| **Fast tier** | Every push to any branch, including `main` | U9/U10's own six items (build, check, clippy, fmt, test, coverage), now also covering `cityloom-accounts-sharing`'s modules per `logical-components.md` LC-1 (this Unit's own coverage contribution to NFR3.5.1's cascade test and NFR6.2.x's security tests); `gitleaks`; the dependency/asset manifest check — **reusing U10's existing `postgres` service container**, not adding a second one | The push is red; nothing merges |
| **Slow tier** | Pull requests to `main`, and nightly | U9/U10's own items (release-mode WASM build — unaffected, this is a server-only crate like U10; browser accessibility tests — unaffected, this Unit exposes no UI itself; `cargo audit`; SonarQube; bundle-size guard — unaffected, server crate) — **plus this Unit's own migration-idempotency check**: running this Unit's `sqlx::migrate!` twice against a fresh copy of the shared schema and asserting the second run is a no-op, same pattern U10's own CP-1 slow-tier row established | The pull request cannot merge |
| **This Unit's own tests, in the fast tier** | Every push | The tests the NFR designs name: fail-closed access-gate test (unset the flag entirely, assert every route refuses, NFR6.2.7); session-revocation-immediacy test (delete a session row, assert the next request with its token is refused, NFR6.2.8); password-hash constant-time-verify test (NFR6.2.1); cookie-flags test (`HttpOnly`/`Secure`/`SameSite` present on every `Set-Cookie`, NFR6.2.2); authorization-before-read test (no `DesignRepository` call before the Grant/ShareLink check returns `Ok`, NFR6.2.3); `ShareLink` token-entropy test (statistically unrelated tokens, NFR6.2.4); typed-error/no-leaked-driver-text test (NFR6.2.6); the account-deletion cascade atomicity test (RD-2/RD-3 — force a mid-transaction failure, assert zero orphaned rows across all five tables); the duplicate-name-refusal test asserting the existing `SavedDesign` row is untouched (BR5.2); the readiness-under-simulated-database-outage test (reusing U10's own pattern, since both Units share the readiness check) | Red; `team.md`'s regression rule applies to every defect these find |
| **Merge** | Squash-merge of the pull request to `main`, one commit per Bolt named by the Bolt slug | Both tiers green; branch rules require the pull request and the checks (unchanged) | — |
| **Build** | Railway, on the push to `main`, from the **same, unmodified** `Dockerfile` U9 committed | The binary now links `cityloom-accounts-sharing` as a second additional workspace crate (alongside `cityloom-design-storage`); no new build stage | Deployment failed; the previous deployment keeps serving |
| **Migrate** (**CP-1a, extending U10's own step**) | The process, at start, after U10's own `sqlx::migrate!` call, before accepting this Unit's routes' traffic | This Unit's own `sqlx::migrate!` applies any migration not yet recorded in the (shared) migration-tracking table; on a fresh database this creates the `account`/`session`/`access_grant`/`share_link`/`saved_design` tables (ID-21; `access_grant` — not `grant`, a reserved PostgreSQL keyword, see Review R-01 below) | The process exits non-zero; Railway marks the deployment `Failed`; the previous deployment keeps serving |
| **Deploy** | Railway | `GET /readyz` returns 2xx within the existing 120 s timeout — unchanged condition, since the readiness check already pings the one shared database | The new deployment never takes traffic; the existing MD-2 notification fires |
| **Rollover** | Railway | Unchanged (5 s drain) | — |
| **Post-deploy check** | The existing external uptime monitor's next probe | 200 on `/readyz` | The existing down notification (MD-1) fires; the maintainer reads the migration/failure logs to tell U10's and this Unit's migration failures apart by their distinct migration-file names |

## Deployment strategy (CP-2)

**Unchanged**: rolling, one replica, health-check gated. This Unit adds no
new deployment strategy question. Because Stage 1 runs exactly one
replica, no two instances ever attempt to run either Unit's migrations
concurrently.

## Rollback (CP-3)

| What went wrong | Rollback | Time |
|---|---|---|
| A code deploy is bad after it passed the health check | Railway's deployment history: redeploy the previous deployment (unchanged) | Inside RTO's ≤ 10 minutes |
| A code deploy fails its health check | Nothing to do: the previous deployment never stopped serving | 0 |
| **This Unit's migration fails to apply** | Nothing to do: the process that failed to migrate never started serving, so the previous deployment (running against the same or an older, compatible schema — this Unit's own migrations are additive-only) keeps serving; fix the migration file and push a new commit through the ordinary pull-request path | 0 for service continuity |
| This Unit's migration applies but is semantically wrong | A forward-fixing migration, committed and merged through the ordinary path — same no-down-migration-tooling limitation U10 already named for itself | One deploy cycle |
| The shared Postgres plugin becomes unreachable | Nothing this Unit's own code can do — same platform-incident stance U10 already holds (by analogy to `osm-extract-proxy`'s own NFR3.1.11), now also covering this Unit's own tables since they live in the same plugin | — |

## Secrets in CI/CD (CP-5)

| Where | Secret | Design |
|---|---|---|
| The service | `DATABASE_URL` (already injected for U10, unchanged) — this Unit introduces **zero** new secrets | `security-design.md` SD-8; this Unit's one new configuration value (the access-gate flag) is explicitly non-secret |
| The CI workflows | Reuses U10's existing fast-tier Postgres service container and its fixed, non-secret test-only password — no new CI-level credential | This Unit's own `.sqlx/` offline cache (ID-22) is committed, non-secret |
| Railway | No project token anywhere, unchanged | This Unit's one new environment variable (the access-gate flag) is set through Railway's own UI/CLI at `environment-provisioning`, not through a pipeline step |

## Artifact management (CP-6)

| Artifact | Where | Identity | Retention |
|---|---|---|---|
| The service image | Railway's build cache and deployment history (unchanged) | The git commit SHA | Railway's deployment history |
| **This Unit's migration files** | The repository, `crates/cityloom-accounts-sharing/migrations/` — a separate directory from U10's own, so the two Units' migration histories never collide on filename | Sequential, timestamped filenames (`sqlx`'s own convention), independent numbering from U10's | Git history; never edited once merged |
| **This Unit's own `.sqlx/` offline query cache** | The repository, alongside this Unit's own crate | Regenerated by `cargo sqlx prepare` whenever a query changes | Git history |
| The Postgres data itself | The Railway-managed plugin's own storage (shared with U10, unchanged) | Not git-tracked | Railway's own plugin backup/retention policy — already named as an open item in U10's own `cicd-pipeline.md`, not re-opened here |

## Environment promotion (CP-7)

Unchanged: one environment, promotion is the merge. A pull request
touching `cityloom-accounts-sharing` passes through the same two tiers as
any other change in the workspace.

## Assumptions & Open Questions

- Running two independent `sqlx::migrate!` calls sequentially at process
  start (U10's, then this Unit's) does not create a race or a deadlock on
  the shared migration-tracking table, since both calls happen in the
  same single-replica process, one after the other, never concurrently.
  [assumption]
- This Unit's migration filenames will not collide with U10's own,
  because `sqlx`'s migration-tracking table keys on the full filename
  (including its timestamp prefix) and each Unit's migrations live in a
  separate crate directory — confirmed by inspection at code-generation
  before the first migration file is written. [assumption]

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-20T16:08:26Z
**Iteration:** 2
**Request Challenge:** review:45f6f44f902634caca5ec8ef61ee75e6

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `infrastructure-specification.md` "Database" row (now `access_grant`, correct), and its "Assumptions & Open Questions" `[correction, review R-01]` note; `nfr-design/reliability-design.md` RD-2 line 24; `nfr-design/performance-design.md` PD-2 line 27 | The migration-governing artifacts of this stage (`infrastructure-specification.md`'s table list, `cicd-pipeline.md`'s CP-1 migrate row) are correctly renamed to `access_grant` throughout — verified by grep across `infrastructure-design/`. The disclosed gap in `reliability-design.md` RD-2 (`DELETE FROM grant WHERE ...`, line 24) is confirmed to still exist, unquoted, exactly as disclosed. It is **not the only remaining stale reference**: `nfr-design/performance-design.md` PD-2's illustrative SQL (line 27, `LEFT JOIN grant g ON g.stored\_design\_id = sd.stored\_design\_id`) also still carries the unrenamed identifier, and this occurrence was never disclosed in the brief or in `infrastructure-specification.md`'s own correction note. That note further overstates the fix: it states the rename was applied "in `nfr-design/reliability-design.md` RD-2's cascade-transaction SQL", which is checkably false — RD-2's SQL is unchanged. A developer trusting this `Confirmed` artifact's own claim would believe RD-2 was already fixed and would have no signal that PD-2 needs the same treatment. | Correct the `[correction, review R-01]` note in `infrastructure-specification.md` to accurately state that RD-2's SQL was **not** renamed (only disclosed, pending the review-freeze on that already-`READY` nfr-design stage), and add `performance-design.md` PD-2 to the same disclosed-gap list — it carries the identical stale identifier and was not previously named anywhere. If PD-2 is also blocked by the review-freeze hook, disclose it the same way RD-2 was disclosed rather than omitting it. | Unresolved |
| R-02 | Major | `infrastructure-specification.md` "Infrastructure Services" table (now cites `ID-1`/`ID-24`, correct); `infrastructure-design/monitoring-design.md` "Where each signal lands" table, "Railway log explorer" row | `infrastructure-specification.md`'s own `LC-4`/`LC-5` citations are fixed — both now resolve to `ID-1` and `ID-24`, IDs actually defined within `infrastructure-specification.md` itself (verified: `ID-1` is the Deployment table's "Compute model" row; `ID-24` is the "Railway log explorer" row in "Infrastructure Services" that itself introduces the ID). But the same broken pattern the original R-02 flagged survives, unfixed, in a sibling artifact of this same stage: `monitoring-design.md`'s "Counters — signups/signins/grants/links/gate-refusals (OD-1)" row still reads "Railway log explorer (**LC-5**, shared)" — the identical nonexistent-element citation R-02 was raised against. `monitoring-design.md`'s own header is stamped `_Confirmed (consolidated summary re-confirmed 2026-09-20 after review-repair)._`, meaning this file was touched in the same repair pass that fixed `infrastructure-specification.md`, yet this specific citation was missed. | Apply the same fix used in `infrastructure-specification.md` (cite `ID-24`, or the workspace's own stated disambiguation convention of prefixing with the owning artifact) to `monitoring-design.md`'s "Railway log explorer" row, so the fix is consistent across every artifact in this stage rather than only the one R-02's location happened to name. | Unresolved |
| R-03 | Minor | `infrastructure-specification.md` "Schema migration" row (Deployment table) | Fixed as claimed: the row now states the no-ordering-dependency claim is "confirmed by inspection: `crates/cityloom-design-storage/migrations/0001_create_stored_designs.sql` creates only `stored_designs`, with no inbound foreign key from this Unit's tables" — verified directly by reading that migration file: it contains one `CREATE TABLE stored_designs (...)`, one `CREATE UNIQUE INDEX`, and one `CREATE INDEX`, with no `REFERENCES`/foreign-key clause anywhere in the file. The claim is now grounded in a checked fact rather than asserted bare. | None — resolved. | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| Manual: `grep -rn '\bgrant\b'` across `infrastructure-design/`, `functional-design/`, `nfr-design/` (this Unit) | `infrastructure-specification.md` and `cicd-pipeline.md`'s own migration-governing text consistently use `access_grant`; the surviving unquoted-`grant`-as-SQL-identifier hits are exactly two: `nfr-design/reliability-design.md:24` and `nfr-design/performance-design.md:27` (plus the `Grant` Rust-type/prose usages in `functional-design/*`, which the correction note correctly says are unaffected) | Confirms R-01's fix landed in the artifacts that govern the actual migration and confirms the disclosed gap is real, but also surfaces a second, undisclosed occurrence that contradicts the brief's premise that RD-2 is the sole remaining stale reference |
| Manual: `nfr-design/reliability-design.md` line 24 read directly | `DELETE FROM grant WHERE account_id = $1 OR granted_to_account_id = $1;` — unchanged, unquoted | Confirms the disclosed gap exists exactly as described |
| Manual: `nfr-design/performance-design.md` lines 24–28 read directly | `SELECT sd.*, COUNT(g.grant_id) ... LEFT JOIN grant g ON g.stored_design_id = sd.stored_design_id ...` — unchanged, unquoted, same file header stamp as `reliability-design.md` (`_Confirmed (consolidated summary confirmed 2026-09-20)._`, no "after review-repair" suffix, indicating this file was not touched in the repair pass at all) | Confirms R-01 is not fully resolved: a second stale reference exists that the developer's disclosure did not name |
| Manual: `infrastructure-specification.md` "Assumptions & Open Questions" `[correction, review R-01]` note, cross-checked against the two files above | The note states the SQL was renamed "in `nfr-design/reliability-design.md` RD-2's cascade-transaction SQL" | That specific claim is false as written — RD-2's SQL is unchanged — an in-scope factual inaccuracy in a `Confirmed` artifact |
| Manual: `grep -n 'LC-4\|LC-5'` across `infrastructure-design/` | `infrastructure-specification.md` now clean (cites `ID-1`/`ID-24`); `monitoring-design.md:23` still reads `Railway log explorer (LC-5, shared)` | Confirms R-02's citation fix was applied to one artifact but not propagated to the sibling artifact carrying the same broken reference |
| Manual: `crates/cityloom-design-storage/migrations/0001_create_stored_designs.sql` read directly | One `CREATE TABLE stored_designs`, one `CREATE UNIQUE INDEX`, one `CREATE INDEX`; no `REFERENCES` clause | Confirms R-03: the "no inbound foreign key" claim now cited in `infrastructure-specification.md` is accurate |
| Manual: re-check `infrastructure-specification.md` Deployment/Shared-Infrastructure/Infrastructure-Services tables for the `access_grant` rename | `Database` row, `Infrastructure Services` implicitly, and `Shared Infrastructure` "Railway PostgreSQL plugin" row all list `access_grant`, not `grant` | Confirms the migration-ordering row and the table lists themselves (the parts of R-01's original location this Unit's own reviewer read scope covers directly) are correctly fixed |

### Summary

Two of the three prior findings show a real but incomplete fix rather than a clean resolution. R-03 is genuinely fixed. R-01's core defect — the reserved-keyword table name in the artifacts that actually govern the migration — is fixed, but the developer's own correction note misstates what was done (falsely claims RD-2's SQL was renamed) and the disclosed single remaining gap turns out to be one of two: `performance-design.md` PD-2 carries the identical unrenamed identifier and was never disclosed. R-02's citation fix was applied to `infrastructure-specification.md` but not to `monitoring-design.md`, which cites the same broken `LC-5` reference and was stamped as touched by the same repair pass without the citation actually being corrected. Neither gap breaks the build path this stage governs, but both indicate the "fix" was narrower than claimed and needs a further, more thorough pass across this stage's own artifact set before this can be called complete. With two Major findings still Unresolved, this artifact remains NOT-READY.
