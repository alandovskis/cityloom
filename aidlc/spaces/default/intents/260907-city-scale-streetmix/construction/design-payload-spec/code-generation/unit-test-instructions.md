# Unit Test Instructions — `design-payload-spec` (U3)

## Framework and setup

Standard `cargo test` (built into the toolchain already pinned by
`rust-toolchain.toml`, rustc/cargo 1.97.1, reused unchanged from
`osm-extract-proxy`/`street-core`). No extra test framework crate is
needed. `serde_json` is a dev-dependency used only inside `#[cfg(test)]`
round-trip tests (never a runtime dependency of the crate's public API,
consistent with `tech-stack-decisions.md`'s framing of the encoding as
illustrative/deferred to `u10-design-storage`).

## Exact unit-scoped run command

```bash
cargo test -p cityloom-design-payload
```

This is scoped to only this crate's tests — it does not rerun
`osm-extract-proxy`'s or `street-core`'s suites.

## Test scope (Standard strategy: 5-8 tests per component)

- **Entity round-trip tests** (`src/lib.rs` or `tests/roundtrip.rs`),
  covering `DesignPayload` and its nested types:
  1. A full `DesignPayload` (with `design`, one `LaneEdit` of kind
     `change`, one `LaneEdit` of kind `add` carrying an `anchor`, one
     `Correction`) round-trips through `serde_json::to_string` /
     `from_str` byte-for-byte equal (BR2.1, AC8.1.1).
  2. A `Dimension` with `provenance: Inferred` round-trips with that
     provenance intact (BR3.1, AC11.2.3).
  3. A `Dimension` with `provenance: Mapped` and one with `UserSet`
     round-trip distinctly (BR3.1).
  4. A `LaneEdit` of kind `remove` (no `attribute`/`width`/`value`)
     round-trips (AC8.3.x — shape covers a removal).
  5. A `Correction` carries no `provenance` field in its serialized
     JSON at all (SD-2/BR4.1 — structural, not a value; assert on the
     serialized JSON's keys, not just deserialize/reserialize equality).
  6. A `Correction`'s `importFingerprint` round-trips with all of
     `wayTags`, `mapConfig` (with `country_code`, `driving_side`,
     `inferred_sidewalks`, `inferred_kerbs`), and `osm2streetsRevision`
     present (BR5.1, AC7.5.1).
  7. Field names on the wire use the exact camelCase Contract 3 names
     (`payloadVersion`, `boundingNodeIds`) — assert against the raw
     JSON string for at least one field, not just round-trip equality,
     since two independent camelCase typos could still round-trip
     against each other.

- **Fail-closed version check tests** (business logic layer):
  8. `payloadVersion: 1` with otherwise-valid fields parses to `Ok`.
  9. `payloadVersion: 2` (or any value other than 1) with
     otherwise-matching field shapes returns
     `Err(PayloadError::UnsupportedPayloadVersion(2))` — and the test
     asserts this using a fixture where every other field DOES match
     the current shape, so the test cannot pass by accident from an
     unrelated deserialization failure (NFR6.4.4, SD-1, BR1.1).
  10. Malformed JSON (not a version problem) returns a distinct
      `PayloadError` variant, never a panic.

10 tests total, within the Standard strategy's 5-8-per-component band
across the two components (data shape: 7 tests; version-check business
logic: 3 tests).

## Coverage target

80% line coverage floor (org default for `feature` scope) applies to
this crate via the workspace's `cargo-llvm-cov` invocation — this crate
is not itself one of `team.md`'s five explicitly named measured
crates/modules (street/lane domain model, provenance model, osm2streets
adapter, persistence layer, editing state machine), but it is a small,
fully-exercised leaf crate with no rendering/glue code to exclude, so
the same 80% floor is the practical bar and is expected to be
comfortably exceeded given every public type is exercised by the
round-trip and version-check tests above.

## Mocking/stubbing guidance

None needed — this crate has no I/O, no external services, no
database. Every test constructs values in-memory and serializes them.

## Test data management

Test fixtures are constructed inline in Rust (builder-style helper
functions in the test module), not loaded from external files — the
shapes are small and the whole point of these tests is asserting the
in-memory-to-JSON-and-back cycle, not characterising an external
system (unlike the osm2streets adapter's fixture-based tests in a
different Unit).
