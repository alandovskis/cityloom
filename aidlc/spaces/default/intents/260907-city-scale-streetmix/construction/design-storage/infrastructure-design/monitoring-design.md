# Monitoring Design — `design-storage` (U10)

Upstream inputs: `observability-design.md`, `reliability-design.md`,
`performance-design.md`, `scalability-design.md`, `security-design.md` and
`logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-specification.md`
(this stage), `osm-extract-proxy`'s own monitoring-design.md (U9),
`team.md` (practices).

Design elements are numbered `MD-n`. `observability-design.md` fixed the
signals this Unit's process emits (OD-1 to OD-4); this file is the
platform side — where each signal lands, what reads it, and what (if
anything) this Unit adds to U9's own alert set. `team.md`'s bar applies
to every row: a control either notifies at the moment of the mistake with
an actionable message, or it is off. No dashboard exists.

## Where each signal lands

| Signal (from NFR design) | Emitted by | Lands in | Read by |
|---|---|---|---|
| Failure row, per failed operation (OD-1) | This Unit's `handlers`, stdout | Railway log explorer (LC-5, shared with U9) | The maintainer, filtered by `operation` and `reason` |
| Aggregate counters (uploads/fetches/deletes/stored-row-count) (OD-2) | This Unit, stdout, on the same periodic cadence U9 established | Railway log explorer | The maintainer, by differencing two rows |
| Sweep-cycle row (OD-3) | This Unit's `sweep` module, stdout, once per sweep cycle | Railway log explorer | The maintainer, watching for a growing `rows_examined` (a sign the interval is too long relative to upload volume) |
| Readiness (OD-4) | `GET /readyz` — the **same shared endpoint** U9 already exposes, extended to also ping the database | Railway's deploy-time health check; U9's own external uptime monitor (MD-1, U9) | Railway, to gate rollover; the same monitor, continuously — no new monitor for this Unit |
| Database metrics (connections, storage used) | Railway (the Postgres plugin) | Railway's service metrics, on the plugin's own service page | The maintainer, monthly, against the storage-growth bound (`scalability-design.md` SC-2) |

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Upload/fetch/remove latency p95/p99 | Latency-bucket histograms alongside OD-2's counters (`performance-design.md` PD-5) | Per `performance-requirements.md`: upload p95 ≤ 500 ms/p99 ≤ 1,500 ms; fetch and remove p95 ≤ 200 ms/p99 ≤ 800 ms | The first real numbers this Unit's provisional NFR1.4 targets are read against, same discipline as U9's own NFR1.1 read-at-B-3 pattern |
| Upload success ratio | Δ`uploads_total{result="success"}` ÷ Δ total uploads | ≥ 99% (no inception SLO exists for this specific ratio; read as a health signal, not a paged SLO) | A dropping ratio without a matching spike in a specific `reason` bucket is the signal to look at `internal` failures first (`security-design.md` SD-6) |
| Stored-row count (anonymous / account-owned) | `stored_rows_count` in OD-2 | Reviewed monthly against `project.md`'s budget-change trigger (`scalability-design.md` NFR2.3.1) | The direct evidence for whether steady-state storage is tracking the expected "uploads/day × 30" bound |
| Sweep `rows_examined` / `rows_deleted` / `duration_ms` | OD-3, once per cycle | `duration_ms` should stay well under the sweep interval (6 hours, ID-18); a single sweep batch never holds a lock over 100 ms (`performance-requirements.md` NFR1.4.4) | A `rows_examined` count climbing cycle over cycle, without a matching `rows_deleted`, would mean the sweep query itself is failing silently — worth catching before storage grows unbounded |
| Rate-limiter `429` rate | `fetches_total{result="429"}` / `deletes_total{result="429"}` (OD-2) | Informational — a sustained rise against one identifier is the guessing-campaign pattern T1 describes (`security-design.md` SD-2), not itself a paged alert | Same "working as designed" reading U9's own `requestsLimited` metric gets |
| Postgres plugin storage used | Railway's Postgres plugin metrics | Reviewed monthly against the cost estimate's provisional ≤ $0.25/month storage term (`infrastructure-specification.md`) | The plugin-specific cost term this Unit's own cost estimate could not check against a live rate card; this is the empirical check |
| Postgres plugin connection count | Railway's Postgres plugin metrics | Should stay well under `max_connections = 10` (ID-17) even at Stage 1 peak | A count consistently near the ceiling would mean the pool is undersized for real traffic, read before it starts rejecting connections |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Service down (reused from U9, MD-1) | U9's own external uptime monitor's probe of the shared `/readyz` fails for two consecutive checks | Notify — now also covers "database unreachable," since `/readyz` is extended (`reliability-design.md` RD-2) to fail when the database ping fails | The maintainer, via U9's already-configured monitor notification — **no new alert or monitor configured by this Unit** |
| Deployment failed (reused from U9, MD-2) | Railway's build fails, or `/readyz` does not return 2xx within the probe timeout | Notify — a migration failure at start-up (ID-15) is one new way this can now happen, alongside U9's own causes | Railway's deployment notifications (already enabled for U9) |
| Spending threshold (reused from U9, MD-3) | Workspace usage passes the soft limit ($4) | Notify — this Unit's own Postgres plugin cost is now part of what this alert watches | Railway's spending-limit e-mail (already configured) |

