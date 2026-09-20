# Observability Design — `design-storage` (U10)

Upstream inputs: `observability-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit), `functional-spec.md`, `rules.md`
(functional-design, this Unit), `osm-extract-proxy`'s own observability
design precedent (this workspace's one other server Unit).

Design elements are numbered `OD-n`; `traceability.json` maps each
`NFR7.4.x` from `observability-requirements.md` to the elements that
meet it.

## OD-1 — Failure log row (NFR7.4.1)

A failed operation emits exactly one `tracing` JSON row with fields
`occurredAt`, `operation` (`upload`/`fetch`/`remove`), `reason` (one of
`StorageFailure`'s closed values), and `status`. The row is built from
this fixed field set only — never the payload, the raw
`anonymous_design_id` value, or a requester-identifying field
(`security-design.md` SD-7's "nothing to leak" extends to logging: the
identifier genuinely never reaches a log statement, not merely redacted
from one). A successful operation emits no row at all, matching
`osm-extract-proxy`'s own "no per-request row" discipline.

## OD-2 — Aggregate counters (NFR7.4.2)

Counters, exported the same stdout-JSON-rows way `osm-extract-proxy`
already established (`tech-stack-decisions.md`'s stated reuse):

| Counter | Increments on |
|---|---|
| `uploads_total{result="success"}` | A committed `INSERT` |
| `uploads_total{result="failure", reason="<reason>"}` | Any `StorageFailure` on upload, one series per reason |
| `fetches_total{result="success"\|"403"\|"404"\|"429"}` | Every fetch outcome |
| `deletes_total{result="success"\|"429"}` | Every delete outcome (deletion is idempotent, so there is no delete-side 403/404, per `functional-spec.md`) |
| `stored_rows_count{ownership="anonymous"\|"account"}` | Sampled periodically (the sweep's own cycle, OD-3), not per-request |

## OD-3 — Expiry sweep aggregate row (NFR7.4.3)

Each sweep cycle (`reliability-design.md` RD-3) emits exactly one row —
`rows_examined`, `rows_deleted`, `duration_ms` — never the identifiers
of the rows it deleted. Because the sweep runs in batches
(`performance-design.md` PD-3), the row is emitted once per *cycle*
(all batches until zero rows remain), not once per batch, so a large
backlog after a skipped cycle (RD-3) still produces one row, not a burst
of them.

## OD-4 — Readiness distinguishes database from process (NFR7.4.4)

The readiness endpoint (`reliability-design.md` RD-2) reports one of two
states distinguishing "database reachable" from "process running": a
successful `SELECT 1` within a short timeout reports ready; a failed or
timed-out ping reports not-ready while the process itself continues
running and continues to accept the readiness check on the next probe.
This is the same signal shape `osm-extract-proxy`'s own OD-4/RD-1
established for its own start-up verification, applied here to an
ongoing per-request database dependency rather than a one-time start-up
load.

## Assumptions & Open Questions

- **[assumption]** The exact export mechanism (stdout JSON rows scraped
  by the platform, vs. a `/metrics` endpoint) is `tech-stack-decisions.md`'s
  choice, consistent with `osm-extract-proxy`'s own precedent and this
  project's no-paid-telemetry-service cost constraint.

## Traceability

See `traceability.json` in this directory.

_Confirmed._ (nfr-design)
