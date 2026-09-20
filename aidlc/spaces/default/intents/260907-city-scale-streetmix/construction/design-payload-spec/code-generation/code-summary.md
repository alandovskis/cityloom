# Code Summary — `design-payload-spec` (U3)

## Files created

- `crates/cityloom-design-payload/Cargo.toml` — new workspace member.
  Runtime dependency: `serde = { version = "=1.0.229", features = ["derive"] }`
  only. Dev-dependency: `serde_json = "=1.0.151"` (test-only, never
  linked into a normal `cargo build`).
- `crates/cityloom-design-payload/src/lib.rs` — the six entity types
  (`DesignPayload`, `Design`, `StreetKey`, `Dimension`/`Provenance`,
  `LaneEdit`/`LaneEditKind`/`LaneEditAttribute`,
  `Correction`/`CorrectionState`, `ImportFingerprint`/`MapConfig`), a
  hand-rolled `JsonValue` type for the free-form `anchor`/`wayTags`
  sub-objects, `PayloadError`, and `DesignPayload::from_json`.

No workspace root `Cargo.toml` edit was needed — it already declares
`members = ["crates/*"]`, so the new crate is picked up automatically.

## Files modified

- `docs/dependencies.md` — added a `cityloom-design-payload` row and
  updated the `serde`/`serde_json` rows' "Used by" columns.

## Key implementation decisions

- **Fail-closed version check without a runtime `serde_json`
  dependency.** `tech-stack-decisions.md` requires `serde_json` to be a
  dev-dependency only, but `security-design.md`'s SD-1 requires
  `DesignPayload::from_json(&str)` to be a real production entry point
  that genuinely parses JSON text, verified by `cargo build` (not just
  `cargo test`). Since Cargo dev-dependencies are not linked into a
  plain build, `from_json` could not call `serde_json` without
  contradicting one of the two constraints. Resolved by implementing a
  minimal internal `serde::Deserializer` for `&str` inside the crate
  (`json_reader` module, ~250 lines, using
  `serde::forward_to_deserialize_any!` for primitive boilerplate) —
  zero new dependencies, `from_json` genuinely parses real JSON in a
  plain build, and `serde_json` still appears only in
  `#[cfg(test)]` code.
- `Correction` carries no `provenance` field, per SD-2/BR4.1 — its
  UserSet-ness is structural. Verified by a test that asserts the
  serialized JSON has no `provenance` key at all, not just by
  round-trip equality.
- `#[serde(rename_all = "camelCase")]` on every struct for Contract 3's
  wire naming; `MapConfig`'s four inner fields
  (`country_code`, `driving_side`, `inferred_sidewalks`,
  `inferred_kerbs`) are kept snake_case on the wire via an explicit
  override, matching `entities.md`'s stated shape exactly.
- `PayloadError` is a small closed enum (`UnsupportedPayloadVersion(u64)`,
  `Malformed(String)`) — no `serde_json`-native error type crosses this
  boundary.

## Test coverage summary

10 tests, all green:

```
$ cargo test -p cityloom-design-payload
running 10 tests
test tests::malformed_json_returns_a_distinct_error_variant_never_a_panic ... ok
test tests::inferred_dimension_round_trips_with_provenance_intact ... ok
test tests::mapped_and_user_set_dimensions_round_trip_distinctly ... ok
test tests::correction_serializes_with_no_provenance_key ... ok
test tests::import_fingerprint_round_trips_completely ... ok
test tests::remove_kind_lane_edit_round_trips ... ok
test tests::unsupported_version_is_rejected_without_constructing_a_payload ... ok
test tests::wire_field_names_are_exact_camel_case ... ok
test tests::full_design_payload_round_trips_byte_for_byte ... ok
test tests::version_one_parses_to_ok ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo build --workspace`, `cargo clippy -p cityloom-design-payload
--all-targets -- -D warnings`, and `cargo fmt --check -p
cityloom-design-payload` all clean. TDD Red state was recorded before
implementation (45 compile errors from the not-yet-existing types).

`cargo-llvm-cov` was not run per-crate for this Unit — it is a
workspace-level `justfile`/`scripts/verify.sh` concern per `team.md`,
not invoked per-Unit in Code Generation. Every public type and all
three `from_json` branches (accept / version-reject / malformed) are
exercised by the 10 tests; the line-coverage percentage was not
independently measured in this stage.

## Deviations from the plan

1. The internal `json_reader` module was not explicit in the plan or
   `entities.md` — it was required to satisfy the plan's own stated
   "serde_json dev-dependency only" constraint literally while still
   meeting SD-1's "from_json genuinely parses production JSON" intent.
   See "Key implementation decisions" above.
2. Step 10 as executed covers only `docs/dependencies.md` — writing
   `code-summary.md`, `source-manifest.json`, and `traceability.json`
   was retained as the orchestrator's responsibility (this document and
   its siblings), not the delegated subagent's, per this stage's
   protocol.

No `street-core`, `osm-extract-proxy`, or `cityloom-api-types` files
were touched.
