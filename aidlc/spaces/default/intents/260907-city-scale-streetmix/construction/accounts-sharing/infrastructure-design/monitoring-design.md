# Monitoring Design — `accounts-sharing` (U11)

_Confirmed (consolidated summary re-confirmed 2026-09-20 after review-repair)._

Upstream inputs: `observability-design.md`, `reliability-design.md`,
`performance-design.md`, `scalability-design.md`, `security-design.md` and
`logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-specification.md`
(this stage), `design-storage`'s own `monitoring-design.md` (U10, this
workspace's precedent), `team.md` (practices).

Design elements are numbered `MD-n` (this Unit's own numbering, distinct
from U9/U10's `MD-n` rows — each Unit's monitoring design numbers its own
elements independently; cross-references to a sibling's row cite it by
Unit prefix). `observability-design.md` fixed the signals this Unit's
process emits (OD-1 to OD-3); this file is the platform side.

## Where each signal lands

| Signal (from NFR design) | Emitted by | Lands in | Read by |
|---|---|---|---|
| Counters — signups/signins/grants/links/gate-refusals (OD-1) | This Unit's `handlers`, stdout | Railway log explorer (LC-5, shared) | The maintainer, by differencing two rows |
| Readiness (OD-2) | `GET /readyz` — the **same shared endpoint** U9/U10 already extended, now also implicitly covering this Unit (the endpoint pings the shared database both Units read/write) | Railway's deploy-time health check; U9's own external uptime monitor (MD-1, U9) | Railway, to gate rollover; the same monitor — no new monitor for this Unit |
| Structured failure rows (OD-3) | This Unit's `handlers`, stdout, one row per failed operation | Railway log explorer | The maintainer, filtered by `operation` and `reason` |
| Database metrics (connections, storage used) | Railway (the shared Postgres plugin) | Railway's service metrics, on the plugin's own service page | The maintainer, monthly — now reading U10's existing dashboard-less metrics page, with this Unit's five tables' contribution folded into the same total U10 already watches |

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Sign-up/sign-in success ratio | Δ`signups_total{result="success"}` ÷ Δ total signups; same for signins | ≥ 99% (health signal, not a paged SLO — same reading U10 applies to its own upload success ratio) | A drop without a matching spike in a specific `reason` bucket is the signal to check `internal`/database-connectivity failures first |
| Access-gate refusal count | `access_gate_refusals_total` (OD-1) | Informational while the flag is intentionally off (pre-public-release); should read **zero** once the flag is turned on for real, sustained traffic — a nonzero rate after go-live means a client is still hitting stale gated endpoints | Directly verifies `security-design.md` SD-1/NFR6.2.5/NFR6.2.7's fail-closed behavior is actually being exercised as designed, not merely present in code |
| Grant/link churn | `grants_total`, `links_total` (OD-1) | Informational | Read alongside the design-list sharing-state query (`performance-design.md` PD-2) if that query's latency ever needs investigating — a spike in churn is the first thing to check |
| Shared Postgres plugin storage used | Railway's Postgres plugin metrics (shared with U10) | Reviewed monthly, folded into U10's existing ≤ $0.25/month storage review, not a separate threshold | This Unit's own tables (no payload bytes) are a small fraction of the total; no separate line needed |
| Shared Postgres plugin connection count | Railway's Postgres plugin metrics (shared with U10) | Should stay well under U10's own `max_connections = 10` ceiling even with this Unit's queries added, since this Unit constructs no new pool (`scalability-design.md` SC-3) | A count consistently near the ceiling now has two Units' query load to account for when diagnosing, not one |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Service down (reused from U9/U10, MD-1) | The shared external uptime monitor's probe of `/readyz` fails for two consecutive checks | Notify — unchanged scope, this Unit's own database reads/writes ride the same readiness signal U10 already extended | The maintainer, via the already-configured monitor notification — **no new alert or monitor configured by this Unit** |
| Deployment failed (reused, MD-2) | Railway's build fails, or `/readyz` does not return 2xx within the probe timeout | Notify — a failure in this Unit's own migration (ID-21) is one more new way this can happen, alongside U9/U10's own causes | Railway's deployment notifications (already enabled) |
| Spending threshold (reused, MD-3) | Workspace usage passes the soft limit ($4) | Notify — this Unit adds no new metered resource, so its marginal contribution to this alert is effectively nil | Railway's spending-limit e-mail (already configured) |

This Unit defines **no new alert and configures no new monitor** — the
same reuse pattern U10 already established for itself. No confirmed NFR
in `nfr-requirements` for this Unit asks for a paged alert; OD-1 through
OD-3 are satisfied by the existing logging/counter/readiness paths.

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Availability (this Unit's endpoints, riding the shared service's own availability) | 99.5% monthly, same target and measurement as U9/U10's own (`reliability-requirements.md` NFR3.2.1's own reference to the shared-service target) | Calendar month, from the same external monitor's probe history |
| Account-deletion cascade correctness (RD-2/RD-3) | 100% atomicity — every test-forced mid-transaction failure leaves zero orphaned rows across the five tables | Every merge, from the cascade's own test suite (`reliability-design.md` RD-3) — a test, not a monitor, same "correctness is tested, not paged" pattern U10 already applies to its own expiry sweep |
| Session revocation immediacy (NFR6.2.8) | 100% — the very next request after a session row is deleted is refused, no grace window | Every merge, from `security-requirements.md` NFR6.2.8's own specified test |

## Logs & Tracing

| Property | Design |
|---|---|
| Aggregation | None beyond the platform — same stdout-JSON-to-Railway-log-explorer pipeline; this Unit's rows are distinguished by their own field shapes (`operation` values like `signup`/`signin`/`grant`/`link`, distinct from U10's `upload`/`fetch`/`delete`) |
| Retention | Whatever the plan provides, same as U9/U10 |
| Querying | By the fixed field names OD-1/OD-3 name — no request-scoped, credential, or token-value field exists to query by, by design (`security-design.md` SD-7) |
| Dashboards | None — same `team.md` rejection U9/U10 already applied |
| Health check semantics | Unchanged shared `/readyz` — this Unit adds no new health-check condition beyond what U10's database ping already covers, since both Units' tables live in the same database |

## What the maintainer does with it

| Moment | Reads | Decides |
|---|---|---|
| A down notification (shared MD-1) | The latest `unready` state and both U10's and this Unit's own failure rows | Whether the cause is the application or the shared database; redeploy or investigate the Postgres plugin |
| A failed deploy (shared MD-2) | The build/migration log | Fix whichever Unit's migration (U10's or this Unit's) failed, before retrying |
| The monthly budget review | This Unit's counters, alongside U10's own storage/connection figures on the shared plugin | Whether the combined allocation still holds — this Unit adds no new line item, only reads into U10's existing one |

## Assumptions & Open Questions

- The shared `/readyz`'s existing database ping (extended once already by
  U10) is sufficient coverage for this Unit too, since both Units' tables
  live in the same database instance — no second, Unit-specific readiness
  check is needed. [assumption]
- Railway's Postgres plugin metrics page does not break down storage/
  connections per-table or per-Unit — the monthly review reads one
  combined figure, same as U10's own monitoring-design.md already
  assumes. [assumption]

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._
