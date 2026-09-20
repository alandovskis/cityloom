# Infrastructure Specification — `design-storage` (U10)

Upstream inputs: `performance-design.md`, `security-design.md`,
`scalability-design.md`, `reliability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `unit-of-work.md`
(units-generation), `infrastructure-design-questions.md` (this stage),
`osm-extract-proxy`'s own infrastructure-specification.md (U9, this
workspace's shared-service precedent), `team.md` and `project.md`
(practices).

Design elements are numbered `ID-n`; `traceability.json` maps each
infrastructure-relevant `NFRx.y` decision from the NFR design to the
resource or setting that meets it. This is a design: it names the
resources, their settings and the reasons; the migration files, the
router-merge code and the Railway configuration changes are code
generation's.

Every platform fact below not already checked by U9's own
infrastructure-specification.md is under "Assumptions & Open Questions",
flagged for confirmation at `environment-provisioning` — this Unit does
not re-verify facts U9 already checked (Railway's region set, builder
options, deployment-variable defaults) and cites them by reference
instead.

## What is placed, and where

```
 GitHub (public repository)                   Railway (Hobby, us-east4)
 +-----------------------------+              +---------------------------+
 | main --------- push ------->|------------->| service: cityloom         |
 |  |                          |  GitHub app  |  (shared with U9/U11/U12) |
 |  | migrations/*.sql         |              |  binary now also mounts   |
 |  | (this Unit's own)        |              |  /api/designs* routes     |
 |  |                          |              |  runs migrations at start |
 +-----------------------------+              +------------+--------------+
                                               | PostgreSQL (Railway       |
                                               |  managed plugin, new,     |
                                               |  this Unit's own service) |
                                               +------------+--------------+
                                               connection: DATABASE_URL
                                               (Railway-injected, internal
                                               network, no public exposure)
```

<!-- Text fallback: the same public GitHub repository and Railway service
`cityloom` that U9 already established gain this Unit's contribution — the
`/api/designs*` routes merged into the shared router (logical-components.md
LC-3) and this Unit's own SQL migration files, run once at process start. A
new Railway-managed PostgreSQL plugin is added as its own service inside the
same Railway project, reachable only over Railway's internal network via the
platform-injected DATABASE_URL — never exposed publicly. -->

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | This Unit's router is merged into the **same** Railway service `cityloom` U9 already created (`logical-components.md` LC-3's composition root) — no new compute service (**ID-1**) | `unit-of-work.md`'s "four Units, one deployable"; `team.md` Deployment: one environment, no staging tier; a second compute service would double the per-service fixed costs U9's own cost estimate already spends |
| Database | A new **Railway-managed PostgreSQL plugin**, provisioned once as its own Railway service inside the same project, distinct from the application service (**ID-13**) | `components.md`'s stated external dependency for `DesignRepository`; `reliability-design.md` RD-4 / `reliability-requirements.md` NFR3.3.1 — data must survive an application redeploy, which a managed plugin (not an attached volume on the app service) provides |
| Network path to the database | Railway's internal private network between the app service and the Postgres plugin, using the platform-injected `DATABASE_URL` service variable; no public endpoint on the Postgres plugin (**ID-14**) | `security-design.md` SD-8 (no credential committed; the connection string is never a repository file); the database has no reason to be reachable from outside Railway, and exposing it would be a new, unreviewed attack surface no requirement asks for |
| Schema migration | A committed set of `sqlx` migration files (`migrations/*.sql` in `cityloom-design-storage`), applied once at process start via `sqlx::migrate!` before the router begins accepting `/api/designs*` traffic (**ID-15**) | `reliability-design.md` RD-4's "additive, backward-compatible" migration discipline; running migrations at start (not as a separate deploy step) keeps the single-binary deploy model U9 established rather than introducing a second deploy artifact |
| Migration compatibility with rollback | Every migration in Stage 1 is additive only (`CREATE TABLE`, `CREATE INDEX` — no `ALTER ... DROP COLUMN`, no data-destructive statement) (**ID-15**) | So `cicd-pipeline.md` CP-3's "redeploy the previous image" rollback path (established by U9) still works for this Unit: an older binary run against a newer, additive-only schema is a no-op for columns it does not know about, never a crash |
| `sqlx` build-time query checking | The `.sqlx/` offline query cache is committed to the repository, generated by `cargo sqlx prepare` against a local development database and refreshed whenever a query changes (**ID-16**) | `security-design.md` SD-9: CI's fast tier must compile `cityloom-design-storage` without a reachable production database; the offline cache is `sqlx`'s own documented mechanism for this, and is the first Unit in this workspace to need it |
| Connection pool sizing | `max_connections = 10` for this Unit's `PgPool`, fixed here per `performance-design.md` PD-2's stated need for a numeric value (**ID-17**) | Railway's Hobby-tier managed Postgres plugin's own default connection ceiling is generous relative to a single-instance, single-Unit consumer at Stage 1 volume; 10 leaves headroom for a burst without a second Unit's future pool needs (U11/U12, if they also use Postgres) exhausting it on this Unit's account alone |
| Expiry sweep mechanism | An in-process `tokio` background task, spawned once by the same composition root that constructs the `PgPool` (`logical-components.md` LC-3), running every **6 hours**, each cycle deleting in batches of **500** rows (**ID-18**) | `performance-design.md` PD-3 and `reliability-design.md` RD-3 fix the *shape* (bounded batches, idempotent, interval-bounded); this stage fixes the *numbers*. An in-process task needs no new Railway resource (no cron service, no separate credential) — the same "authenticates to nothing extra" preference U9's Q1 decision already established for its own weekly job, reapplied here as "runs inside the process that already exists" rather than "runs as its own scheduled job" |
| Rate-limiter thresholds | **10 requests/minute** and **100 requests/hour** per `anonymousDesignId` (`security-design.md` SD-2), fixed here (**ID-19**) | Deliberately lower than U9's own address-keyed 30/min–300/hour: this limiter protects one specific identifier from a guessing/scraping campaign, not a general client from being throttled on ordinary use (a legitimate client fetches or removes its own design rarely, not repeatedly) |
| Payload size cap | **5 MiB** per upload, checked as a raw byte-length before deserialization (`security-design.md` SD-3), fixed here (**ID-20**) | `performance-requirements.md`'s own stated assumption expects realistic multi-street corridor designs "well under 1 MB"; 5 MiB is generous headroom above realistic usage (not tuned to it, per `scalability-requirements.md` NFR2.3.2) while still bounding worst-case storage and request-body memory use |
| Configuration as code | `DATABASE_URL` (Railway-injected, not committed), plus this Unit's own service variables for the rate-limiter thresholds, payload cap, sweep interval and batch size — same typed, fail-fast, non-secret discipline as U9's own service variables (**ID-7**, extended) | `security-design.md` SD-8; keeps every numeric decision on this row movable without a code change, consistent with U9's own established pattern |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Railway service `cityloom` (ID-1, shared with U9) | compute | Unchanged from U9's own row, except the router now also serves `/api/designs*` and runs this Unit's migrations at start | This Unit adds no new compute service; its resource contribution (memory, CPU) is additive to U9's own figures, per `logical-components.md`'s shared-resources note |
| **Railway PostgreSQL plugin (ID-13, new)** | database | Railway-managed Postgres, Hobby tier, same region as `cityloom` (`us-east4`, matching U9's ID-1 for lowest inter-service latency); one database, one schema | The only new Railway resource this Unit introduces; `infrastructure-design`'s own scope for this Unit is almost entirely "how the existing service gains a database," not a new deployment topology |
| Railway edge (LC-4, shared with U9) | load-balancer / TLS termination / DNS | Unchanged from U9's own row | The `/api/designs*` routes inherit the same TLS termination and header limits U9's SD-2 already established at the shared router level |
| Railway log explorer (LC-5, shared with U9) | logs | Unchanged from U9's own row | This Unit's own log rows (`observability-design.md` OD-1, OD-3) land in the same stream, distinguished by their own field shapes, never by a separate log destination |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Railway service `cityloom` (process, memory, CPU, domain) | U9 (created first, at U9's own environment-provisioning) | **U10 (this Unit, mounts `/api/designs*`)**, U11, U12, U6 | This Unit's handlers (`logical-components.md` `handlers` module) are the only code in this crate that touches the shared `axum::Router`; the `PgPool` this Unit constructs is never passed to another Unit's code |
| Spending limit (ID-9, set by U9) | U9 | Every Unit, including this one | This Unit's Postgres plugin cost (see "Cost estimate" below) draws on the same $5 workspace ceiling U9's own allocation already budgets against |
| GitHub Actions CI (fast and slow tiers) | `team.md` (workspace-wide practice) | Every Unit | `cicd-pipeline.md` (this Unit's own contribution to each tier is listed there) |
| The `Dockerfile` and Railway configuration file (set up by U9) | U9 | U10 (this Unit), U11, U12, U6 | This Unit adds no new build stage to the `Dockerfile` — the binary already being built gains a new crate dependency (`cityloom-design-storage`) and a migration-run step at start, not a new image layer |

## Cost estimate (ID-13, extending U9's ID-9)

Provisional figures against the remaining headroom inside the workspace's
single $5 hard spending limit, after U9's own ≤ $2.50 allocation:

| Term | Basis | Monthly |
|---|---|---|
| PostgreSQL plugin, compute + memory | Railway's smallest managed-Postgres footprint, metered the same way as the app service (rate to be confirmed at `environment-provisioning` — see Assumptions) | **≤ $1.00** (provisional) |
| PostgreSQL plugin, storage | Anonymous-upload steady state bounded by expiry (`scalability-design.md` SC-2) — a few hundred MB at realistic Stage 1 volume | **≤ $0.25** (provisional) |
| This Unit's own compute/memory contribution to the shared `cityloom` process | The crate's own handlers add negligible steady-state memory beyond the connection pool's buffers; no new CPU-bound work (no cutting, no cryptography beyond what the client already performed for the identifier) | **~$0** additional beyond U9's own figures |
| Fixed terms, this Unit | | **≤ $1.25** |

Combined with U9's own $2.25 fixed terms, the two Units together commit
**≤ $3.50** of the $5 hard limit, leaving headroom for U9's own egress
variability and any future Unit (U11, U12) that also needs a database —
still inside `project.md`'s "budget growth is a constraint change" rule
rather than assumed. The Postgres plugin's exact rate card is not yet
checked against Railway's current pricing (unlike U9's own compute/memory
rates, which its own infrastructure-specification.md already checked on
2026-09-14) — this is the first genuinely new item this workaround's cost
model has not verified, and is named explicitly under "Assumptions & Open
Questions" for confirmation before the plugin is created.

## Environment-provisioning handoff

Named so `environment-provisioning` has a list rather than a search:

1. Create the Railway PostgreSQL plugin inside the existing project (ID-13);
   confirm the platform injects `DATABASE_URL` into the `cityloom` service's
   environment automatically (Railway's documented plugin-linking behaviour;
   confirm on the day, since this Unit's own check of it is deferred here).
2. Confirm the plugin has no public network exposure by default (ID-14) —
   verify no public connection string is issued unless explicitly requested.
3. Set this Unit's own service variables (rate-limiter thresholds, payload
   cap, sweep interval/batch size — ID-19, ID-20, ID-18).
4. Run this Unit's migrations against the freshly created database once,
   confirming `sqlx::migrate!` at process start creates the schema cleanly
   on an empty database (ID-15).
5. Confirm the actual Postgres plugin cost against Railway's current rate
   card and reconcile against the provisional "Cost estimate" above; if it
   diverges materially, that reconciliation is itself the constraint-change
   trigger `project.md` requires, raised here rather than absorbed.
6. Confirm a redeploy of the `cityloom` service (a U9-only code change,
   touching nothing in this Unit) leaves the Postgres plugin and its data
   untouched — the same "redeploy is always just the previous image"
   property U9's own `cicd-pipeline.md` CP-3 established, now confirmed to
   also hold with a database attached.

## Rejected alternatives

- **An attached volume on the `cityloom` service, holding a file-based
  database (e.g. SQLite).** Forbidden by `reliability-requirements.md`
  NFR3.3.1 and this Unit's own `tech-stack-decisions.md`, which already
  fixed PostgreSQL as the chosen technology; a volume would also break
  U9's zero-downtime redeploy property (RD-4), since a new instance and
  the old one cannot share a volume safely during the rollover window.
- **A separately deployed database-access service** (a second Railway
  compute service fronting Postgres). Rejected for the same reason U9
  rejected a second compute service for itself: `unit-of-work.md`'s "one
  deployable" and the fixed per-service cost this workspace's budget
  cannot absorb twice.
- **A Railway cron service for the expiry sweep**, instead of an in-process
  `tokio` task. Rejected for the same reason U9's own Q1 rejected a cron
  service for its data build (`infrastructure-specification.md`
  "Rejected alternatives"): a second Railway resource and, in this case,
  its own database credential, for work an already-running process can do
  with a background task and its existing pool connection.
- **Exposing the Postgres plugin publicly** so a future analytics tool
  could query it directly. No requirement in this Unit or any confirmed
  Stage 1 scope asks for this, and it would be a new, unreviewed network
  attack surface `security-design.md`'s threat model does not name.

## Assumptions & Open Questions

- Railway's managed-Postgres plugin injects `DATABASE_URL` into a linked
  service's environment automatically, the same way it injects `PORT`;
  confirmed by doing at `environment-provisioning` (U9's own
  infrastructure-specification.md checked comparable platform facts on
  2026-09-14, but this specific plugin-linking behaviour was not one of
  them). [assumption]
- The Postgres plugin's compute/memory/storage rates are metered
  comparably to the app service's own rates U9 already checked
  ($10/GB-month memory, $20/vCPU-month, storage separately); the
  provisional cost estimate above is not yet checked against Railway's
  current plugin-specific pricing. [assumption]
- Railway's managed Postgres plugin does not expose a public connection
  string by default; if it does, an explicit configuration step is needed
  at `environment-provisioning` to disable public exposure before the
  plugin is ever linked to real data. [assumption]
- `sqlx::migrate!` running at process start (rather than as a separate CI/CD
  step) does not introduce a race between two instances during a rollover
  window; Stage 1 runs one replica (matching U9's own ID-1), so no two
  instances ever run migrations concurrently in practice, but this is
  worth re-confirming if a future Unit's scale needs ever motion toward
  more than one replica. [assumption]

## Traceability

See `traceability.json` in this directory.

_Confirmed._
