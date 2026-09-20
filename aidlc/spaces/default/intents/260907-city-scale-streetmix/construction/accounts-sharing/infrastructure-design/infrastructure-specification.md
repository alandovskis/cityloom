# Infrastructure Specification — `accounts-sharing` (U11)

_Confirmed (consolidated summary re-confirmed 2026-09-20 after review-repair)._

Upstream inputs: `performance-design.md`, `security-design.md`,
`scalability-design.md`, `reliability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-design-questions.md`
(this stage), `design-storage`'s own `infrastructure-specification.md` (U10,
this workspace's shared-service-with-database precedent), `team.md` and
`project.md` (practices).

Design elements are numbered `ID-n`. This Unit's infrastructure footprint
is almost entirely additive to what `design-storage` already provisioned:
no new compute service, no new database — only this Unit's own migration
file(s), a new access-gate configuration value, and its share of the
existing CI/CD pipeline.

## What is placed, and where

```
 GitHub (public repository)                   Railway (Hobby, us-east4)
 +-----------------------------+              +---------------------------+
 | main --------- push ------->|------------->| service: cityloom         |
 |  |                          |  GitHub app  |  (shared with U9/U10/U12) |
 |  | migrations/*.sql         |              |  binary now also mounts   |
 |  | (this Unit's own)        |              |  /api/account* /api/share*|
 |  |                          |              |  routes; runs this Unit's |
 |  |                          |              |  migrations at start too  |
 +-----------------------------+              +------------+--------------+
                                               | PostgreSQL (Railway       |
                                               |  managed plugin — SAME    |
                                               |  plugin U10 provisioned,  |
                                               |  new tables, no new DB)   |
                                               +------------+--------------+
                                               connection: DATABASE_URL
                                               (already injected for U10,
                                               reused unchanged)
```

