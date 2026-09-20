# Performance Requirements — `design-storage` (U10)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md`, `entities.md`
(functional-design, this Unit); `requirements.md` NFR1;
`contract-summary.md` Contract 2.

No inception NFR fixes a numeric budget for this Unit's three endpoints
directly — NFR1.1's 10-second budget is scoped to a full osm2streets
import (U4/U9), and uploading/fetching a stored design happens outside
that workflow (at "explicitly keep a design" and "reopen a design"
moments, not at import time). This stage therefore sets its own
provisional targets, following the same discipline `osm-extract-proxy`
(U9) used for its own NFR1.1.1–NFR1.1.6: numbers the service is built
and tested against, replaced by a measured figure through an explicit
amendment, never silently.

## Requirements

| ID | Refines | Requirement | Target | Measured how |
|---|---|---|---|---|
| NFR1.4.1 | (proposed — see Amendments) | **Upload latency.** `POST /api/designs` completes (payload validated, row committed, receipt returned) within budget. | p95 ≤ 500 ms, p99 ≤ 1,500 ms, on the shared Railway service/database sizing. | CI benchmark against a representative `DesignPayload` size (a multi-street corridor design); production: aggregate latency buckets exported alongside counters (same pattern as `osm-extract-proxy`'s NFR1.1.2 measurement, avoiding a per-request log row). |
| NFR1.4.2 | (proposed) | **Fetch latency.** `GET /api/designs/{id}` (including the object-level authorization check) completes within budget. | p95 ≤ 200 ms, p99 ≤ 800 ms. | Same two mechanisms as NFR1.4.1. |
| NFR1.4.3 | (proposed) | **Removal latency.** `DELETE /api/designs/{id}` completes within budget, whether or not the row existed. | p95 ≤ 200 ms, p99 ≤ 800 ms. | Same two mechanisms. |
| NFR1.4.4 | (proposed) | **Expiry sweep does not compete with request-serving latency.** Whatever mechanism removes expired rows (BR2.1) runs without holding a lock that blocks concurrent upload/fetch/delete requests for more than a bounded window. | A single sweep batch holds no lock for more than 100 ms. | A test runs concurrent requests during a simulated sweep and asserts no request exceeds its own p99 target by more than 2x. |

## Amendments required

| Artifact | What must change | Why |
|---|---|---|
| `requirements.md` NFR1 | Gains a proposed **NFR1.4** — "Server-side design storage operations (upload, fetch, remove) meet a stated latency budget" — as the inception home for NFR1.4.1–NFR1.4.4 | No inception requirement currently covers storage-operation latency directly; `osm-extract-proxy` set the precedent of proposing a numbered home for a genuinely new requirement class rather than attaching it to an unrelated existing id |

## Assumptions & Open Questions

- **[assumption]** The exact database technology and its typical query
  latency are `tech-stack-decisions.md`'s to fix; these targets assume a
  managed PostgreSQL instance on the same platform (Railway), consistent
  with `components.md`'s stated `PostgreSQL` external dependency for
  `DesignRepository`.
- **[assumption]** Payload size distribution (small single-street edits
  vs. large multi-street corridor designs) is not yet measured; the CI
  benchmark's representative size is a reasonable estimate pending real
  usage data.

## Traceability

See `traceability.json` in this directory.
