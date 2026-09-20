# Observability Design — `osm-extract-proxy` (U9)

Upstream inputs: `observability-requirements.md`, `reliability-requirements.md`,
`performance-requirements.md` and `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md`, `rules.md` and
`entities.md` (functional-design, this Unit), `contract-summary.md`
(contract-design), `nfr-design-questions.md` (this stage).

Design elements are numbered `OD-n`; `traceability.json` maps each
`NFRx.y.z` from `observability-requirements.md` to the elements that meet
it. Export and viewing are `observability-setup`'s; this file fixes the
in-process design that produces the signals.

## One emitter, a closed catalogue

Everything the service ever writes outside its memory goes through one
module with one function per catalogued event. No other code calls the
logging facade directly. That is the design's whole enforcement of BR7.1,
BR7.2 and BR7.4: the fields each event may carry are its function's
parameters, so a new field is a code change in one place that the
NFR6.3.1 log test then exercises.

```
   request path            periodic task            lifecycle
        |                       |                        |
   record_failure()      emit_counters_row()     emit_ready()/emit_unready()
        |                       |                emit_shutdown()
        v                       v                        v
   +------------------------------------------------------------+
   |  emitter module: fixed events, fixed fields, JSON to stdout |
   +------------------------------------------------------------+
                                |
                                v
                     platform log collection
```

<!-- Text fallback: the request path calls record_failure, a periodic task calls emit_counters_row, and lifecycle code calls emit_ready, emit_unready and emit_shutdown; all go through one emitter module that writes fixed events with fixed fields as JSON to stdout, which the platform collects. -->

## Design elements

### OD-1 — Subscriber configuration

`tracing-subscriber` with the JSON formatter to stdout, RFC 3339 UTC
timestamps, level and target included, **span events off** (no enter,
exit or close rows), no ANSI. Configured once in `main`, before anything
else runs, and never reconfigured. The HTTP framework's request-tracing
layer is not installed (`security-design.md` SD-7). The minimum level is
INFO in every environment; there is no DEBUG mode for request contents
(`observability-requirements.md`, "Nothing else is emitted").

### OD-2 — Counters, histograms and the periodic row (NFR3.1.12, NFR3.1.14)

One `Counters` struct of atomics, created at start and bound to the
`buildId`:

- the `ServiceCounters` fields of `entities.md`: `extractsServed`,
  `bytesServed`, `cacheHits`, `cacheMisses`, `requestsLimited`,
  `failuresByReason` (one atomic per `FailureReason` value),
  `lastExtractBytes`;
- the two nine-bucket latency histograms of `performance-design.md` PD-6
  (`hitLatencyBuckets`, `missLatencyBuckets`, upper bounds 10, 25, 50,
  100, 250, 500, 1,000, 3,000 ms and over);
- `uptimeSeconds`, computed at emission from the start instant.

The request path updates these with relaxed atomic increments at the
response (W3 step 13) or at the failure mapping; no lock, no allocation.
A `tokio` interval task emits one `counters` row every 60 seconds while
Ready, and the shutdown path emits a final one (`reliability-design.md`
RD-7). The row is a snapshot of every field; counters are monotonic since
process start, so a reader differences consecutive rows.

### OD-3 — Failure rows (NFR3.1.13)

`record_failure(reason, phase, detail)` is the single failure-mapping
function (`reliability-design.md` RD-4). It increments
`failuresByReason[reason]`, then emits one `failure` row with exactly
`occurredAt`, `reason`, `status` (fixed per reason, BR10.1), `phase` (one
of the `FailureRecord.phase` values), `detail` (the project-authored text
from the failure mapping table), at the level the requirements assign
(ERROR for `internal`; WARN for `timeout` and `upstream_unavailable`; INFO
for the four client-caused reasons). Its parameters are the only inputs, so
it cannot carry a request field it was not given; the address and the box
are not in scope where it is called (`security-design.md` SD-1, SD-5).

