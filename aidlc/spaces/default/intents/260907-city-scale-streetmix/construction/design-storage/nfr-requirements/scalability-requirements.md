# Scalability Requirements — `design-storage` (U10)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR2; `contract-summary.md` Contract 2.

NFR2's stated scale target ("thousands of streets, only the visible
portion loaded") is a client-side concern (map/viewport, U6) — this
Unit's own scale question is different: how many stored designs and how
much storage growth this service must absorb within the ~$5/month
budget (`project.md` OC-4).

## Requirements

| ID | Refines | Requirement | Target | Measured how |
|---|---|---|---|---|
| NFR2.3.1 | (proposed — see Amendments) | **Storage growth is bounded by expiry.** Anonymous uploads expire after 30 days (BR2.1), which caps steady-state anonymous storage at roughly (uploads per day × 30), not an ever-growing total. | No fixed number — this is a bound on growth *rate*, not a ceiling; monitored via the stored-row count metric (`observability-requirements.md`). | Reviewed monthly against the OC-4 budget-change trigger (`project.md` Forbidden). |
| NFR2.3.2 | (proposed) | **Payload size cap bounds per-row storage.** Every stored row's `payload` is bounded by the same size cap `security-requirements.md` NFR6.4.3 enforces at write time. | Cap value fixed at `tech-stack-decisions.md` (a reasonable design payload — lane edits and corrections for a multi-street corridor — is expected to be well under 1 MB; the cap is set generously above realistic usage, not tuned to it). | The same NFR6.4.3 test also confirms the cap is enforced. |
| NFR2.3.3 | (proposed) | **Concurrent request handling does not require this Unit to hold in-memory per-request state across requests** (unlike `osm-extract-proxy`'s in-flight-cut de-duplication, BR6.3 there) — each upload/fetch/delete is independent and stateless at the application layer, so horizontal scaling (if ever needed) requires no coordination between instances. | N/A — a design property, not a numeric target. | Code review: no shared mutable state beyond the database connection pool and the rate limiter's own bookkeeping (which is per-identifier, not global). |

## Amendments required

| Artifact | What must change | Why |
|---|---|---|
| `requirements.md` NFR2 | Gains a proposed **NFR2.3** — "Server-side design storage growth and per-row size are bounded, and the service holds no cross-request coordination state" — as the inception home for NFR2.3.1–NFR2.3.3 | NFR2 as written is scoped to client-side map/viewport scale, not server-side storage growth |

## Assumptions & Open Questions

- **[assumption]** The single Railway service/database pairing
  (`team.md` Deployment, "one environment") is expected to handle
  Stage 1's realistic anonymous-upload volume without needing read
  replicas or sharding — this is a solo-builder-scale product, not a
  high-traffic one, per `constraint-register.md`'s cost constraint.

## Traceability

See `traceability.json` in this directory.
