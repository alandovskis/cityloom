# Observability Requirements — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `requirements.md` (requirements-analysis),
`contract-summary.md` (contract-design), `nfr-requirements-questions.md`
(this stage), `team.md` (practices).

Ids follow `performance-requirements.md`: a third level refines an inception
requirement. Observability here serves two upstream obligations: FR6.3
(import failures are recorded where the maintainer can observe them) and
NFR3.1 (an availability target needs a measurement to be checked against).

## The constraint this whole file lives under

BR7.1, BR7.2 and BR7.4 fix what this service may ever write outside its own
memory:

- never a network address, a bounding box, an extract key, or anything
  derived from one of them (BR7.1);
- for a failure, exactly `occurredAt`, `reason`, `status`, `phase`, `detail`
  (BR7.2);
- otherwise aggregate counters only — **no per-request row exists anywhere**
  (BR7.4).

So this is observability without a request log. A successful request leaves
no trace of itself; what the maintainer can see is *how many*, *how fast*
(in buckets), *what failed and why*, and *whether the service is up*. That is
enough for FR6.3 and NFR3.1 and it is all ADR-004 permits. Every
requirement below is checked against those three rules, and NFR6.3.1's log
test (`security-requirements.md`) is the enforcement.

The stage's remit is *what* is observable and *what it may contain*; *how*
it is exported and where it is viewed is `observability-setup`'s
(`functional-spec.md`, integration points). Everything below is written to
stdout as one JSON object per line, which is what the platform collects
without any agent, endpoint or credential.

## Signals

