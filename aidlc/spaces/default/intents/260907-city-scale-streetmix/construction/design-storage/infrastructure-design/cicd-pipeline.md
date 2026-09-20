# CI/CD Pipeline — `design-storage` (U10)

Upstream inputs: `security-design.md`, `reliability-design.md`,
`performance-design.md`, `scalability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-specification.md`
(this stage), `osm-extract-proxy`'s own cicd-pipeline.md (U9), `team.md`
and `project.md` (practices).

Design elements are numbered `CP-n`. `team.md` already fixed the
pipeline's gates — the two CI tiers, the merge gate's seven items,
`gitleaks`, `cargo audit`, SonarQube, Dependabot security-only, deploy on
merge, rollback through Railway's deployment history — and U9's own
`cicd-pipeline.md` already fixed the code-path shape (feature branch →
fast tier; pull request → both tiers; squash-merge → push to `main` →
Railway build → health-check-gated rollover). This file states what this
Unit adds to each tier and to the deploy path, and the one genuinely new
thing this Unit introduces: a database migration step ahead of rollover.
The workflow files, the migration files and the `Dockerfile` changes are
code generation's.

## One path into production, now with a migration step

```
 code path (shared with U9, extended)
 -------------------------------------
 feature branch --push--> fast tier (now includes cityloom-design-storage's
        |                             own build/test/clippy/fmt, plus a
        |                             Postgres service container for tests)
 pull request -----> fast + slow tiers
        |               (merge gate)
 squash-merge to main
        |
 push to main --> fast tier (re-run)
        |
 Railway build (Dockerfile, unchanged — no new image stage)
        |
 process starts --> sqlx::migrate! runs against DATABASE_URL
        |                     |
        |                 fails? -----> process exits non-zero; Railway
        |                               marks the deploy Failed; previous
        |                               deployment keeps serving (CP-3)
        |                     |
        |                 succeeds
        |                     v
 /readyz health check (now also pings the database) --fail--> previous
        |                                                       deployment
        |                                                       keeps serving
 rollover; SIGTERM old (5 s drain, unchanged from U9)
        |
 uptime monitor (U9's own, unchanged) sees the new deployment as up
```

<!-- Text fallback: the code path is unchanged from U9's own pipeline up
through the squash-merge and Railway build steps; this Unit adds its own
crate to the fast tier's build/test/clippy/fmt, and the fast tier now runs
a Postgres service container so this Unit's tests can hit a real database.
After the Railway build, the process now runs its sqlx migrations against
DATABASE_URL before the shared health check is evaluated; a migration
failure exits the process non-zero, which Railway treats as a failed
deployment, leaving the previous deployment serving. The health check
itself now also pings the database. Rollover and the uptime monitor are
unchanged from U9's own design. -->

## Stage-to-gate mapping (CP-1)

