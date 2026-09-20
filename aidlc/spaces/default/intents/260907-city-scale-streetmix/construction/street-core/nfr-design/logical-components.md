# Logical Components — `street-core` (U2)

_Confirmed._

Bridges this Unit's NFR design with Infrastructure Design by giving a
component-level view of where the NFR patterns above apply.

## Component inventory

| Component | Role | Failure domain | Blast radius | Isolation strategy |
|---|---|---|---|---|
| `StreetModel` (this Unit's crate) | In-process domain-model library, compiled into the client WASM bundle | None at runtime — a panic here would be a compile-time-caught bug (SD-3 forbids `unsafe`, and the crate's own construction rules make invalid states largely unrepresentable) | Whatever consumes this crate (embedded in the same WASM binary) — there is no process or network boundary to contain a defect to this Unit alone | Crate-boundary isolation only (SD-2): no shared mutable state, no dependency surface for a defect to travel in through |

## Shared resources

None. This Unit owns no database, cache, queue, or other infrastructure
resource — it is a compiled-in library.

## What Infrastructure Design needs from this

Nothing beyond "this crate exists in the workspace and compiles as part
of the client WASM build" — there is no separate infrastructure resource,
deployment unit, or monitoring target for `street-core` itself.
`infrastructure-design` for this Unit is expected to be correspondingly
thin (see that stage's own artifacts).

## Traceability

See `traceability.json` in this directory.
