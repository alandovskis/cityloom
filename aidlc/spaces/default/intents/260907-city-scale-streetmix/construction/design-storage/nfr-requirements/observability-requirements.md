# Observability Requirements — `design-storage` (U10)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR7; `team.md` Deployment;
`osm-extract-proxy`'s own observability precedent (this workspace's one
other server Unit).

Following `osm-extract-proxy`'s established discipline (BR7.1/BR7.2/BR7.4
there): **no per-request row for a successful request**, since a
request-level log for `POST/GET/DELETE /api/designs/*` would otherwise
carry the `anonymousDesignId` and risk becoming a linkage record this
Unit's own BR7.1 (no personal signal on the identifier) forbids treating
casually.

## Requirements

| ID | Requirement | Verified by |
|---|---|---|
| NFR7.4.1 | A failed operation (upload/fetch/delete) emits exactly one structured log row with `occurredAt`, `operation`, `reason` (one of `StorageFailure`'s closed values), and `status` — never the payload, the identifier's raw value, or a requester-identifying field. | A test drives one of each failure reason through the service with log capture and asserts none of the forbidden fields appear. |
| NFR7.4.2 | Aggregate counters exist for: uploads (success/failure by reason), fetches (success/403/404/429), deletes (success/429), and current stored-row count (split anonymous/account-owned). | A metrics-endpoint test asserts each counter exists and increments on the corresponding operation. |
| NFR7.4.3 | The expiry mechanism emits one aggregate row per sweep cycle: rows examined, rows deleted, duration — never the deleted rows' identifiers. | A test triggers a sweep and asserts the log row's field set matches exactly this list. |
| NFR7.4.4 | A readiness/health signal exists distinguishing "database reachable" from "process running," consistent with NFR3.2.1's health-check requirement. | A test with a simulated database outage asserts the health endpoint reports not-ready while the process itself keeps running. |

## Amendments required

None — these requirements refine `requirements.md` NFR7's existing
"maintainability and verification" heading directly (NFR7.1–NFR7.3 are
already about coverage/pinning/regression tests; NFR7.4 extends that
heading to this Unit's own operational visibility, matching the pattern
`osm-extract-proxy` would have used had its own numbering not already
occupied NFR7.2 for dependency pinning specifically — no conflict here
since this Unit proposes NFR7.4, the next unused sub-number).

## Assumptions & Open Questions

- **[assumption]** Log/metric export mechanism (stdout JSON rows vs. a
  dedicated telemetry SDK) is a `tech-stack-decisions.md` choice,
  consistent with `osm-extract-proxy`'s own stdout-JSON-rows precedent
  under this project's cost constraint (no paid telemetry service).

## Traceability

See `traceability.json` in this directory.
