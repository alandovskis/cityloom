# Performance Requirements — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._


Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR1; `design-storage/performance-requirements.md`
(sibling precedent, same single-instance deployment).

This Unit's tables (`Account`, `Session`, `SavedDesign`, `Grant`,
`ShareLink`) are small relative to `design-storage`'s `stored_designs`
table — targets here are looser, not because the requirement is less
real, but because the workload genuinely is: sign-ins and shares happen
per-user-action, not per-edit.

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR1.5.1 | NFR1 | Sign-up and sign-in requests complete with p95 latency under 300ms under normal single-instance load. | A load test against a local instance measures p95 over a representative request mix. |
| NFR1.5.2 | NFR1 | A design-list response (US10.3's per-design computed sharing state) completes with p95 latency under 400ms for an account with up to 100 saved designs. | A test seeds 100 `SavedDesign` rows with a mix of `Grant`/`ShareLink` state and measures the list query's latency. |
| NFR1.5.3 | NFR1 | Granting, revoking, or toggling a link's enabled state completes with p95 latency under 300ms. | A load test measures p95 over repeated grant/revoke/toggle requests. |

## Amendments required

None — NFR1.5 is a new elaboration under `requirements.md`'s existing
NFR1 (Performance) group, not a change to it.

## Assumptions & Open Questions

- **[assumption]** The 400ms list-response target assumes the
  sharing-state computation (BR9.1) is a single indexed join per design,
  not an N+1 query per row — `nfr-design` will confirm the actual query
  shape; this stage fixes the target, not the implementation.

## Traceability

See `traceability.json` in this directory.