| ID | Refines | Signal | Contents (closed list) | Cadence |
|---|---|---|---|---|
| NFR3.2.3 | NFR3.2 | **Readiness.** An HTTP endpoint answering ready or not-ready, read from the service readiness state machine (`functional-spec.md`): Ready → ready; Starting or Unready → not-ready. | A fixed token and the HTTP status the platform expects (path, status codes and probe interval are Infrastructure Design's). No build id, no region, no version. Exempt from requester limiting (NFR3.2.5). | On every probe. |
| NFR3.1.12 | NFR3.1 | **Counters row.** One line carrying `ServiceCounters` in full. | `buildId`, `extractsServed`, `bytesServed`, `cacheHits`, `cacheMisses`, `requestsLimited`, `failuresByReason` (one integer per `FailureReason` value, zero included), `lastExtractBytes`, plus the latency buckets of NFR3.1.14, plus `uptimeSeconds`. Counters are monotonic since process start (BR6.4). | **Every 60 seconds** while Ready, and once at graceful shutdown. |
| NFR3.1.13 | NFR3.1, FR6.3 | **Failure row.** One line per `FailureRecord`, at the moment the failure is answered (BR10.1). | Exactly `occurredAt`, `reason`, `status`, `phase`, `detail` (BR7.2). Level: `internal` → ERROR; `timeout`, `upstream_unavailable` → WARN; `invalid_area`, `area_too_large`, `area_not_found`, `rate_limited` → INFO. The `detail` is the project-authored text of the failure mapping table (`functional-spec.md`), never a dependency's error text and never anything computed from the request. | Per failure. |
| NFR3.1.14 | NFR3.1 | **Latency buckets** (the production measurement for NFR1.1.2 and NFR1.1.3). Two fixed histograms, hit and miss, each a count per bucket. | Bucket upper bounds in milliseconds: 10, 25, 50, 100, 250, 500, 1000, 3000, and over. Counts only; no per-request duration is written. Requires the `entities.md` amendment recorded in `performance-requirements.md`. | Carried in the counters row. |
| NFR3.1.15 | NFR3.1 | **Lifecycle rows.** One line at each service state transition. | Starting → Ready: `buildId`, `cellCount` verified, `elapsedMs` since process start, `regions` as the count of configured regions. Starting → Unready: which check failed (manifest missing, manifest unreadable, cell missing, cell digest mismatch) and the **count** of failing cells. A cell id here would be build data, not request data, so BR7.1 does not forbid it; the count is chosen because a build is replaced whole and the build rows (NFR3.1.16) already name what went into it. Shutdown: the final counters row. | Per transition. |
| NFR3.1.16 | NFR3.1 | **Build rows** (W1, the offline pipeline). | Per region: publisher path, published timestamp, source bytes, digest verified (yes/no), kept ways and nodes after the filter; per build: `buildId`, `cellCount`, `totalBytes`, elapsed time, and whether it was published, unchanged or failed. These name places by design — a region is public configuration, not a request. | Per build run. |

Nothing else is emitted. In particular: no access log, no per-request span
close events, no metric label carrying a build id per request, no
`X-Request-Id`, and no debug logging of request contents at any level in
any environment — a debug-level line that prints the box would be a BR7.1
violation that is only ever one configuration change from production.

## Service level indicators

The SLIs in `reliability-requirements.md` are computed from the signals
above and from nothing else:

| SLI | From |
|---|---|
| Availability (NFR3.1.1) | The platform's readiness probe history (NFR3.2.3). |
| Success ratio (NFR3.1.2) | `extractsServed` and `failuresByReason` in the counters row (NFR3.1.12), differenced between rows. |
| Hit and miss latency p95 / p99 (NFR1.1.2, NFR1.1.3) | The latency buckets (NFR3.1.14), differenced between rows. |
| Cost inputs (NFR5.1.2, NFR5.1.4, NFR5.1.5) | `bytesServed` for egress; memory and CPU from the platform's own metrics, which carry no per-request data. |
| The B-3 egress figure (W5) | `lastExtractBytes` after one real street and `bytesServed` over a known number of requests. |

## Alerting

`team.md` sets the bar: every control either blocks at the moment of the
mistake with an actionable message, or it is off — no dashboard, no periodic
report, no queue for one person. Applied here:

| ID | Refines | Requirement |
|---|---|---|
| NFR3.1.17 | NFR3.1 | **No alert is defined by this Unit in product Stage 1.** The alerts that exist are the platform's own: a failed deploy, a crashed and restarting service, and the account usage limit (NFR5.1.6). Each reaches the maintainer by the platform's notification channel without anything to configure in this Unit, and each is actionable (redeploy, read the failure rows, raise or hold the limit). A log-based alert on `internal` failures is the first candidate if one is ever added, and it belongs to `observability-setup` to decide — not here, because a threshold nobody has measured against is exactly the noisy alert `team.md` refuses. |

## Log format and retention

| Property | Requirement |
|---|---|
| Format | One JSON object per line on stdout; keys as named above; timestamps in RFC 3339 UTC. The platform's log collection reads stdout as-is. |
| Levels | ERROR, WARN, INFO as assigned to failure rows; lifecycle and counters rows at INFO; build rows at INFO with failures at ERROR. |
| Retention | Whatever the platform keeps; nothing is shipped elsewhere. The counters are monotonic since start, so a gap in collection loses resolution, never totals. |
| Volume | At the expected Stage 1 load: one counters row a minute, a handful of failure rows an hour, lifecycle rows per deploy. Under a request flood the failure rows are the one signal that scales with requests (one INFO row per `rate_limited`); a flood at the requester limits (NFR5.1.3) produces at most 30 rows a minute per limited requester, which is bounded but worth knowing. |
| Tracing | None. One process, one service, no downstream call at request time: there is no boundary for a trace to cross and no trace context to propagate. If a second server Unit ever calls this one in-process, that is a function call, not a span. |

## Amendments required

| Artifact | What must change | Why |
|---|---|---|
| `entities.md` `ServiceCounters` | Gains the two latency-bucket histograms and `uptimeSeconds` | NFR3.1.14 and NFR3.1.12 (recorded once, in `performance-requirements.md`; repeated here because this file is where the row's contents are fixed) |

## Assumptions & Open Questions

- The platform collects stdout as structured logs and shows them per
  deployment; confirmed at environment-provisioning. [assumption]
- A once-a-minute counters row is enough resolution to read a monthly SLI
  and the B-3 figure; if a finer window is ever wanted the cadence is a
  configuration value, not a design change. [assumption]
- Counting failing cells at Unready without naming them is enough to
  diagnose a bad build, because a build is replaced whole and the build
  rows (NFR3.1.16) name what went into it. [assumption]
