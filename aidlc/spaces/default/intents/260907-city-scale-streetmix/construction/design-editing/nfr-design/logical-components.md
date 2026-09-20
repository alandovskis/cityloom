# Logical Components — `design-editing` (U5)

_Confirmed._

Upstream inputs: `security-design.md` (this stage); `components.md`
(`DesignOverlay`, `EditingSession`, `CorridorPlanner`);
`tech-stack-decisions.md`.

Like `street-import` (U4) and `street-core` (U2), this Unit's deployment
model is "embedded — a crate in the client Cargo workspace," not a
deployed service — no service boundary or infrastructure blast radius to
map. Isolation is at the crate boundary (Cargo's dependency graph) and,
within the crate, at the type boundary between the three components.

## Component inventory (in-process)

| Component | Failure domain | Isolation mechanism |
|---|---|---|
| `DesignOverlay` | A single edit's validation/application. A rejected width (BR2.1) affects only that one `LaneEdit` attempt — `EditOutcome{applied: false}` leaves every other state untouched. | Pure in-memory data structure; validation happens before any state mutation, so a rejected edit never partially applies. |
| `EditingSession` | Selection and undo-stack state for the active session. Undo failures (nothing to undo) are a reported outcome, never a panic. | The undo stack (SD-1) bounds memory growth; the session holds no reference to anything outside this Unit's own types plus read-only references into `street-core`/`street-import`. |
| `CorridorPlanner` | A single target street's fit-check or apply-preparation failure. Per BR10.1, one street's failure is isolated to that street's entry in `CorridorApplyOutcome.failed` — it does not propagate to sibling streets in the same corridor apply. | Each target street's fit assessment and apply preparation is an independent computation; `CorridorApplyOutcome` aggregates per-street results rather than short-circuiting on the first failure. |

## Shared resources

None owned by this Unit. `street-core`'s baseline types and
`street-import`'s `Correction`/`CorrectionOverlay` are read-only inputs;
this Unit never mutates them (the imported baseline is immutable per
`team.md` Code Style — a design is an overlay, never a mutation of it).

## Blast radius summary

The highest-consequence failure this Unit could introduce is an
unbounded undo stack or corridor selection exhausting the browser tab's
memory (SD-1) — bounded by construction, not by runtime monitoring
(this Unit has no monitoring surface of its own; a memory exhaustion
symptom would appear as a browser-level crash, out of this Unit's
control to detect). A single street's fit-check or apply failure within
a corridor apply is deliberately contained to that street (BR10.1) and
never escalates to abort the whole operation.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
