# Code Summary — `street-core` (U2)

## What was built

A new workspace member, `crates/street-core`, implementing the imported
street/lane baseline exactly as scoped by `entities.md`, `rules.md` and
`functional-spec.md`:

- `provenance.rs` — `Provenance` (closed 3-variant enum: `Mapped`,
  `Inferred`, `UserSet`) and `Dimension` (`{ metres: f64, provenance:
  Provenance }`) with a `corrected(&self, new_metres) -> Dimension`
  method that always returns `UserSet` (BR1.1, BR1.2, BR6.1).
- `lane.rs` — `Direction` (`Forward`/`Backward`), `LaneKey` (derived from
  `lane_type`, `direction`, `ordinal_from_kerb`), `Lane`, and `LaneList`
  (a bounds-checked wrapper over `Vec<Lane>` — `.get(i)` only, never
  `[i]`) (BR2.1, BR2.2).
- `street.rs` — `BoundingNodeIds` (a direction-normalised OSM node-id
  pair), `StreetIdentity` (`osm_way_id` + `BoundingNodeIds`), `Street`
  (identity, optional name, `LaneList`, computed `carriageway_width`),
  and the sole `compute_carriageway_width` function implementing BR4.1
  (BR3.1, BR4.1, BR5.1).
- `graph.rs` — `IntersectionId` and `StreetNetworkGraph` (a graph_id, its
  `Street`s, its intersections, and read-only adjacency between them).
- `source.rs` — the `StreetSource` trait (an associated `Error` type and
  `import_area(&self, area_id: &str) -> Result<StreetNetworkGraph,
  Self::Error>`), declared and object-safe, implemented by nothing in
  this crate (BR7.1).
- `lib.rs` — `#![forbid(unsafe_code)]` (SD-3) and the crate's public
  re-exports.

`crates/street-core/Cargo.toml` declares no `[dependencies]` and no
`[dev-dependencies]` — `cargo test` runs against the standard library
alone (SD-2). The root `Cargo.toml`'s existing `members = ["crates/*"]`
glob already covered the new directory, so no workspace-manifest edit
was needed for Step 1.1.

## TDD process followed

Strict Red→Green→Refactor per the affirmed `team.md` Testing Posture
(Methodology: tdd), once per module group as the plan laid out: for each
of `provenance`, `lane`, `street`, and `graph`/`source`, the test module
was written first, `cargo test -p street-core <filter>` was run and
confirmed to fail at compile time (types not yet declared), the failure
was recorded in `unit-test-instructions.md`, then the minimum
implementation was added and the same command re-run to green. All five
Red steps' compiler output is recorded verbatim in
`unit-test-instructions.md`.

## Key implementation decisions

- **`corrected` rather than `correct`.** The plan's Step 4.1 named the
  method `correct(&self, new_metres) -> Dimension`; it is implemented as
  `corrected` because it returns a new value rather than mutating
  `self` (BR5.1 immutability applies to `Dimension` too, since it is
  part of the imported baseline once inside a `Lane`/`Street`) — the
  past-participle name matches that shape more precisely. Behaviour is
  unchanged from the plan's description.
- **`Direction` has two variants (`Forward`, `Backward`).** `entities.md`
  specifies `direction: enum, required` without listing allowed values.
  Since this crate declares no dependency on osm2streets (BR7.1) and
  therefore cannot import its `Direction` type, a minimal closed enum
  covering the two directions relative to a street's digitised direction
  was chosen. If `u4-street-import`'s adapter needs a third value (e.g.
  bidirectional), that is this crate's decision to extend, flagged here
  for the architect/next Unit rather than invented silently.
- **Carriageway-width lane-type classification constants
  (`KERB_BUFFER_LANE_TYPE`, `VERGE_BUFFER_LANE_TYPE`,
  `WALKABLE_LANE_TYPES`) are this project's own canonical strings**
  (`"kerb_buffer"`, `"verge"`, `["sidewalk", "footway"]`), not literal
  osm2streets enum names. `rules.md` BR4.1 specifies the *calculation*
  precisely but not the exact lane-type-name strings a `Lane.lane_type`
  will hold, and this crate cannot depend on osm2streets to obtain its
  literal vocabulary (BR7.1). These constants are exported publicly so
  `u4-street-import`'s `StreetImportAdapter` has a fixed, single target
  to map osm2streets' own `LaneType` variants onto. **This is a decision
  that should be confirmed against osm2streets' actual `LaneType` naming
  when `u4-street-import` is built** — flagged here rather than left
  implicit.
