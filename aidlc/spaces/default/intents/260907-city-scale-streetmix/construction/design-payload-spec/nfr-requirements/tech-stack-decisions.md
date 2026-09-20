# Tech Stack Decisions — `design-payload-spec` (U3)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design,
this Unit); `contract-summary.md` Contract 3.

## Decisions

- **Language/crate**: Rust, a standalone workspace crate
  (`cityloom-design-payload`, the exact name `contract-summary.md`
  already commits to as "Source of truth: crate `cityloom-design-payload`").
  Reuses the workspace's existing `rust-toolchain.toml` (rustc/cargo
  1.97.1).
- **Serialization**: `serde` with `serde_json` (or an equivalent binary
  format if a later Unit's storage design prefers one — this Unit fixes
  the Rust type shape, not the wire encoding, per `contract-summary.md`'s
  framing of this as "the Rust types are the definition"). `serde` is
  the one dependency this crate needs beyond std, since the entire
  purpose of this Unit is a shape that crosses the device/server/export
  boundaries.
- **No dependency on `street-core`**: this crate does not depend on
  `street-core` despite sharing conceptually similar `StreetKey`/`Dimension`
  shapes — `unit-of-work.md` names this Unit's `kind` as `spec` carrying
  "None from the [component] catalogue," and `contract-summary.md`'s
  cross-check confirms `design-payload-spec` has no direct or transitive
  dependency edge onto `street-core` in the Unit dependency graph. The
  duplication between this crate's `Dimension`/`StreetKey` and
  `street-core`'s is intentional: one is the imported baseline's live
  representation, the other is the wire/storage shape, and coupling them
  would force every storage-format change to also touch the core domain
  model crate.
- **No dependency on `osm2streets`**: obviously — this crate is two
  layers removed from the adapter boundary.

## Rationale

Keeping this crate to `serde` alone (no database driver, no HTTP
client, no framework) matches its stated boundary: "the shape only."
Every consumer (`u7-local-persistence`, `u10-design-storage`,
`u12-data-rights`) brings its own storage/transport dependencies; this
crate stays a pure, reusable schema.

## Assumptions & Open Questions

- **[assumption]** JSON (via `serde_json`) is assumed as the concrete
  encoding for illustration; the actual choice between JSON and a binary
  format (e.g. for the server database column) is deferred to
  `u10-design-storage`'s own `nfr-requirements`/`tech-stack-decisions`,
  since this Unit fixes the Rust struct shape and `serde` derive traits,
  not the storage engine's column type.

## Traceability

See `traceability.json` in this directory.