| Stage | Runs on | Gate (what must pass) | On failure |
|---|---|---|---|
| **Fast tier** | Every push to any branch, including `main` | U9's own six items (build, check, clippy, fmt, test, coverage), now also covering `cityloom-design-storage`'s modules per `logical-components.md` LC-1 (this Unit's own coverage contribution to NFR7.1.1); `gitleaks`; the dependency/asset manifest check — **plus a `postgres` service container** (the same major version targeted at `environment-provisioning`) so this Unit's repository/handler tests run against a real database rather than a mock, consistent with `team.md`'s TDD posture and this Unit's own security-design.md SD-5 (compile-time-checked queries need a real schema to check against, whether live or via the committed `.sqlx/` offline cache) | The push is red; nothing merges |
| **Slow tier** | Pull requests to `main`, and nightly | U9's own items (release-mode WASM build, browser accessibility tests, `cargo audit`, SonarQube, bundle-size guard) — **plus this Unit's own CI benchmark**: a representative multi-street corridor `DesignPayload` driven through upload/fetch/remove against the fast tier's Postgres container, failing on p95/p99 above `performance-requirements.md`'s NFR1.4.1–NFR1.4.3 targets, and **a migration-idempotency check**: running `sqlx::migrate!` twice against a fresh database and asserting the second run is a no-op | The pull request cannot merge |
| **This Unit's own tests, in the fast tier** | Every push | The tests the NFR designs name: atomic-write test (no partial row on a simulated mid-transaction failure, NFR3.4.1); rate-limiter window tests at both thresholds (ID-19, NFR5.3.1); payload-size and `payloadVersion` rejection tests, asserting no row is created either way (NFR6.4.3); the object-level authorization test (a 403/404 response body contains no payload field, NFR6.4.2); the two-designs-same-payload-different-identifiers test (NFR6.4.1); the log-capture test asserting no identifier, payload, or requester-identifying field in a failure row (NFR7.4.1, this Unit's own SD-7 discipline); the sweep-cycle test asserting the exact three-field row shape — `rows_examined`, `rows_deleted`, `duration_ms` (NFR7.4.3, `observability-design.md` OD-3); the skipped-sweep-cycle test asserting no early deletion (NFR3.4.2); the readiness-under-simulated-database-outage test (NFR7.4.4, NFR3.2.1) | Red; `team.md`'s regression rule (NFR7.3.1) applies to every defect these find |
| **Merge** | Squash-merge of the pull request to `main`, one commit per Bolt named by the Bolt slug | Both tiers green; branch rules require the pull request and the checks (unchanged from U9) | — |
| **Build** | Railway, on the push to `main`, from the **same, unmodified** `Dockerfile` U9 committed | The binary now links `cityloom-design-storage` as an additional workspace crate; no new build stage — this Unit adds a dependency to the existing binary, not a new image layer *(checked 2026-09-14 by U9's own infrastructure-specification.md for the base build; this Unit's own addition changes only the crate graph, not the Dockerfile's stages)* | Deployment failed; the previous deployment keeps serving, per U9's own checked claim |
| **Migrate** (**new, CP-1a**) | The process, at start, before accepting `/api/designs*` traffic | `sqlx::migrate!` applies any migration not yet recorded in the schema's own migration-tracking table; on a fresh database this creates the `stored_designs` table (ID-15) | The process exits non-zero; Railway marks the deployment `Failed`; the previous deployment (which has no dependency on this Unit's schema existing, since it is the same binary version or older) keeps serving — the same "previous deployment survives a failed build/start" property U9's own `infrastructure-specification.md` handoff item 5 confirmed, now exercised by a migration failure rather than a corrupted data asset |
| **Deploy** | Railway | `GET /readyz` returns 2xx within U9's own 120 s timeout — now also requiring a successful database ping (`reliability-design.md` RD-2) in addition to U9's own manifest/cell verification | The new deployment never takes traffic; U9's own MD-2 notification fires |
| **Rollover** | Railway | Unchanged from U9's own design (5 s drain) | — |
| **Post-deploy check** | U9's own external uptime monitor's next probe | 200 on `/readyz`, now implying both U9's own readiness and this Unit's database reachability | U9's own down notification (MD-1) fires; the maintainer reads the migration/failure logs to tell the two apart |

## Deployment strategy (CP-2)

