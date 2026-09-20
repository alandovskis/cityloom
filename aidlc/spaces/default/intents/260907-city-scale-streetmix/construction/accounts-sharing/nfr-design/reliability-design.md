# Reliability Design — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `reliability-requirements.md` (nfr-requirements, this
Unit), `nfr-design-questions.md` Q3, `contract-summary.md` Contract 6
(AC11.1.4), `design-storage/reliability-design.md` (sibling precedent).

Design elements are numbered `RD-n`.

## RD-1 — Sessions survive redeploy (NFR3.2.2)

Sessions are rows in the shared Postgres database, not process memory
(`security-design.md` SD-3) — a Railway redeploy restarts the process but
never the database, so no active session is invalidated by a deploy.

## RD-2 — Account-deletion cascade: one transaction

Resolves NFR3.5.1. Per `nfr-design-questions.md` Q3, the cascade is one
database transaction:

```sql
BEGIN;
DELETE FROM grant WHERE account_id = $1 OR granted_to_account_id = $1;
DELETE FROM share_link WHERE stored_design_id IN
  (SELECT stored_design_id FROM saved_design WHERE account_id = $1);
DELETE FROM saved_design WHERE account_id = $1;
DELETE FROM session WHERE account_id = $1;
DELETE FROM account WHERE account_id = $1;
COMMIT;
```

A failure at any statement rolls back the whole transaction — Postgres's
native atomicity is the mechanism, not application-level compensation
logic, satisfying AC11.1.4's "never a half-deleted account" requirement
directly. **Known gap carried from `nfr-requirements`** (see
`security-requirements.md`'s `[flagged gap, review R-02]`): this
capability is exposed as an internal function
(`delete_account_cascade(account_id)`), not a public route — no BR in
this Unit's `functional-design` names it, since the capability was
identified at `nfr-requirements` rather than `functional-design`. It is
designed here against the NFR/Contract 6 directly. `data-rights`/U12 owns
deciding when to invoke it (`reliability-requirements.md`'s own stated
assumption).

## RD-3 — Failure-injection test point

The test `reliability-requirements.md` NFR3.5.1 specifies (force an error
after `session` deletes but before `saved_design` deletes) targets this
transaction directly — a test-only hook or a deliberately-failing query
mid-transaction, asserting the whole operation rolls back and no row from
any of the five tables is missing or orphaned.

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._
