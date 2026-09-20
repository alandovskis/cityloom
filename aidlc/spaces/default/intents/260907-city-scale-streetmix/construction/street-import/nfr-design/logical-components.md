# Logical Components — `street-import` (U4)

_Confirmed._

Upstream inputs: `security-design.md` (this stage); `components.md`
(`ExtractFetcher`, `StreetImportAdapter`, `CorrectionOverlay`);
`tech-stack-decisions.md`.

This Unit's deployment model is "embedded — a crate in the client Cargo
workspace" (`unit-of-work.md`), not a deployed service, so there is no
service boundary, load balancer, or infrastructure blast radius to map.
The isolation this stage would normally describe at the infrastructure
level instead exists at the **crate boundary**, enforced by Cargo's
dependency graph (`team.md` Code Style's three-inward-pointing-layers
rule) — a compile-time guarantee, not a runtime one.

## Component inventory (in-process)

| Component | Failure domain | Isolation mechanism |
|---|---|---|
| `ExtractFetcher` | A single import's fetch. A timeout or transport failure produces an `ImportFailure` for that one import; it does not affect a concurrent or subsequent import. | No shared mutable state between calls — each fetch is an independent async operation. |
| `StreetImportAdapter` | A single import's conversion, including any osm2streets panic. | `std::panic::catch_unwind` (security-design.md SD-1) is the blast-radius boundary: without it, a panic inside osm2streets during conversion would unwind across the WASM/JS boundary and abort the entire client module — every open design, not just the one import. With it, the panic is caught, mapped to `ImportFailure::internal`, and only that one import fails. |
| `CorrectionOverlay` | A single correction's construction or reconciliation. Malformed correction state (e.g. an unresolved correction with no matching lane) is a data condition this component surfaces, never a panic. | Pure in-memory data structure with no I/O; failures are typed `Result`s, not exceptions. |

## Shared resources

None owned by this Unit. `street-core`'s `Street`/`Lane`/`StreetNetworkGraph`
types are constructed here but owned (and their invariants enforced) by
U2. The pinned `osm2streets` dependency is shared read-only workspace
infrastructure (U1's artifact), not a resource this Unit provisions or
manages.

## Blast radius summary

The single highest-consequence failure this Unit could introduce is an
uncaught panic escaping the adapter and aborting the WASM module for the
whole tab — which would take down every other Unit's in-progress state
(an open design in `design-editing`, unsaved edits) along with the one
failed import. SD-1's `catch_unwind` boundary exists specifically to keep
that blast radius to "one import fails," which is why it is treated as a
security design element (information-disclosure/DoS boundary) rather than
ordinary error handling.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
