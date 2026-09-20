# Reliability Requirements — `design-storage` (U10)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `requirements.md` NFR3; `team.md` Deployment.

## Requirements

| ID | Refines | Requirement | Target | Measured how |
|---|---|---|---|---|
| NFR3.1.1 | NFR3.1 | This Unit's endpoints target the same 99.5% monthly availability as the rest of the single Railway service. | 99.5% monthly, measured at the shared service level (this Unit is not separately deployed — see `unit-of-work.md`'s "four Units, one deployable"). | Platform uptime monitoring, per `environment-provisioning`. |
| NFR3.2.1 | NFR3.2 | A deploy carrying a change to this Unit does not count as downtime: the new version passes a health check (which must itself exercise a real database round-trip, not just process liveness) before receiving traffic; the previous version keeps serving until then. | Health check includes a lightweight database ping. | Verified at `environment-provisioning`/`deployment-execution`. |
| NFR3.3.1 | NFR3.3 | This Unit's data lives in a separate database service, never an attached volume on the application service — required for NFR3.2's zero-downtime redeploy to hold. | PostgreSQL as a distinct Railway-managed service (`components.md`'s stated external dependency). | Confirmed at `infrastructure-design`. |
| NFR3.4.1 | (proposed — see Amendments) | **A failed upload never leaves a partial row.** BR6.1's atomicity guarantee is a reliability property as much as a security one: a crash or database error mid-write leaves zero rows for that upload attempt, never a half-written one. | No partial-write test finds an orphaned row after a simulated mid-transaction failure. | A test that kills the write mid-transaction (or simulates the failure point) and asserts the table has no matching row afterward. |
| NFR3.4.2 | (proposed) | **The expiry mechanism (BR2.1) is itself resilient to being skipped for a cycle** — a missed sweep does not corrupt state, only delays deletion; retention is never understated (a design never disappears before its 30 days), only potentially overstated by one sweep interval. | Sweep interval ≤ 24 hours, so worst-case over-retention is < 24 hours past the 30-day mark. | A test simulates a skipped sweep cycle and asserts no design is deleted early; the interval is confirmed at `infrastructure-design`. |

## Amendments required

| Artifact | What must change | Why |
|---|---|---|
| `requirements.md` NFR3 | Gains a proposed **NFR3.4** — "Storage writes are atomic and the expiry mechanism degrades safely" — as the inception home for NFR3.4.1–NFR3.4.2 | No inception requirement currently covers write atomicity or expiry-mechanism resilience specifically |

## Assumptions & Open Questions

- **[assumption]** The expiry sweep's exact interval and mechanism
  (scheduled job vs. lazy-check-on-read) are `infrastructure-design`'s
  to fix — this stage only requires that whatever mechanism is chosen
  cannot understate retention.

## Traceability

See `traceability.json` in this directory.