**Unchanged from U9's own design**: rolling, one replica, health-check
gated. This Unit adds no new deployment strategy question — it rides the
same single-replica rollover U9's own `cicd-pipeline.md` CP-2 already
designed, with the one addition that the health check's own definition
(RD-2) now includes a database ping. Because Stage 1 runs exactly one
replica, no two instances ever attempt to run this Unit's migrations
concurrently (`infrastructure-specification.md`'s own stated assumption).

## Rollback (CP-3)

| What went wrong | Rollback | Time |
|---|---|---|
| A code deploy is bad after it passed the health check | Railway's deployment history: redeploy the previous deployment (unchanged from U9) | Inside RTO's ≤ 10 minutes (U9's own RD-9) |
| A code deploy fails its health check (including a failed database ping) | Nothing to do: the previous deployment never stopped serving | 0 |
| **A migration fails to apply** (new to this Unit) | Nothing to do: the process that failed to migrate never started serving, so the previous deployment (running against the same or an older, compatible schema, per `infrastructure-specification.md` ID-15's additive-only discipline) keeps serving; fix the migration file and push a new commit through the ordinary pull-request path | 0 for service continuity; the time to fix and re-merge the migration is separate |
| **A migration applies but is semantically wrong** (e.g. a bad default value) — a genuinely new rollback class this Unit introduces that U9's own Unit (no schema) never had | A forward-fixing migration, committed and merged through the ordinary path — Stage 1 has no down-migration tooling, matching this Unit's own additive-only discipline (there is nothing generated to roll a `CREATE TABLE`/`CREATE INDEX` backward that also preserves data) | One deploy cycle; named here as a genuine limitation rather than assumed away |
| The Postgres plugin itself becomes unreachable (a platform incident) | Nothing this Unit's own code can do — this is this Unit's own design stance, held by analogy to (not stated by) U9's own NFR3.1.11 ("The platform is down... Nothing this Unit can do; the downtime counts against NFR3.1.1"); this Unit's own `reliability-requirements.md` NFR3.1.1 only fixes the 99.5% availability target this downtime counts against, the same way U9's downtime counts against its own NFR3.1.1 | — |

Unlike U9's own Unit, this Unit's rollback story does have a data
consequence for the semantically-wrong-migration case — recorded above as
a genuine, not-yet-fully-tooled limitation rather than silently treated as
equivalent to U9's own "always just run the previous image" claim.

## Secrets in CI/CD (CP-5)

| Where | Secret | Design |
|---|---|---|
| The service | `DATABASE_URL` (Railway-injected, never committed) — this Unit's one new secret in the whole workspace | `security-design.md` SD-8; `gitleaks` and GitHub push protection (already enabled by U9) cover the repository; the connection string never becomes a file in it |
| The CI workflows | The fast tier's Postgres service container uses a fixed, non-secret test-only password (a CI-local database, never the production one) — no new repository secret | This Unit introduces no new CI-level credential; the offline `.sqlx/` cache (ID-16) is committed, non-secret, and lets the fast tier compile even without the Postgres container reachable at the compile step |
| Railway | No project token anywhere, unchanged from U9's own design; the Postgres plugin is created and linked through Railway's own UI/CLI at `environment-provisioning`, not through a pipeline step | Same "authenticates to nothing extra" preference `infrastructure-specification.md`'s rejected-alternatives already state for the expiry sweep |

## Artifact management (CP-6)

| Artifact | Where | Identity | Retention |
|---|---|---|---|
| The service image | Railway's build cache and deployment history (unchanged from U9) | The git commit SHA | Railway's deployment history |
| **This Unit's migration files** | The repository, `crates/cityloom-design-storage/migrations/` | Sequential, timestamped filenames (`sqlx`'s own convention) | Git history; never edited once merged — a schema change is always a new migration file, never an edit to an old one, so the migration-tracking table's recorded history stays truthful |
| **The `.sqlx/` offline query cache** | The repository, alongside the crate | Regenerated by `cargo sqlx prepare` whenever a query changes | Git history; a stale cache that no longer matches a query is a compile-time error in the fast tier, so drift cannot silently ship |
| **The Postgres data itself** | The Railway-managed plugin's own storage | Not git-tracked — this is the one artifact in this Unit's pipeline that is not reproducible from the repository | Railway's own plugin backup/retention policy, confirmed at `environment-provisioning` (not yet checked — see Assumptions) |

## Environment promotion (CP-7)

Unchanged from U9's own design: one environment, promotion is the merge.
This Unit adds no new promotion step — a pull request touching
`cityloom-design-storage` passes through the same two tiers as any other
change in the workspace.

## Assumptions & Open Questions

- The fast tier's Postgres service container (GitHub Actions' own
  `services:` feature) starts fast enough to not measurably slow the fast
  tier's overall run time; if it does, the container is scoped to only
  the jobs that need it (this Unit's own test job) rather than the whole
  fast-tier workflow. [assumption]
- Railway's managed Postgres plugin has its own backup/retention policy
  on the Hobby tier; not yet checked, and named here rather than assumed,
  for confirmation at `environment-provisioning`. [assumption]
- `sqlx::migrate!` failing mid-way through a multi-statement migration
  leaves the migration-tracking table in a state the next deploy attempt
  can safely retry from (this is `sqlx`'s own documented transactional
  migration behavior for a single migration file); confirmed by doing
  the first time a migration actually needs more than one statement.
  [assumption]

## Traceability

See `traceability.json` in this directory.

_Confirmed._

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-19T04:33:59Z
**Iteration:** 1
**Request Challenge:** review:bf06dc2516c467597b4c09f0333cb580

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `cicd-pipeline.md` "This Unit's own tests, in the fast tier" row (CP-1), versus `observability-design.md` OD-3 and `observability-requirements.md` NFR7.4.3 | This row says the fast tier runs "the sweep-cycle test asserting the exact five-field row shape (NFR7.4.3)". Both `observability-design.md` OD-3 ("Each sweep cycle... emits exactly one row — `rows_examined`, `rows_deleted`, `duration_ms` — never the identifiers of the rows it deleted") and `observability-requirements.md` NFR7.4.3 ("rows examined, rows deleted, duration — never the deleted rows' identifiers") define the sweep row as having exactly **three** named fields, and `monitoring-design.md`'s own Metrics & KPIs table ("Sweep `rows_examined` / `rows_deleted` / `duration_ms`") repeats the same three-field shape a second time. No artifact in this Unit's design chain — nfr-design, nfr-requirements, or either of the other two infrastructure-design files — names a fourth or fifth field for this row. "Five-field" is therefore internally contradicted by every other place the row's shape is stated, and names no fields for a developer to write the "exact" assertion the test description promises. | Change "five-field" to "three-field" (or state the row's field list explicitly, matching OD-3/NFR7.4.3) so the test description matches the one row shape defined everywhere else. | Resolved — the row now reads "the sweep-cycle test asserting the exact three-field row shape — `rows_examined`, `rows_deleted`, `duration_ms` (NFR7.4.3, `observability-design.md` OD-3)", matching every other statement of this row's shape. |
| R-02 | Minor | `cicd-pipeline.md` Rollback table (CP-3), "The Postgres plugin itself becomes unreachable" row | The row attributes the "nothing this Unit's own code can do... platform outage, no in-Unit treatment" position to *this Unit's own* `reliability-requirements.md` NFR3.1.1. Read directly, this Unit's NFR3.1.1 only states an availability SLO target ("This Unit's endpoints target the same 99.5% monthly availability as the rest of the single Railway service... measured at the shared service level") — it does not itself say "nothing can be done" about a platform outage. That specific "no in-Unit treatment" reading is the one `osm-extract-proxy`'s own NFR3.1.11 ("The platform is down... Nothing this Unit can do; the downtime counts against NFR3.1.1") states explicitly, which this row does correctly reference by analogy ("the same reading U9's own NFR3.1.11 already established"), but the citation is worded as if this Unit's own NFR3.1.1 carries that statement, which it does not. | Reword to make clear the "no in-Unit treatment" position is this Unit's own design stance (consistent with, and justified by analogy to, U9's NFR3.1.11), rather than implying this Unit's own NFR3.1.1 states it directly. | Resolved — the row now states this is this Unit's own design stance held "by analogy to (not stated by) U9's own NFR3.1.11", quoting NFR3.1.11's text directly, and clarifies this Unit's own NFR3.1.1 only fixes the availability target the downtime counts against. |
| R-03 | Minor | `traceability.json` `upstream_ids`, versus `performance-requirements.md`'s prose | Running `aidlc-sensor-traceability.ts --output-path traceability.json --stage infrastructure-design` returns `missing_from_upstream_ids":["NFR1.1"]`. This is the same residual false-positive already identified and accepted as non-blocking at this Unit's `nfr-design` stage (`security-design.md`'s own `## Review`, finding R-04): `performance-requirements.md` mentions `NFR1.1` only to state it does **not** apply to this Unit's three endpoints (it is U9's own budget for a full osm2streets import). Adding `NFR1.1` to this file's `upstream_ids` would misrepresent this Unit as covering another Unit's requirement, so the gap is a known sensor artifact rather than a real coverage hole. | No action required for readiness; carried forward as the same accepted non-blocking observation already on record at `nfr-design` for this Unit. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --output-path traceability.json --stage infrastructure-design` | `{"pass":false,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":["NFR1.1"],"invalid_entries":[],"invalid_targets":[],"findings_count":1}` | Every real `NFRx.y.z` id this Unit's infrastructure design covers resolves cleanly; the one flagged id (`NFR1.1`) is the known false-positive addressed in R-03. |
| `aidlc-sensor-required-sections.ts --output-path cicd-pipeline.md` | `{"pass":true,"h2_count":9,...}` | Well-formed H2 structure; no template configured for this stage, so this confirms structure only, not content completeness. |
| `aidlc-sensor-upstream-coverage.ts --output-path cicd-pipeline.md` | `{"pass":true,"consumes":[],"unreferenced":[],"scanned_files":[],"reason":"no upstream","findings_count":0}` — vacuous (no `consumes:` frontmatter configured for this file) | Cross-checked manually instead: every upstream file cicd-pipeline.md's header lists (`security-design.md`, `reliability-design.md`, `performance-design.md`, `scalability-design.md`, `observability-design.md`, `logical-components.md`, `functional-spec.md`, `components.md`, `contract-summary.md`, `infrastructure-specification.md`, U9's own `cicd-pipeline.md`, `team.md`, `project.md`) is referenced by name or by design-element id at least once in the body. No orphaned upstream reference found. |
| Manual: cost-arithmetic re-derivation, `infrastructure-specification.md` "Cost estimate" | PostgreSQL compute/memory ≤ $1.00 + storage ≤ $0.25 = **$1.25** fixed for this Unit (matches the stated total); $1.25 (U10) + $2.25 (U9's own checked fixed terms) = **$3.50**, inside the $5 hard limit with $1.25 of stated headroom remaining even before crediting U9's own $0.25 egress reservation | Arithmetic is internally correct; no repeat of the U9 precedent's original cost-summation error (that error was already found and fixed in U9's own iteration-1 review). |
| Manual: cross-file consistency — CP-3 rollback table vs. `infrastructure-specification.md` ID-15's additive-only migration discipline | Consistent: CP-3's "migration fails to apply" and "migration applies but is semantically wrong" rows both rely on, and correctly restate, ID-15's additive-only discipline and the absence of down-migration tooling | No contradiction found here. |
| Manual: cross-file consistency — `monitoring-design.md`'s "no new alert" claim vs. `cicd-pipeline.md`'s new "Migrate" step | Consistent: the Migrate step's failure mode is folded into the *existing*, reused "Deployment failed (MD-2)" alert ("a migration failure at start-up (ID-15) is one new way this can now happen, alongside U9's own causes"), not a new alert condition | No contradiction found; `monitoring-design.md`'s claim holds given what the new step can actually fail at. |
| Manual: spot-check of the sibling `osm-extract-proxy` integration point this Unit's design names directly — `osm-extract-proxy/nfr-requirements/reliability-requirements.md` NFR3.1.11 | Text confirmed to exist verbatim as cited ("The platform is down... Nothing this Unit can do; the downtime counts against NFR3.1.1") | Confirms the substance behind R-02's citation is sound; only the attribution in `cicd-pipeline.md`'s own row is imprecise. |

### Summary

The pipeline design is implementable: the code path, the new Migrate step, the rollback table and the reuse of U9's shared Railway service, `Dockerfile`, `/readyz` and alerts are all internally consistent, the cost arithmetic checks out by hand, and the one genuine cross-Unit citation this file makes (U9's NFR3.1.11) resolves to real, matching text. One Major finding blocks nothing on its own but is a real, checkable contradiction worth fixing before code generation: the fast-tier sweep-cycle test is described as asserting a "five-field" row shape that no artifact in the chain — including this Unit's own `monitoring-design.md`, written by the same stage — actually defines; every other mention of this row names exactly three fields. Two Minor findings (an imprecise NFR citation in the rollback table, and a known non-blocking traceability-sensor false-positive already on record from this Unit's `nfr-design` review) round out the findings. With zero Critical and one Major, the artifact is READY.
