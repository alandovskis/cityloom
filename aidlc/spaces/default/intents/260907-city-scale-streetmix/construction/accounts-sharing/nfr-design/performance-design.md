# Performance Design — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `performance-requirements.md` (nfr-requirements, this
Unit), `nfr-design-questions.md` Q1/Q2, `design-storage/performance-design.md`
(sibling precedent — indexed-query-only pattern).

Design elements are numbered `PD-n`.

## PD-1 — No cache layer; indexed point-lookups only

Resolves NFR1.5.1/NFR1.5.3. `Account.email` and `Session.token_hash` each
carry a unique index (`scalability-design.md` SC-2). Sign-in, sign-up, and
grant/revoke/toggle operations are each a single indexed lookup plus a
single indexed write — no cache layer, matching `design-storage`'s own
resolution for the same budget/scale reasons (`nfr-design-questions.md`
Q1).

## PD-2 — Sharing-state list query, one round trip

Resolves NFR1.5.2. The design-list handler issues one query:

```sql
SELECT sd.*, COUNT(g.grant_id) AS grant_count, sl.enabled AS link_enabled
FROM saved_design sd
LEFT JOIN grant g ON g.stored_design_id = sd.stored_design_id
LEFT JOIN share_link sl ON sl.stored_design_id = sd.stored_design_id
WHERE sd.account_id = $1
GROUP BY sd.saved_design_id, sl.enabled
```

Never N+1: `functional-spec.md`'s Workflow "sharing state visible in the
design list" step 1 requires a computed, not cached, value per row, and
this join computes every row's value in the same round trip.

## PD-3 — Migration and save-with-name write paths

Resolves BR4.1/BR5.1's implied write-latency (covered by NFR1.5.1's
sign-up-path target, since migration happens inline with sign-up). Each
write is a single `INSERT`; the `name` uniqueness check (BR5.2) is
enforced by a unique index on `(account_id, name)`, not an application-side
`SELECT` before `INSERT` — index-enforced constraints resolve the
duplicate-name race that a check-then-insert would leave open.

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._
