# Reliability Requirements — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._


Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR3; `contract-summary.md` Contract 6
(erasure's all-or-nothing behaviour, AC11.1.4); `project.md` Mandated
(data subject rights).

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR3.2.2 | NFR3.2 | Sessions are stored in the database, not process memory; a process restart (a Railway redeploy, which NFR3.2 already requires to not count as downtime) does not sign out any active session. | A test creates a session, restarts the test harness's connection/process boundary, and asserts the session is still valid. |
| NFR3.5.1 | NFR3 | Deleting an `Account` cascades to every `Session`, `SavedDesign`, and `Grant` row referencing it, and to every `ShareLink` on a `SavedDesign` it owned, with no orphaned row left behind; the operation either completes in full or leaves the account and its rows completely intact — never a partially-deleted account, matching Contract 6's stated erasure behaviour (AC11.1.4). | A test simulates a failure partway through the cascade (e.g. a forced error after `Session` rows are deleted but before `SavedDesign` rows are) and asserts the whole operation rolled back — no row from any of the four tables is missing or orphaned. |

## Amendments required

None. NFR3.2.2 is a third-level elaboration of `requirements.md`'s
existing NFR3.2 (deployments must not cause downtime) — a session
surviving the redeploy NFR3.2 already governs is a direct consequence of
that requirement, not a new concern. NFR3.5 is a new second-level
elaboration under NFR3 (Availability) for the erasure-cascade concern,
which no existing NFR3.x item covers — the next unused NFR3 slot after
`design-storage`'s NFR3.1.1/NFR3.2.1/NFR3.3.1/NFR3.4.

## Assumptions & Open Questions

- **[assumption]** This Unit exposes the deletion-cascade *capability*
  only; deciding *when* to invoke it (the erasure request flow, any
  confirmation/grace period) is `data-rights`/U12's scope per
  `contract-summary.md` Contract 6/7 — not re-specified here.
- **[assumption]** The cascade is implemented as one database transaction
  (all four tables' deletes succeed or none do), the natural mechanism
  for AC11.1.4's all-or-nothing requirement — `nfr-design` will confirm
  the actual mechanism; this stage fixes the requirement, not the
  implementation.

## Traceability

See `traceability.json` in this directory.
