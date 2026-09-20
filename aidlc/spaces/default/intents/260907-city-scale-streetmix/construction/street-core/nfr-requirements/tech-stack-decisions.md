# Tech Stack Decisions — `street-core` (U2)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design, this
Unit); `team.md` Deployment/Code Style.

## Decisions

- **Language/crate**: Rust, a standalone workspace crate (`street-core` or
  equivalent) in the same Cargo workspace `osm-extract-proxy` and
  `osm2streets-build` already established. No dependency on the
  osm2streets crate (`rules.md` BR7.1) — this crate compiles cleanly even
  before `u4-street-import`'s adapter exists.
- **Dependencies**: none beyond the Rust standard library. This Unit's
  entities (`Provenance`, `Dimension`, `Lane`, `Street`,
  `StreetNetworkGraph`) are plain Rust types with no serialization,
  networking, or persistence responsibility — those cross-cutting
  concerns belong to `u3-design-payload-spec` (the wire/storage shape)
  and the outer-ring Units that consume this crate. Adding `serde`
  derives here, for example, would be premature: nothing in this Unit's
  own scope needs to serialize a `Street`.
- **Toolchain**: reuses the workspace's existing `rust-toolchain.toml`
  pin (`rustc`/`cargo` 1.97.1), established by `osm-extract-proxy` and
  confirmed by `osm2streets-build`. No new toolchain requirement.
- **Testing**: `cargo test` only — no external test infrastructure
  needed for a pure in-memory domain model.

## Rationale

Minimal dependency footprint is itself a design property this Unit is
responsible for: `unit-of-work.md` describes it as depended on by nearly
every other client Unit, so any dependency added here is inherited by
all of them. `team.md`'s three-ring layering names this crate explicitly
as having "no dependency on the osm2streets crate, no map-rendering
crate, no UI framework crate, no jurisdiction constants" — this stage
extends that to "no dependency beyond std at all," since nothing in the
entity/rule set actually requires one yet.

## Assumptions & Open Questions

- **[assumption]** If a later Unit's design turns out to need this
  crate's types to implement `serde::Serialize`/`Deserialize` directly
  (rather than through `u3-design-payload-spec`'s separate wire shape),
  that is a scope change to be raised explicitly at that Unit's own
  `nfr-requirements`/`tech-stack-decisions`, not assumed here.

## Traceability

See `traceability.json` in this directory.
