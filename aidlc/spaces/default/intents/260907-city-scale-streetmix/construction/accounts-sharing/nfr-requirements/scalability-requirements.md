# Scalability Requirements — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._


Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR2; `design-storage/scalability-requirements.md`
(sibling precedent — same single Railway instance, same database).

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR2.4.1 | NFR2 | This Unit holds no per-session state in process memory beyond what a stateless request handler needs; every `Session` is a database row, so horizontal scaling (if ever adopted past the current single-instance deployment) requires no sticky-session mechanism. | Code review / architecture check: no `HashMap`-backed or in-process session store exists (unlike `design-storage`'s intentionally in-process rate limiter, which is a different, short-lived kind of state). |
| NFR2.4.2 | NFR2 | `Session.token_hash` carries a unique index, so session-lookup latency (this Unit's highest-frequency query) does not degrade as the account/session count grows. | A test asserts the migration creates the index; a query-plan check (`EXPLAIN`) confirms an index scan, not a sequential scan, on lookup. |

## Amendments required

None — NFR2.4 is a new elaboration under `requirements.md`'s existing
NFR2 (Scale) group.

## Assumptions & Open Questions

- **[assumption]** This Unit shares one Postgres database and one
  `sqlx::PgPool` with `design-storage` once merged at the composition
  root (`infrastructure-design`'s decision) — this stage assumes, but
  does not fix, that sizing; a shared-pool sizing requirement belongs to
  whichever Unit's `infrastructure-design` stage actually configures the
  pool.

## Traceability

See `traceability.json` in this directory.
