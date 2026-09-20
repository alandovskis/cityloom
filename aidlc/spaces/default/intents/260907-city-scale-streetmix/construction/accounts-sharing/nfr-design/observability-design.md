# Observability Design — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `observability-requirements.md` (nfr-requirements, this
Unit), `design-storage/observability-design.md` OD-2 (sibling precedent —
`Counters`/`snapshot()` pattern, stdout-JSON-rows export, no HTTP
`/metrics` route).

Design elements are numbered `OD-n`.

## OD-1 — Counters (NFR7.5.1, NFR7.5.2, NFR7.5.3)

Same pattern as `design-storage`'s `Counters` (atomics, a `snapshot()`
method, direct unit tests, no HTTP route):

| Counter | Increments on |
|---|---|
| `signups_total{result="success"\|"failure", reason="<reason>"}` | Every sign-up attempt |
| `signins_total{result="success"\|"failure", reason="<reason>"}` | Every sign-in attempt |
| `grants_total{result="created"\|"revoked"}` | Every grant create/revoke |
| `links_total{result="enabled"\|"disabled"}` | Every link enable/disable |
| `access_gate_refusals_total` | A request refused specifically because the access-gate flag is off (NFR7.5.3) — distinct from an authorization refusal (flag on, no grant), which increments a different, existing counter path, never this one |

## OD-2 — Readiness signal (NFR7.5.4)

Same shape as `design-storage`'s NFR7.4.4 design: a `/readyz` route checks
the database connection independently of whether the process itself is
healthy — a simulated database outage reports not-ready while the process
keeps serving (so a load balancer can distinguish "restart me" from
"wait, backend is down").

## OD-3 — No per-request row; structured failure logs only

Consistent with `security-design.md` SD-7: a failed operation logs one
structured row (`occurredAt`, `operation`, `reason`) — never the
credential, session token, or `ShareLink.token` value.

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._