<!-- Text fallback: the same public GitHub repository and Railway service
`cityloom` that U9/U10 already established gain this Unit's contribution —
new account/session/sharing routes merged into the shared router
(logical-components.md LC-3) and this Unit's own SQL migration files, run
once at process start alongside U10's. No new Railway resource: this Unit
reuses the same Railway-managed PostgreSQL plugin U10 already provisioned,
adding its own tables to the same database rather than a second database. -->

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | This Unit's router merges into the **same** Railway service `cityloom` U9/U10 already share (**ID-1**, unchanged) — no new compute service | `unit-of-work.md`'s "four Units, one deployable"; `infrastructure-design-questions.md` Q1 |
| Database | **Reuses** the existing Railway-managed PostgreSQL plugin U10 provisioned (**ID-13**, unchanged) — this Unit adds five new tables (`account`, `session`, `access_grant`, `share_link`, `saved_design`) to the same database, not a new plugin | `scalability-design.md` SC-3 (shared `PgPool`, no per-Unit pool); `reliability-design.md` RD-2 (the account-deletion cascade needs single-database transactional atomicity — a second database would break this outright); `infrastructure-design-questions.md` Q2 |
| Network path to the database | Unchanged from U10's own ID-14 — Railway's internal private network, the already-injected `DATABASE_URL`; no public endpoint | No new decision needed; this Unit's queries use the same pool U10's composition root already constructs |
| Schema migration | A second, independent set of `sqlx` migration files (`migrations/*.sql` in `cityloom-accounts-sharing`), applied at the same process-start point as U10's, via the same `sqlx::migrate!` call against the shared migration-tracking table (**ID-21**) | Extends U10's own ID-15 discipline; `infrastructure-design-questions.md` Q3 — no ordering dependency on U10's migrations, since neither schema references the other's tables (confirmed by inspection: `crates/cityloom-design-storage/migrations/0001_create_stored_designs.sql` creates only `stored_designs`, with no inbound foreign key from this Unit's tables) |
| Migration compatibility with rollback | Every migration in Stage 1 is additive only (`CREATE TABLE`, `CREATE INDEX`, `CREATE UNIQUE INDEX` — no destructive statement) (**ID-21**) | Same reasoning as U10's own ID-15: keeps `cicd-pipeline.md` CP-3's "redeploy the previous image" rollback path working |
| `sqlx` build-time query checking | This Unit's own `.sqlx/` offline query cache, committed alongside `cityloom-design-storage`'s (**ID-22**) | `security-design.md` SD-7 (compile-time-checked queries); same mechanism U10's own ID-16 established, this Unit's own cache entries |
| Connection pool sizing | **Shared** `PgPool` (`max_connections = 10`, U10's own ID-17) — this Unit adds no new pool and requests no dedicated connection ceiling | `scalability-design.md` SC-3; a second pool against the same database would double-count Railway's connection ceiling for no benefit |
| Account-deletion cascade transaction | `RD-2`'s five-table `DELETE` cascade runs as one Postgres transaction against the shared connection pool — no new infrastructure, a query pattern only | `reliability-design.md` RD-2; this is a code-generation concern once the tables exist, not a new resource |
| Access-gate configuration | A new Railway service variable (`ACCOUNTS_SHARING_ENABLED` or equivalent), **absent = off** (fail-closed, `security-design.md` SD-1) (**ID-23**) | `infrastructure-design-questions.md` Q5; `project.md`'s Mandated access-gate rule — this is the literal Railway-level mechanism that rule requires |
| Configuration as code | The new access-gate flag (ID-23) plus any future numeric thresholds this Unit needs — same typed, fail-fast, non-secret discipline as U9/U10's own service variables (**ID-7**, extended again) | `security-design.md` SD-8 |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Railway service `cityloom` (ID-1, shared with U9/U10) | compute | Unchanged, except the router now also serves this Unit's account/session/sharing routes and runs a second migration set at start | No new compute service; resource contribution is additive to U9/U10's own figures |
| Railway PostgreSQL plugin (ID-13, shared with U10 — **not new**) | database | Unchanged plugin, five new tables added by this Unit's own migration | This Unit introduces **zero** new Railway resources — the entire infrastructure footprint is schema and configuration, not provisioning |
| Railway edge (ID-1, shared with U9/U10 — the same Railway service row above, not a separate `logical-components.md` component) | load-balancer / TLS termination / DNS | Unchanged | This Unit's routes inherit the same TLS termination and header limits already established |
| Railway log explorer (ID-24, shared with U9/U10 — a platform facility, not a `logical-components.md` component) | logs | Unchanged | This Unit's own log rows (`observability-design.md` OD-1, OD-3) land in the same stream, distinguished by field shape |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Railway service `cityloom` (process, memory, CPU, domain) | U9 (created first) | U10, **U11 (this Unit, mounts account/session/sharing routes)**, U12, U6 | This Unit's `handlers` module is the only code in this crate that touches the shared `axum::Router`; this Unit's own `Session`/`Grant` types are never passed to another Unit's code |
| Railway PostgreSQL plugin | U10 (provisioned it) | **U11 (this Unit, own tables)**, U12 (when it exists) | This Unit owns five tables (`account`, `session`, `access_grant`, `share_link`, `saved_design`) in the shared database; it reads U10's `stored_designs` table only through U10's `DesignRepository` in-process call (`logical-components.md` LC-2), never a direct cross-table SQL join |
| Spending limit (ID-9, set by U9) | U9 | Every Unit, including this one | This Unit adds no new metered resource, so no new cost line beyond negligible additional compute/memory (see Cost estimate) |
| GitHub Actions CI (fast and slow tiers) | `team.md` (workspace-wide) | Every Unit | `cicd-pipeline.md` |
| The `Dockerfile` and Railway configuration file | U9 | U10, **U11 (this Unit)**, U12, U6 | This Unit adds no new build stage — the binary gains a new crate dependency (`cityloom-accounts-sharing`) and a second migration-run call at start, not a new image layer |

## Cost estimate (extends U10's ID-13 total)

| Term | Basis | Monthly |
|---|---|---|
| New Railway resources | None — no new compute service, no new database plugin | **$0** |
| This Unit's own compute/memory contribution to the shared `cityloom` process | Argon2 hashing is deliberately more CPU-costly than a plain hash (by design, to resist offline cracking) but runs only on sign-up/sign-in, not per-request; negligible steady-state contribution at Stage 1's expected volume | **~$0** additional |
| This Unit's own storage contribution to the shared Postgres plugin | Five new tables at Stage 1 volume (accounts, sessions, grants, links, saved-design metadata — no payload bytes, those stay in `design-storage`'s own table) — a small fraction of U10's own storage estimate | **~$0** additional, folded into U10's existing ≤ $0.25/month storage term rather than a new line |
| Fixed terms, this Unit | | **$0** (this Unit adds no new fixed cost) |

Combined with U9's $2.25 and U10's $1.25 fixed terms, the workspace total
remains **≤ $3.50** of the $5 hard limit — this Unit's addition is the
first Unit in the sequence to add **zero** new fixed infrastructure cost,
since it rides entirely on what U9/U10 already provisioned. Still subject
to `project.md`'s "budget growth is a constraint change" rule if real
usage proves this estimate wrong.

## Environment-provisioning handoff

Named so `environment-provisioning` has a list rather than a search:

1. Set the new access-gate service variable (ID-23) — confirm it is
   **absent** by default in the Railway environment (not set to any
   value, including `"false"` as a string, since SD-1's mechanism is
   `Option<bool>` → `unwrap_or(false)`, and an explicit `"false"` string
   parses the same as absent, but should be verified once at setup time
   rather than assumed).
2. Confirm this Unit's migration runs cleanly against the existing
   database (no table-name collision with U10's own schema — verified by
   inspection at code-generation, re-confirmed here by running the
   migration against a copy of U10's live schema before merging to
   `main`).
3. No new plugin, no new service, no new spending-limit line to configure
   — this handoff item exists mainly to state that explicitly, since
   every other Unit's handoff list up to now has had at least one.

## Rejected alternatives

- **A dedicated Postgres plugin for this Unit's own tables.** Rejected:
  splits the account-deletion cascade (RD-2) across two databases, which
  Postgres's own transactional atomicity cannot span — the whole point of
  RD-2's design (one `BEGIN`/`COMMIT`) would need application-level
  compensation logic instead, exactly the failure class `AC11.1.4`
  ("never a half-deleted account") exists to prevent. Also doubles the
  per-plugin fixed cost for no benefit, the same reasoning U9/U10 already
  used against a second compute service.
- **A dedicated connection pool for this Unit.** Rejected per
  `scalability-design.md` SC-3 — a second pool against the same database
  instance only fragments the existing `max_connections = 10` ceiling
  U10's ID-17 already sized for the whole shared service, not per-Unit.
- **JWT-based sessions instead of DB-backed rows** (would have avoided
  needing this Unit's own `session` table at all). Already rejected at
  `nfr-requirements`/`nfr-design` for NFR6.2.8's immediate-revocation
  requirement; not re-litigated here.

## Assumptions & Open Questions

- Adding a second, independent `sqlx::migrate!` call (this Unit's own
  migration set, run alongside U10's) at process start does not
  measurably lengthen the shared service's startup time or create a
  migration-tracking-table race, since both calls run sequentially in the
  same single-replica process — confirmed at `environment-provisioning`
  by observing actual startup time after this Unit's first deploy.
  [assumption]
- No table-name collision exists between this Unit's five new tables
  (`account`, `session`, `access_grant`, `share_link`, `saved_design`) and any
  table `design-storage` or a future Unit might name — checked by
  inspection at code-generation against U10's own `migrations/` directory
  before this Unit's first migration file is written. [assumption]
- **[correction, review R-01]** The original name for this Unit's grant table,
  `grant`, is a reserved PostgreSQL keyword (`CREATE TABLE grant (...)` fails
  with a syntax error, confirmed directly against a live Postgres instance).
  Renamed to `access_grant` throughout this stage's artifacts and in
  `nfr-design/reliability-design.md` RD-2's cascade-transaction SQL. The
  design-level `Grant` entity name in `functional-design`/`nfr-design`
  (a Rust type, not a SQL identifier) is unaffected.

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._
