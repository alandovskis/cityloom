# Observability Requirements — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._


Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR7; `design-storage/observability-requirements.md`
(sibling precedent, same stdout-JSON-rows export mechanism); `project.md`
Mandated (access gate — this Unit is the one place its "verifiably off"
claim can actually be checked).

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR7.5.1 | NFR7 | Aggregate counters exist for sign-up and sign-in outcomes, split success/failure by reason. | A test asserts each counter exists and increments on the corresponding operation, same pattern as `design-storage`'s NFR7.4.2 test. |
| NFR7.5.2 | NFR7 | Aggregate counters exist for grant creation, grant revocation, link enable, and link disable. | A test asserts each counter increments on its corresponding operation. |
| NFR7.5.3 | NFR7 | A counter tracks requests refused because the access-gate flag is off, distinct from any other refusal reason. | A test with the flag off asserts the counter increments and that an authorization refusal (flag on, but no grant) does not increment the same counter. |
| NFR7.5.4 | NFR7 | A readiness/health signal distinguishes "database reachable" from "process running," consistent with `design-storage`'s NFR7.4.4. | A test with a simulated database outage asserts the health endpoint reports not-ready while the process itself keeps running. |

## Amendments required

None — NFR7.5 is a new elaboration under `requirements.md`'s existing
NFR7 (Maintainability and verification) group, continuing the numbering
`design-storage`'s NFR7.4 already started (the next unused NFR7
sub-group).

## Assumptions & Open Questions

- **[assumption]** Export mechanism: stdout JSON rows, the same
  established pattern `osm-extract-proxy` and `design-storage` both use
  — no new export mechanism introduced for this Unit.
- **[assumption]** NFR7.5.3's counter is what makes `security-requirements.md`
  NFR6.2.5's "verifiably off" claim checkable in production rather than
  only in tests — this is the cross-reference that requirement names.

## Traceability

See `traceability.json` in this directory.