- **Carriageway-width provenance aggregation** (not fully specified by
  `rules.md`, which only defines the *metres* calculation): the computed
  `Dimension`'s `provenance` is `Mapped` only when every contributing
  lane's width was itself `Mapped`, and `Inferred` otherwise (i.e. if any
  contributor is `Inferred` or `UserSet`). This directly operationalises
  AC6.2.3's "a wholly-inferred carriageway width is representable"
  requirement, and additionally ensures a derived value is never
  presented as more certain than its least-certain contributor — added
  two tests beyond the plan's 6-7 estimate to cover this explicitly
  (`carriageway_width_is_mapped_when_every_contributing_dimension_is_mapped`,
  `carriageway_width_is_inferred_when_any_contributing_dimension_is_inferred`).
- **No `trybuild` compile-fail test for BR5.1's "no `&mut` on `Street`"
  invariant.** The plan's Step 9.1 left this open ("decided at Green
  based on what's already in the dependency-free budget"). `trybuild` is
  a dev-dependency, and `security-design.md` SD-2 scopes this crate to
  zero dependencies beyond `std` — a single dev-dependency for one
  compile-fail assertion was judged not worth breaching that budget for.
  The invariant is instead documented as a doc comment on `Street`
  explaining why it holds at compile time (no `&mut self` method exists
  to call), consistent with how `rules.md` BR5.1 itself describes the
  enforcement ("a compile error — this Unit's public API has no mutable-
  access function to violate at runtime").
- **`LaneKey` derivation is inline in `Lane::new`, not a separate free
  function.** Plan Step 8.1 suggested extracting it "into one pure
  function shared by construction and any future re-derivation." There
  is currently exactly one construction path (`Lane::new`) and no
  re-derivation call site anywhere in this Unit's scope, so extracting a
  second function with the same three-line body would have added
  indirection without a second caller. If `u5-design-editing` later
  needs to re-derive a `LaneKey` independently of `Lane::new`, that is
  the point to extract it.
- **Module test placement:** the pinned toolchain's `clippy` (1.97)
  enforces `items_after_test_module`, which the plan's inline-`mod
  tests`-then-implementation layout (visible in the plan's own step
  descriptions) would have violated in every file except the one it was
  first caught in. All five modules place `#[cfg(test)] mod tests` after
  the production code, which is also the more common Rust convention.

## Test / coverage results

- `cargo test -p street-core`: **28 passed, 0 failed.**
- `cargo clippy -p street-core --all-targets -- -D warnings`: **clean.**
- `cargo fmt --all -- --check`: **clean** (repo-wide, not just this
  crate).
- `cargo llvm-cov -p street-core --fail-under-lines 80`: **94.84% line
  coverage** (exit code 0), no ignore pattern:

  | File | Line coverage |
  |---|---|
  | `provenance.rs` | 100.00% |
  | `source.rs` | 100.00% |
  | `street.rs` | 96.61% |
  | `lane.rs` | 93.55% |
  | `graph.rs` | 85.00% |
  | **TOTAL** | **94.84%** |

  The uncovered lines are almost entirely accessor methods this Unit's
  test suite had no need to call directly (e.g. `Street::name`,
  `Street::lane_count`, `StreetNetworkGraph::intersections`,
  `StreetNetworkGraph::streets_at`, `Lane::direction`,
  `Lane::ordinal_from_kerb`) — present because `entities.md` specifies
  them as attributes callers need, exercised transitively by
  construction but not asserted on individually since no rule or
  acceptance criterion in this Unit's scope calls for it. No coverage
  gap touches a rule (BR1–BR7) or traceable acceptance criterion; the
  floor is met with margin.

## Verification commands run (final)

```
cargo build -p street-core                                        # pass
cargo test -p street-core                                         # 28 passed
cargo clippy -p street-core --all-targets -- -D warnings          # clean
cargo fmt --all -- --check                                        # clean
cargo llvm-cov -p street-core --fail-under-lines 80                # 94.84%, exit 0
```

## Deviations from the plan

None that weaken a quality target. All deviations are additive (extra
tests, an inline rather than extracted helper, a doc comment instead of
a dev-dependency, module reordering for a clippy lint) or naming
precision (`corrected` vs `correct`), and are called out individually
above and inline in `code-generation-plan.md`'s checkboxes. The 80%
coverage floor, `clippy -D warnings`, `cargo fmt --check`, zero
dependencies beyond `std`, `#![forbid(unsafe_code)]`, and "no `&mut`
public method on `Street`/`Lane`/`StreetNetworkGraph`" targets from the
plan's "Quality targets carried in, not negotiable" section are all met
as stated, with no exceptions taken.

## Open items for the next Unit (`u4-street-import`)

- Confirm the `Direction` enum's two variants are sufficient for
  osm2streets' actual lane-direction vocabulary, or extend it.
- Confirm/align `KERB_BUFFER_LANE_TYPE`, `VERGE_BUFFER_LANE_TYPE`, and
  `WALKABLE_LANE_TYPES`' string values against osm2streets' real
  `LaneType` naming when the adapter is built, since this crate declared
  them without a live osm2streets dependency to check against.
