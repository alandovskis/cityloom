# Monitoring Design — `osm-extract-proxy` (U9)

Upstream inputs: `observability-design.md`, `reliability-design.md`,
`performance-design.md`, `scalability-design.md`, `security-design.md` and
`logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-design-questions.md`
(this stage), `team.md` (practices).

Design elements are numbered `MD-n`. `observability-design.md` fixed the
signals the process emits (OD-1 to OD-9) and what they may contain; this
file is the platform side — where each signal lands on Railway, what reads
it, and the one external probe Q2 added. `team.md`'s bar applies to every
row: a control either notifies at the moment of the mistake with an
actionable message, or it is off. No dashboard exists.

## Where each signal lands

| Signal (from NFR design) | Emitted by | Lands in | Read by |
|---|---|---|---|
| `counters` row, every 60 s and at shutdown (OD-2) | the process, stdout | Railway log explorer (LC-5) | The maintainer, by differencing two rows (OD-7) |
| `failure` row, per failure (OD-3) | the process, stdout | Railway log explorer | The maintainer, filtered by `reason` |
| `ready` / `unready` / `shutdown` rows (OD-4) | the process, stdout | Railway log explorer | The maintainer; `elapsedMs` on `ready` is the NFR1.1.4 figure |
| `region` / `build` rows (OD-5) | the region-build tool, stdout | The GitHub Actions run log, retained with the run | The maintainer, on a failed or unchanged build |
| Readiness (OD-6) | `GET /readyz` → 200 `ready` / 503 `not-ready` | Railway's deploy-time health check; the uptime monitor (MD-1) | Railway, to gate rollover; the monitor, continuously |
| Memory, CPU, network (platform metrics) | Railway | Railway's service metrics | The maintainer, monthly, against NFR5.1.2 / NFR5.1.4 / NFR5.1.5 |

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Availability — fraction of 5-minute probes answering 200 (**MD-1**) | Uptime monitor on `/readyz` | ≥ 99.5% per calendar month (NFR3.1.1) | The only continuous up/down history: Railway's health check runs at deploy time only *(checked 2026-09-14)*, so the probe history `reliability-requirements.md` assumed does not exist on the platform |
| Success ratio | Δ`extractsServed` ÷ (Δ`extractsServed` + Δ`failuresByReason.internal` + Δ`failuresByReason.timeout`) between `counters` rows | ≥ 99.9% per month (NFR3.1.2) | Reads a server-side fault rate without a request log (BR7.4) |
| Miss latency p95 / p99 | `missLatencyBuckets` Δ between rows | p95 ≤ 300 ms, p99 ≤ 1,000 ms (NFR1.1.2) | B-3's first real number; drives the permit count and any move to precomputed extents (`performance-design.md`, rejected alternatives) |
| Hit latency p95 / p99 | `hitLatencyBuckets` Δ | p95 ≤ 30 ms, p99 ≤ 100 ms (NFR1.1.3) | Confirms the common path never touches disk |
| Ready time | `elapsedMs` on the `ready` row | ≤ 30 s at p95 (NFR1.1.4); must sit well inside the 120 s probe timeout (NFR3.2.2) | Grows with the region set (SC-4); the trigger to re-check before a second province |
| `timeout` failures | `failuresByReason.timeout` Δ | Rising from zero at Stage 1 load | Misses exceeding ~4/s sustained (SC-2's threshold table); raise the permit count or examine CPU |
| `requestsLimited` | `counters` row | Informational | Working as designed (SC-1); a sustained rise from one requester is the T4 pattern and costs nothing |
| Memory in use | Railway metrics | ≤ 128 MB typical, ≤ 192 MB p99 (NFR5.1.2) | The one number that is a monthly charge; approaching 192 MB is the "investigate" row of SC-2's threshold table |
| CPU averaged | Railway metrics | ≤ 0.05 vCPU (NFR5.1.4) | Cutting is the only CPU-heavy work |
| Egress | `bytesServed` Δ; Railway's network metric | Inside NFR5.1.5's ≤ $2.50 all-in, provisional | The B-3 gate figure (W5); a measured breach is a constraint change under `project.md`, never absorbed |
| Monthly spend | Railway usage | Soft $4, hard $5 (NFR5.1.6) | The last line of defence for the budget |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Service down (**MD-1**) | The uptime monitor's probe of `/readyz` fails (non-200 or unreachable) for two consecutive checks | Notify — the only actionable outage signal; action: read the `unready` / `failure` rows, redeploy the last good build (RD-9) | The maintainer, by the monitor's e-mail (and the monitor's own app or webhook if wanted); nothing to configure in this Unit |
| Deployment failed (**MD-2**) | Railway's build fails, or the health check on `/readyz` does not return 2xx within 120 s | Notify — the previous deployment keeps serving (RD-2); action: read the build log or the `unready` row | Railway's deployment notifications to the maintainer *(the platform's own; `observability-requirements.md` NFR3.1.17)* |
| Crash-restart (**MD-2**) | The process exits non-zero and the `ON_FAILURE` restart policy fires; the retry count exhausts | Notify — a crash is a defect (NFR3.1.5, NFR7.3.1); action: read the last `failure` row, write the regression test, fix | Railway's deployment notifications |
| Spending threshold (**MD-3**) | Workspace usage passes the soft limit ($4) | Notify — action: read egress and memory against NFR5.1.5; decide whether a constraint change is needed | Railway's spending-limit e-mail; at the hard limit ($5) services stop, which is the designed last line (NFR5.1.6) |
| Data build failed or unchanged (**MD-4**) | The weekly GitHub Actions run fails, or exits with "unchanged" | Notify on failure — action: read the `build` row's `step` and `detail`, re-run on demand (BR8.5); unchanged is informational | GitHub's workflow-failure notification to the maintainer |

MD-1 is the one alert this Unit defines, which NFR3.1.17's text ("no alert
is defined by this Unit") does not allow for — see "Amendments required"
below for the supersession and why. Deliberately absent, per NFR3.1.17 and `team.md`: any log-based alert on
`internal` failures (no baseline exists to set a threshold against; the
first candidate if one is ever added, keyed on OD-3's `reason` field), any
latency alert (the buckets are read at B-3 and at review, not paged on),
and any alert on `rate_limited` (a correct answer, not a failure).

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Availability: probes answering 200 ÷ all probes (MD-1) | ≥ 99.5% (216 minutes of downtime allowed) | Calendar month, from the monitor's history at a 5-minute interval — a resolution of 5 minutes on a 216-minute budget |
| Success ratio (OD-7) | ≥ 99.9% | Calendar month, from the first and last `counters` rows of the month, plus every restart boundary (counters reset with the process; each `ready` row starts a new run to sum) |
| Correctness: identical bytes for the same canonical box on the same build (NFR3.1.3) | 100% | Every merge, from the golden-fixture suite — a test, not a monitor |
| Hit and miss p95 / p99 | NFR1.1.2, NFR1.1.3 | Read at B-3 and at each review; between two `counters` rows |

A month whose availability budget is spent is reported at the gate of the
next Bolt touching this Unit (`reliability-requirements.md` NFR3.1.1),
which is the one "periodic" reading `team.md` permits because it is a gate
input, not a report nobody reads.

## Logs & Tracing

| Property | Design |
|---|---|
| Aggregation | None beyond the platform: the process writes one JSON object per line to stdout (OD-1) and Railway's log explorer collects and searches it. No agent, no shipping, no log-analytics service. |
| Retention | Whatever the plan provides; counters are monotonic per process run so a gap loses resolution, never totals (`observability-requirements.md`). The monthly success ratio is computed from the rows that exist. |
| Querying | By the fixed field names OD-2 to OD-5 name — `reason`, `check`, `outcome`, `buildId` — in Railway's filter; no request-scoped field exists to query by, by design (BR7.1). |
| Build logs | The GitHub Actions run log keeps the `region` and `build` rows with the run; a failed run is the notification (MD-4). |
| Tracing | None (OD-9). One process, no downstream call at request time; no trace exporter, no trace context, no sampling. |
| Dashboards | None. `team.md` rejects a dashboard for a solo builder; the four SLIs are arithmetic on two rows, and the uptime monitor's own status page is the one view that exists, unbuilt by this project. |
| Health check semantics | Railway's check (`GET /readyz`, 2xx, hostname `healthcheck.railway.app`, 120 s timeout) is a **deploy-time gate only** *(checked)*; continuous availability is MD-1's. The two read the same flag (OD-6), so a deployment the platform accepted is one the monitor will see as up. |

## What the maintainer does with it

| Moment | Reads | Decides |
|---|---|---|
| A down notification (MD-1) | The latest `unready` or `failure` rows; Railway's deployment status | Redeploy the last good build (RD-9), or wait out a platform incident (NFR3.1.11) |
| A failed deploy (MD-2) | The build log; the `unready` row's `check` and `failingCells` | Fix the artifact (a data build re-run, BR8.5) or the code |
| B-3's gate | `lastExtractBytes`, `bytesServed`, the miss buckets, memory and CPU | Whether the ≤ $2.50 allocation holds; whether to change the permit count or take the precomputed-extent optimisation |
| The next Bolt's gate for this Unit | The month's availability and success ratio | Whether the SLOs held; a spent budget is stated, not absorbed |

## Amendments required

Recorded rather than edited, because the NFR requirements are approved —
the treatment `performance-requirements.md` and `security-requirements.md`
use for the same situation.

| Artifact | What must change | Why |
|---|---|---|
| `observability-requirements.md` NFR3.1.17 | "No alert is defined by this Unit in product Stage 1 … without anything to configure in this Unit" becomes: this Unit defines **exactly one** alert, the uptime monitor's down notification on `/readyz` (MD-1), configured outside the service in the monitor's account; the platform's own three notifications stay as listed; the log-based `internal` alert stays undefined | Q2 of this stage: Railway's health check is deploy-time only *(checked 2026-09-14)*, so NFR3.1.1's SLI had no source; the human chose an external monitor over an unmeasured SLO. The monitor is a fourth alert and a thing to configure, which NFR3.1.17's text as written denies — the requirement is superseded for this one case, not quietly contradicted |
| `reliability-requirements.md` NFR3.1.1, "Measured how" | "The platform's own health-check history" becomes "the external uptime monitor's probe history at a 5-minute interval (`monitoring-design.md` MD-1)" | The platform keeps no such history; the SLI is now read from the monitor |
| `observability-requirements.md` NFR3.2.3 | Add: the readiness endpoint is probed by an external monitor every 5 minutes in addition to the platform's deploy-time check; both read the same flag | So the endpoint's stated readers match its actual ones |

## Assumptions & Open Questions

- The uptime monitor's free tier provides a 5-minute interval, at least a
  month of history and an e-mail notification; the service is chosen at
  environment-provisioning (UptimeRobot's free plan was checked to offer
  this on 2026-09-14, and any comparable service will do). [assumption]
- Railway's deployment notifications reach the maintainer's e-mail for a
  failed build, a failed health check and a crash loop without anything to
  configure in this Unit beyond enabling them on the project; confirmed at
  environment-provisioning. [assumption]
- Two consecutive failed probes (10 minutes) as the down condition is a
  provisional debounce against a single dropped probe; it costs at most 10
  minutes of the 216-minute budget before a notification. [assumption]
