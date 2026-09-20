# Code Summary — `design-editing` (U5)

## Files created

- `crates/cityloom-design-editing/Cargo.toml` — depends only on `street-core` and `cityloom-street-import` (both workspace paths), no third-party dependency.
- `crates/cityloom-design-editing/src/lib.rs`
- `crates/cityloom-design-editing/src/entities.rs` — the ten `entities.md` types: `Design`, `LaneEdit` (+ `LaneEditKind`, `LaneAttribute`, `Anchor`), `EditOutcome`, `EditFinding`, `EditSession`, `UndoEntry` (+ `UndoRecord`), `CorridorSelection`, `FitState`/`FitStatus`, `CorrespondenceResult`, `CorridorApplyOutcome`.
- `crates/cityloom-design-editing/src/design_overlay.rs` — `DesignOverlay` (add/remove/change_width, width validation, revert-through-corrections, `effective_lanes`).
- `crates/cityloom-design-editing/src/editing_session.rs` — `EditingSession` (selection, undo stack bounded at `MAX_UNDO_ENTRIES = 200` per SD-1, three-layer delta report, corridor-apply undo-as-one-unit).
- `crates/cityloom-design-editing/src/corridor_planner.rs` — `CorridorPlanner` (connectivity from `StreetNetworkGraph` adjacency, correspondence by `(lane_type, ordinal_from_kerb)`, four-state fit assessment, non-atomic bulk apply).

## Files modified

None beyond this stage's own artifacts. No existing crate touched.
`docs/dependencies.md` was not modified — no new third-party dependency.

## Key implementation decisions

- **`LaneEdit` carries three fields beyond `entities.md`'s literal list**:
  `lane_type: Option<String>`, `ordinal_from_kerb: Option<u32>`, and
  `direction: Option<Direction>`. These are required for BR7.1's
  correspondence rule to match by `(lane_type, ordinal_from_kerb)`
  without inventing a second string grammar parsed out of
  `target_lane_discriminator` — which itself deliberately encodes
  `"{lane_type}|{ordinal}"` (excluding direction). `direction` is needed
  separately to fully specify an added lane's identity for `kind: add`.
  Documented in place.
- **Two non-persisted call-return types added beyond the ten entities**:
  `design_overlay::RevertOutcome` (revert's return — `entities.md` has
  no dedicated revert-result type) and `editing_session::DeltaReport`
  (the three-layer delta's return). Both are implementation-detail
  return shapes, not domain state this Unit stores.
- **SD-1's undo-stack bound** (`MAX_UNDO_ENTRIES = 200`) is enforced in
  `EditingSession`, with a test confirming the oldest entry is dropped
  once the cap is exceeded.

## Test coverage summary

31 tests, all green (19 required + 12 additional):

```
$ cargo test -p cityloom-design-editing
running 31 tests
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo build -p cityloom-design-editing        # Finished cleanly
$ cargo clippy -p cityloom-design-editing --all-targets -- -D warnings   # clean
$ cargo fmt --check -p cityloom-design-editing  # clean, no diff
$ cargo build --workspace                        # Finished cleanly
```

Breakdown: 11 entity-construction tests, 7 `DesignOverlay`, 6
`EditingSession` (5 specified + the SD-1 bounded-undo-stack test), 7
`CorridorPlanner`.

TDD evidence: each of the three Red/Green/Refactor cycles (entities,
then `DesignOverlay`, then `EditingSession`, then `CorridorPlanner`) was
written test-first against not-yet-existing types, confirmed failing to
compile (`E0433`/`E0432`/`E0425`), then implemented to green, then
refactored (clippy-driven cleanups folded in) before moving to the next
layer.

## Deviations from the plan

1. `LaneEdit`'s field set extended beyond `entities.md`'s literal list
   (see "Key implementation decisions" above) — required to make BR7.1's
   correspondence rule implementable without duplicating parsing logic.
2. Two non-persisted return types (`RevertOutcome`, `DeltaReport`) added
   beyond the ten named entities — implementation detail, not new
   domain state.

## Deferred housekeeping

`docs/dependencies.md`'s internal-crate listing was not updated with a
`cityloom-design-editing` row — the code-generation subagent correctly
deferred this (no new third-party dependency to add), and the
orchestrator's own attempt to add the row afterward was refused by the
`aidlc-plan-approval-guard` hook (workspace-path writes are restricted
to the dispatched subagent once a plan is approved). No functional
impact — this is a documentation convenience row, not a build or test
dependency. Should be added the next time this file is touched.

## Gaps / honest limitations

- **`CorridorPlanner::assess_fit` cannot distinguish "every contributing
  lane is Inferred" from "at least one contributing lane is Inferred."**
  BR8.1 asks for `could_not_be_checked` only when *every* contributing
  lane's width is Inferred. `street-core::Street::carriageway_width()`
  exposes a single computed `Dimension` whose provenance is `Inferred`
  if *any* contributor is Inferred (a deliberate design choice in that
  crate, per its own doc comment, to never overstate certainty). This
  Unit has no access to `Street`'s private per-lane contributing set,
  and per `components.md` must not duplicate that computation — it is
  `street-core`'s sole owned definition. The implemented behavior
  therefore treats any Inferred carriageway as `could_not_be_checked`,
  a conservative superset of BR8.1's literal condition: correct for a
  wholly-Mapped or wholly-Inferred carriageway (both tested), but may
  over-trigger `could_not_be_checked` for a carriageway that is only
  partially Inferred, where BR8.1 would want a real `fits`/`does_not_fit`
  verdict. Resolving this would require either a new accessor on
  `street-core::Street` (out of this Unit's scope to add) or duplicating
  carriageway logic here (forbidden by `components.md`). Flagged for a
  later stage (or `street-core`'s own maintenance) to decide whether a
  stricter accessor should be added.