This Unit defines **no new alert and configures no new monitor** — every
alerting path it needs (service down including a failed database ping,
deployment failure including a failed migration, the shared spending
threshold) already exists from U9's own `monitoring-design.md`, extended
in scope rather than duplicated. `observability-requirements.md`'s
NFR7.4 group (this Unit's own proposed addition, `nfr-requirements`)
never asked for a paged alert — only the four logging/counter/readiness
requirements OD-1 to OD-4 already satisfy — so no amendment is needed
here the way U9's own MD-1 needed one against NFR3.1.17: this Unit is
reusing an existing alert's *coverage*, not defining a new alert that
contradicts a "no alert" requirement.

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Availability (this Unit's endpoints, riding the shared service's own availability) | 99.5% monthly, same target and same measurement as U9's own (`reliability-requirements.md` NFR3.1.1) | Calendar month, from the same external monitor's probe history — no separate SLO for this Unit's own uptime, since it is not separately deployed |
| Upload/fetch/remove latency | `performance-requirements.md` NFR1.4.1–NFR1.4.3 | Read at the same review cadence U9's own NFR1.1 figures are read at, once real traffic exists |
| Expiry correctness (no early deletion, no unbounded over-retention) | 100% no-early-deletion; over-retention bounded to < 24 hours past the 30-day mark (`reliability-requirements.md` NFR3.4.2) | Every merge, from the sweep's own test suite — a test, not a monitor, matching U9's own "correctness is tested, not paged" pattern for NFR3.1.3 |

## Logs & Tracing

| Property | Design |
|---|---|
| Aggregation | None beyond the platform — the same stdout-JSON-to-Railway-log-explorer pipeline U9 already established; this Unit's rows are distinguished by their own field shapes (`operation`, `reason` vs. U9's own field set), never by a separate log destination |
| Retention | Whatever the plan provides, same as U9; this Unit's counters are monotonic per process run, same caveat as U9's own (a gap loses resolution, never totals) |
| Querying | By the fixed field names OD-1 to OD-3 name — `operation`, `reason`, `status` — in Railway's filter; no request-scoped field exists to query by, by design (`security-design.md` SD-7, matching U9's own BR7.1-driven choice) |
| Dashboards | None — same `team.md` rejection U9's own monitoring-design.md already applied |
| Health check semantics | The shared `/readyz` (U9's ID-6, this Unit's `reliability-design.md` RD-2) now also gates on a database ping; still a deploy-time gate for Railway plus a continuous probe from U9's own external monitor — no second health-check endpoint is introduced |

## What the maintainer does with it

| Moment | Reads | Decides |
|---|---|---|
| A down notification (shared MD-1, now covering this Unit too) | The latest `unready` state and this Unit's own failure rows | Whether the cause is the application or the database; redeploy or investigate the Postgres plugin |
| A failed deploy (shared MD-2) | The build/migration log | Fix the migration or the code before retrying |
| The monthly budget review | This Unit's stored-row count, Postgres plugin storage/connection metrics, alongside U9's own egress figures | Whether the combined ≤ $3.50 provisional allocation (`infrastructure-specification.md`) still holds, or whether growth is a `project.md` constraint change |

## Assumptions & Open Questions

- Extending the shared `/readyz` endpoint to also ping the database does
  not measurably change U9's own Ready-time budget (NFR1.1.4) — a single
  lightweight query adds negligible latency relative to U9's own
  manifest/cell verification at start. [assumption]
- Railway's Postgres plugin exposes per-plugin storage and connection
  metrics on its own service page, the same way it exposes memory/CPU for
  a compute service; confirmed at `environment-provisioning`. [assumption]

## Traceability

See `traceability.json` in this directory.

_Confirmed._
