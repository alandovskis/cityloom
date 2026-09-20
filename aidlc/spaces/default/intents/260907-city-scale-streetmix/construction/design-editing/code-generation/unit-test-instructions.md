# Unit Test Instructions — `design-editing` (U5)

## Framework and setup

Standard `cargo test`, reusing `rust-toolchain.toml`. No new test
framework or fixture files needed — this crate constructs synthetic
`street-core`/`street-import` values inline for every test.

## Exact unit-scoped run command

```bash
cargo test -p cityloom-design-editing
```

## Test scope (Standard strategy: 5-8 tests per component, 3 components)

**`DesignOverlay` (data model + business logic):**
1. Adding a lane constructs a `LaneEdit{kind: add}` with an anchor and
   no `target_lane_discriminator`, UserSet on every attribute (BR4.1,
   AC5.2.1).
2. Removing a lane constructs `LaneEdit{kind: remove}`; remaining lanes
   keep relative order (AC5.2.2).
3. A width change outside `(0, 20]` metres is rejected with a stated
   reason; the previous value is retained (BR2.1, AC5.1.5).
4. A valid width change sets UserSet provenance (BR3.1, AC5.1.3).
5. Reverting a changed attribute with no correction on it returns the
   imported baseline value and that value's own provenance (BR3.1,
   AC5.4.3).
6. Reverting a changed attribute where a correction exists on it
   returns the corrected value and its provenance, not the raw imported
   one (BR3.1, AC5.4.3).
7. Every `DesignOverlay` method's signature contains no `async` and no
   `Future`-returning type (BR1.1, AC5.1.2) — asserted structurally
   (e.g. via a `fn` pointer type-check in a test, or simply by the
   absence of `.await` anywhere in the crate, confirmed by inspection
   and a compile-time guarantee).

**`EditingSession` (business logic):**
8. Zero edits reports "nothing has changed," never an empty comparison
   (BR5.1, AC5.4.2).
9. A design with edits reports the three layers (baseline, corrections,
   design) distinguished (BR5.1, AC5.4.1).
10. Undo reverses the most recent single-street action and restores the
    prior state (BR6.1, AC5.5.1).
11. Undo on a corridor-apply `UndoEntry` reverses every affected street
    as one unit (BR6.1, AC5.5.2).
12. Undo with an empty stack reports "nothing to undo" without changing
    state (BR6.1, AC5.5.3).

**`CorridorPlanner` (business logic):**
13. The connected-street set is computed from a synthetic
    `StreetNetworkGraph`'s adjacency, and an unnamed connected street
    gets a stable human-readable fallback identifier (BR11.1, AC6.3.1,
    AC6.3.3).
14. The correspondence rule matches by `(lane_type, ordinal_from_kerb)`;
    an edit with no matching target lane is named unmatched and the
    target lane is untouched (BR7.1, AC6.1.2, AC6.1.3).
15. Extending onto a target leaves the target's own corrections and
    baseline intact — only a new design-layer entry is added (BR7.1,
    AC6.1.4).
16. Fit assessment returns `fits`/`does_not_fit` (with shortfall)
    correctly for a target whose carriageway width is at least
    partially `Mapped` (BR8.1, AC6.2.1).
17. Fit assessment returns `could_not_be_checked` for a target whose
    carriageway width is wholly `Inferred` or absent (BR8.1, AC6.2.3,
    AC6.2.4).
18. Applying past a `does_not_fit` warning applies every value verbatim
    — no width is scaled (BR9.1, AC6.2.2).
19. A bulk apply where one target street cannot be prepared: the other
    target streets still succeed, and the failed one is named with a
    reason in `CorridorApplyOutcome.failed` (BR10.1, AC6.4.2, AC6.4.3).

19 tests total (7 for `DesignOverlay`, 5 for `EditingSession`, 7 for
`CorridorPlanner`), within the Standard strategy's 5-8-per-component band.

## Coverage target

80% line coverage floor — this crate IS `team.md`'s explicitly named
"the editing state machine," one of the five crates/modules the
workspace `cargo-llvm-cov` invocation measures directly.

## Mocking/stubbing guidance

No mocking needed — this crate has no I/O. Tests construct synthetic
`street-core`/`street-import` values (a minimal `Street` with a few
`Lane`s, a minimal `StreetNetworkGraph`, a synthetic `Correction`) inline
using those crates' real public constructors.

## Test data management

All test fixtures are constructed inline in Rust; no external test-data
files. Builder-style helper functions in the test module keep the
synthetic `Street`/`Lane`/`StreetNetworkGraph` construction readable
across the 19 tests.