### OD-4 — Lifecycle rows (NFR3.1.15)

- `ready`: `buildId`, `cellCount`, `regions` (count), `elapsedMs` since
  process start — emitted once when the Ready flag is set (RD-1).
- `unready`: `check` (which step failed: `manifest-missing`,
  `manifest-unreadable`, `store-missing`, `cell-missing`,
  `cell-digest-mismatch`), `failingCells` (count), `elapsedMs`.
- `shutdown`: the final counters row plus `reason` (`sigterm` or
  `sigint`).

### OD-5 — Build rows (NFR3.1.16)

The region-build tool uses the same emitter module with its own events:
`region` (publisher path, published timestamp, source bytes, verified,
kept ways, kept nodes) per region, and `build` (`buildId`, `cellCount`,
`totalBytes`, `elapsedMs`, `outcome` ∈ published | unchanged | failed,
and on failure `step` and a project-authored `detail`). Region rows name
public configuration, never a request.

### OD-6 — Readiness signal (NFR3.2.3)

The readiness endpoint reads the atomic Ready flag and answers 200 `ready`
or 503 `not-ready` (`security-design.md` SD-8). It emits no row per probe;
the platform's check history is the availability record (NFR3.1.1).

### OD-7 — SLI computation, specified for `observability-setup`

| SLI | Computation from the rows |
|---|---|
| Availability (NFR3.1.1) | Fraction of one-minute intervals with a passing readiness probe, from the platform's check history. |
| Success ratio (NFR3.1.2) | Between two `counters` rows: Δ`extractsServed` ÷ (Δ`extractsServed` + Δ`failuresByReason.internal` + Δ`failuresByReason.timeout`). |
| Hit / miss p95, p99 (NFR1.1.2, NFR1.1.3) | Between two `counters` rows: the Δ of each bucket; the percentile is the first bucket whose cumulative share reaches 95% / 99%. |
| B-3 egress figure (W5) | `lastExtractBytes` after the fixture street's request; Δ`bytesServed` over a known request count. |
| Cost inputs (NFR5.1.4, NFR5.1.5) | Δ`bytesServed` for egress; memory and CPU from the platform's metrics. |

### OD-8 — Alerting: none defined here (NFR3.1.17)

The platform's own notifications — failed deploy, crash-restart, account
usage limit — are the alerts of product Stage 1. This design adds no alert
rule and no threshold; `observability-setup` may add a log-based rule on
`failure` rows with `reason=internal` once a baseline exists, and OD-3's
fixed field names are what such a rule would key on.

### OD-9 — Tracing: deliberately none

One process, no downstream call, no cross-service boundary. No trace
context is propagated and no trace exporter is configured. A request span
may exist in code generation for the deadline (PD-4), carrying no fields
and never emitted (OD-1).

## Dashboard specification

None. `team.md` rejects dashboards for a solo builder; the counters row is
readable in the platform's log viewer, and the four SLIs above are
arithmetic on two rows. If a view is ever wanted, `observability-setup`
builds it from the fields named here, not from new signals.

## Rejected alternatives

- **A `/metrics` endpoint.** A public aggregate endpoint on an
  unauthenticated single-instance service is surface for no reader
  (`tech-stack-decisions.md` TS-6).
- **Per-request access logging with the address and box redacted.** A
  redaction is one configuration change from a leak; BR7.4 forbids the row
  itself, so the design emits none.
- **Sampling request spans at 10%.** Any sample is a per-request row.

## Assumptions & Open Questions

- The platform collects stdout as one JSON object per line and retains it
  long enough to difference two `counters` rows a month apart; if retention
  is shorter, the monthly SLIs are computed from the rows that exist and
  the gap is recorded. [assumption]
- Relaxed atomics are sufficient for the counters because every field is
  independent and read only by the periodic snapshot; no two fields need a
  consistent view. [assumption]
