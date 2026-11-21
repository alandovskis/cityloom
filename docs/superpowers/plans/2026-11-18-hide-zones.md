# Changing a street along its length, without zones: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the street page's zone strip and zone commands with a horizontal, to-scale plan of the whole street
where a person drags to pick a stretch and edits it; boundaries appear and join by themselves, and no wording names a
zone.

**Architecture:** The saved model (`cityloom_model::street::Zone`) is untouched. The editor (`street/model/`) gains a
*pick* (a stretch, or the whole street) that replaces "the selected zone" as what edits act on: an edit is computed on
the run shown (the zone at the pick's left end) and replayed onto every zone in the pick, splitting at the pick's ends
first and joining identical neighbours after. A pure `PlanView` feeds a pure SVG renderer; a `Plan` component and pure
pointer/keyboard state machines drive the existing `StreetVm` commands. The Zone level, zone tools and every zone word
then go.

**Tech Stack:** Rust 2024, Leptos 0.8 (CSR), wasm-bindgen, Fluent (`.ftl`), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-11-18-hide-zones-design.md` (read it first; this plan argues from it).

## Global Constraints

- Saved form unchanged: `Street.zones`, `Zone { from_mm, segments, name, speed }`, `from_end`, `length_mm`, record
  versions, sync. A saved zone name round-trips and is never shown.
- No joining on load: `cityloom_model::street::settle_zones`, `Street::settled()` and `Content::settle` keep today's
  behaviour; the editor's join runs only after an edit. (`Editor::from_street` pushes `rev-earlier` through `push`, so
  nothing in `push`/`Content::settle` may join.)
- `MIN_ZONE_MM` = 5 000 mm minimum stretch; positions snap to 100 mm (`snap`); 2.5D stretch `STRETCH_MM` 40 m.
- Positions shown or typed count from the plan's **left end = the street's first end**, whatever `from_end` is; the
  editor stays in anchored millimetres, converted at the view-model's edge.
- Words: "stretch" (en), "tronçon" (fr-CA, OQLF typography, add to `docs/glossary-fr-ca.md`). "school zone" / "zone
  scolaire" and "loading zone" / "zone de livraison" stay. Every new string in both `.ftl` files, same ids and variables.
- Test-driven, a commit per change once tests pass, `just format` before each commit, `RUSTFLAGS=-D warnings`.
- Four pull requests (phases below), each bumping the minor version in `crates/cityloom_editor/Cargo.toml` (planned
  0.94.0 → 0.95.0 → 0.96.0 → 0.97.0; rebased on the Transform dialog (0.94.0), the read-only mode (0.95.0) and the merged
  blocks (0.96.0), 0.97.0 → 0.98.0 → 0.99.0 → 0.100.0) with `cargo update -p cityloom_editor`.
- One worktree per checkout, no shared `CARGO_TARGET_DIR` (memory: stale binaries keep another checkout's paths).
- Accessibility WCAG 2.2 AA: every new text/control colour pair measured in `shared/contrast.rs`; focus always
  visible; 24 px targets (44 px on a coarse pointer).

## Review Focus

1. **A whole-street edit after the School template** (school stretch has fresh uids): widening "the sidewalk" must
   reach the school stretch through kind matching. Test in Task 2 and Task 5.
2. **A street anchored at its last end** (`from_end = Last`): the pick's left end is the anchored *far* end; the shown
   run, the plan's order, typed positions and announcements must all agree. Tests in Task 3 and Task 7.
3. **A saved street with two identical neighbouring zones** opens unchanged, snapshot equal to the input, and a click
   picks both as one run; an edit touching them joins them. Test in Task 4.
4. **An edit made while a drag is under way in the 2.5D** (gesture) with a pick inside one run: the split happens once,
   the gesture records one revision, Escape restores the street with no boundary. Test in Task 4.
5. **Undo after an edit that split and then joined**: undo restores the exact zones and the pick is clamped, not lost
   or out of range. Test in Task 4.

---

## Phase 1 (PR 1): the pick in the editor, no visible change

The zone UI keeps working on top of it: `select_zone(z)` becomes "pick zone z's span".

### Task 1: Piece matching shared by transitions and replays

**Files:**
- Create: `crates/cityloom_editor/src/street/model/matching.rs`
- Modify: `crates/cityloom_editor/src/street/model/transition.rs` (move `match_pieces` out), `street/model/mod.rs` (`mod matching;`)

**Interfaces:**
- Produces: `pub(crate) fn match_pieces(a: &[Segment], b: &[Segment]) -> Vec<(Option<usize>, Option<usize>)>` (moved
  unchanged: uid+kind, then kind left to right, longest non-crossing) and
  `pub(crate) fn counterpart(a: &[Segment], b: &[Segment]) -> Vec<Option<usize>>` (for each piece of `a`, its index in
  `b`, from `match_pieces`).

- [ ] **Step 1: Write the failing tests** in `matching.rs`:
  `counterpart_matches_by_uid_first` (same uids, different widths → `[Some(0), Some(1), Some(2)]`);
  `counterpart_falls_back_to_kind_left_to_right` (b has fresh uids, kinds sidewalk/travel/travel/sidewalk → each
  matched in order); `counterpart_leaves_a_piece_with_no_kind_in_b_unmatched` (a bike lane only in `a` → `None`).
- [ ] **Step 2:** `cargo test street::model::matching` → fails to compile (no module).
- [ ] **Step 3:** Move `match_pieces` and its tests' helper into `matching.rs`; add `counterpart`; `transition.rs` uses
  `super::matching::match_pieces`.
- [ ] **Step 4:** `cargo test street::model` → all pass (transition tests unchanged).
- [ ] **Step 5:** Commit "Move piece matching out of transitions, for replaying edits too".

### Task 2: Replaying a section edit onto another section

**Files:**
- Create: `crates/cityloom_editor/src/street/model/replay.rs`

**Interfaces:**
- Consumes: `counterpart` (Task 1).
- Produces: `pub(super) fn replay(before: &[Segment], after: &[Segment], target: &[Segment]) -> Replayed` (`missed` when a changed, removed or moved piece had no counterpart).

Semantics (decided here):
- Pieces of `before` found in `after` by uid: every field that differs between the two (kind, width, material, curb,
  tram, direction, variants) is set to `after`'s value on the target's counterpart; fields that did not change keep the
  target's value.
- A uid in `after` not in `before` (added): inserted into the target right after the counterpart of its left neighbour
  in `after` (or before the counterpart of its right neighbour, or at the same end), with the **same uid**.
- A uid in `before` not in `after` (removed): its counterpart is removed.
- Order: when the relative order of matched pieces changed (a move), the moved piece's counterpart is placed after the
  counterpart of its new left neighbour.
- A piece with no counterpart in the target leaves the target unchanged for that piece.
- Replay never refuses: refusals come only from the edit on the shown section, so "refused in any run" reduces to
  "refused in the shown run" (the spec's per-edit confirmation). It reports what it could not place:
  `pub(super) struct Replayed { pub section: Vec<Segment>, pub missed: bool }` is the actual return type.

- [ ] **Step 1: Write the failing tests:**
  `a_width_change_reaches_the_same_piece_and_nothing_else` (target's other widths kept);
  `a_change_reaches_a_piece_matched_by_kind_when_uids_differ` (target with fresh uids: sidewalk widened);
  `an_added_piece_lands_beside_the_same_neighbour_with_the_same_uid`;
  `a_removed_piece_is_removed_where_it_has_a_counterpart`;
  `a_moved_piece_moves_beside_the_same_neighbour`;
  `a_change_to_a_piece_the_target_lacks_leaves_the_target_as_it_was` (bike lane only in before/after).
- [ ] **Step 2:** `cargo test street::model::replay` → fails (no module).
- [ ] **Step 3:** Implement `replay` in `replay.rs`.
- [ ] **Step 4:** `cargo test street::model::replay` → pass.
- [ ] **Step 5:** Commit "Replay a section edit onto another section".

### Task 3: The pick

**Files:**
- Create: `crates/cityloom_editor/src/street/model/pick.rs`
- Modify: `street/model/mod.rs` (field `pick: Pick` on `Editor`, initialised `Pick::Whole`; `select_zone`, `with_frame`,
  `reselect`, `reset`, `undo`/`redo` keep `zone` and `pick` consistent), `street/model/zones.rs` (`ZoneRefusal`
  unchanged), `street/i18n/{en,fr}.ftl` (refusal messages below)

**Interfaces:**
- Produces:
  ```rust
  pub enum Pick { Whole, Stretch { from_mm: i32, to_mm: i32 } } // anchored mm, from < to
  pub struct Run { pub zones: std::ops::Range<usize>, pub from_mm: i32, pub to_mm: i32 } // anchored
  pub enum PickRefusal { NoAnchor, TooShort, OutsideStreet, Dragging, WholeRunsOnly }
  impl PickRefusal { pub fn said(self) -> Said } // keys pick-refused-no-anchor|too-short|outside|dragging|whole-runs
  impl Editor {
      pub fn pick(&self) -> Pick;
      pub fn pick_whole(&mut self);
      pub fn pick_stretch(&mut self, a_mm: i32, b_mm: i32) -> Result<Pick, PickRefusal>; // either order, anchored
      pub fn pick_run_at(&mut self, at_mm: i32) -> Result<Pick, PickRefusal>;           // anchored
      pub(super) fn runs(&self) -> Vec<Run>;            // maximal identical neighbours, cut-off zones left out
      pub(super) fn zones_in_pick(&self) -> std::ops::Range<usize>;
      pub fn left_mm(&self, anchored_mm: i32) -> i32;   // and its inverse:
      pub fn anchored_mm(&self, left_mm: i32) -> i32;   // identity for First, length − x for Last
  }
  ```
- `Editor.zone` stays and always equals the zone containing the pick's **left** end (`shown zone`); every existing
  reader of `self.zone` keeps working.

Rules: snap both ends (`snap`); an end within `MIN_ZONE_MM` of a street end or a boundary snaps to it; a result shorter
than `MIN_ZONE_MM` → `TooShort` (pick unchanged); a stretch equal to the whole length → `Pick::Whole`; no frame → only
`Whole` (`NoAnchor`); elsewhere → only whole runs (`WholeRunsOnly`; `pick_run_at` works); during a gesture → `Dragging`.
`select_zone(z)` = pick the span of zone z (kept for the zone UI until Phase 3). The pick is outside history; after
undo/redo/Start over it is clamped to the street (a stretch past the length becomes `Whole`).

- [ ] **Step 1: Write the failing tests** in `pick.rs`:
  `a_new_editor_picks_the_whole_street`;
  `a_stretch_snaps_to_100_mm_and_to_a_boundary_or_end_within_5_m`;
  `a_stretch_shorter_than_5_m_is_refused_and_the_pick_stays`;
  `a_stretch_as_long_as_the_street_is_the_whole_street`;
  `picking_a_run_takes_identical_neighbouring_zones_together` (two zones with equal sections and speed → one `Run`);
  `cut_off_zones_are_in_no_run_and_cannot_be_picked`;
  `a_street_with_no_frame_can_only_be_picked_whole`;
  `an_elsewhere_street_picks_only_whole_runs`;
  `the_shown_zone_is_the_one_at_the_picks_left_end_on_a_street_anchored_at_its_last_end`;
  `left_and_anchored_positions_convert_both_ways_for_either_end`;
  `the_pick_survives_undo_clamped_to_the_street`.
- [ ] **Step 2:** `cargo test street::model::pick` → fail.
- [ ] **Step 3:** Implement; add the five `pick-refused-*` messages to both `street/i18n` files
  (en: "Both ends of this street are cut by the edge of the map, so it can only be changed along its whole length." /
  "A stretch must be at least { $min } long." / "That position is outside the street." / "Finish the drag before
  picking another stretch." / "The changes along this street were drawn on a road of another length, so only a whole
  stretch that looks the same can be picked here.").
- [ ] **Step 4:** `cargo test street` → pass (zone tests untouched).
- [ ] **Step 5:** Commit "Pick a stretch of the street, or the whole street, outside the history".

### Task 4: Edits act on every zone in the pick, then join

**Files:**
- Modify: `street/model/mod.rs` (`edit`, `edit_content`, `end_gesture`, `set_speed`, `follow_street_speed`),
  `street/model/pick.rs` (`split_at_pick`, `join_alike`)

**Interfaces:**
- Consumes: `replay` (Task 2), `Pick`, `zones_in_pick`, `runs` (Task 3).
- Produces (on `Content`, in `pick.rs`):
  `fn split_at_pick(&mut self, pick: Pick, frame: ZoneFrame) -> std::ops::Range<usize>` (inserts zones at the pick's
  ends when inside a zone, copying section and speed and setting `from_end`/`length_mm` as `split_zone` does; returns
  the zones now covering the pick) and
  `fn join_alike(&mut self, around: std::ops::Range<usize>)` (joins adjacent zones in `around` widened by one each side
  whose `section`s are `same_pieces` and whose `speed_in` are equal; the earlier zone survives with its uids and name).
- `Editor` produces `fn edit_each<F: Fn(&mut Vec<Segment>) -> bool>(&mut self, label: Said, f: F) -> bool` (runs `f`
  on every picked section, for `make_pedestrian`) and `fn edit_uniform(&mut self, label: Said, section: Vec<Segment>) -> bool`
  (every picked zone gets `section`, for measures and templates; used in Task 5).

Algorithm of `edit(label, f)`: compute `after = f(shown)`; if unchanged → false. Then in one content edit: if the pick is
a `Stretch` and the street can be structured (`structure_frame(true)`), `split_at_pick`; for every zone in the pick
range that is not cut off, `section_mut(z) = if z == shown { after } else { replay(before, after, section(z)) }`; for a
`Whole` pick also replay onto the base so zones that followed it still do; then `join_alike(range)` unless elsewhere.
Inside a gesture the split happens on the first changing step only (the live content already holds it afterwards) and
`join_alike` runs in `end_gesture` before `push`. Speed: `set_speed` sets every picked zone (a `Whole` pick on a zoned
street sets `Street.speed` and every zone to `Undecided`); `follow_street_speed` sets every picked zone `Undecided`.
After the edit, `zone` and `pick` are recomputed (the pick keeps its anchored span). The zones where a replay
`missed` are kept on the editor as `pub fn missed(&self) -> Vec<(i32, i32)>` (anchored spans, cleared by the next edit)
for the view-model to say "Not changed from { $from } to { $to }: that stretch has no { $kind }." (`edit-missed`,
Task 8).

- [ ] **Step 1: Write the failing tests** (in `pick.rs` tests, using `Editor::with_frame`):
  `picking_alone_never_splits` (pick 40–80 m of 120 m, no edit → `snapshot().zones` empty);
  `an_edit_in_a_stretch_splits_at_its_ends_and_changes_only_inside` (three zones, middle changed, outer = before);
  `a_whole_street_edit_reaches_the_same_piece_in_every_run_and_leaves_a_piece_only_one_run_has` (sidewalk 3 m
  everywhere, middle bike lane intact);
  `an_edit_back_to_the_neighbours_section_joins_the_stretch_back` (edit then inverse edit → `zones` empty);
  `a_saved_street_with_identical_neighbouring_zones_opens_unchanged` (`Editor::from_street(..).snapshot() == input`) and
  `an_edit_touching_identical_neighbours_joins_them`;
  `a_drag_in_a_picked_stretch_splits_once_and_is_one_revision` and `cancelling_that_drag_leaves_no_boundary`;
  `undo_after_split_and_join_restores_the_zones_exactly_and_clamps_the_pick`;
  `the_speed_of_a_stretch_is_set_for_it_and_whole_street_speed_clears_the_zones_own`;
  `an_elsewhere_street_edits_every_zone_of_the_picked_run_without_splitting_or_joining`;
  `cut_off_zones_are_never_edited`;
  `a_change_to_a_piece_one_run_lacks_reports_that_run_as_missed`.
- [ ] **Step 2:** `cargo test street::model` → the new tests fail.
- [ ] **Step 3:** Implement as above. `make_pedestrian` switches to `edit_each`.
- [ ] **Step 4:** `cargo test --workspace --features test-provider` (`just test`) → all pass, including every existing
  zone test (a `select_zone(z)` pick is exactly zone z, so their behaviour holds).
- [ ] **Step 5:** Commit "Edits act on every zone in the pick and identical neighbours join".

### Task 5: Templates and measures on a pick, School in the middle of it

**Files:**
- Modify: `street/model/mod.rs` (`apply_measure`, `apply_template`), `street/model/school.rs`

**Interfaces:**
- Consumes: `edit_uniform`, `pick_stretch`, `set_speed`, `add_calming` (existing).

Rules: a measure or non-School template is arranged from the shown section and given, with one set of uids, to every
picked zone (`edit_uniform`), so the pick becomes one run. School: within the pick `[p0, p1]` (the whole street when
`Whole`), when `p1 − p0 ≥ SCHOOL_ZONE_MM + 2·MIN_ZONE_MM` the school stretch is the middle `SCHOOL_ZONE_MM`; else the
pick itself; it then sets the pick to that stretch, `edit_uniform(arranged)`, `set_speed(SCHOOL_KMH)` unless lower,
`add_calming` a table at the middle, and squashes into one revision (as today); the parts of the pick outside the school
stretch are untouched. The template report keeps its sentences; positions are now stretch positions (Phase 3 rewords).

- [ ] **Step 1: Write the failing tests:**
  `a_measure_on_a_stretch_makes_it_one_run_with_one_set_of_uids`;
  `the_school_template_on_a_long_stretch_makes_its_60_m_stretch_in_the_middle_and_leaves_the_rest`;
  `the_school_template_on_a_short_stretch_makes_the_stretch_itself_the_school_zone`;
  `a_whole_street_sidewalk_change_after_the_school_template_reaches_the_school_stretch`;
  `the_school_template_is_still_one_undo`.
- [ ] **Step 2:** run → fail.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `just test` → pass; `e2e/tests/templates.spec.ts` unchanged and passing (`just e2e tests/templates.spec.ts`).
- [ ] **Step 5:** Commit; bump to the next minor version (`cargo update -p cityloom_editor`), commit; open PR 1
  ("The street's edits act on a picked stretch") via `alandovskis:create-pr`.

---

## Phase 2 (PR 2): the plan replaces the zone strip and the rows

### Task 6: `PlanView`, the plan's data

**Files:**
- Create: `crates/cityloom_editor/src/street/model/plan.rs`
- Modify: `street/model/mod.rs` (`View.plan: PlanView`, built in `Editor::view`)

**Interfaces:**
- Produces (all positions **left** mm):
  ```rust
  pub struct PlanView {
      pub length_mm: Option<i32>,          // None: no anchor, drawn schematic
      pub runs: Vec<PlanRun>,
      pub boundaries: Vec<PlanBoundary>,
      pub objects: Vec<PlanObject>,
      pub pick: Option<(i32, i32)>,        // None: the whole street
      pub cut: [bool; 2],                  // left end, right end cut by the area's edge
      pub elsewhere: bool,
  }
  pub struct PlanRun { pub from_mm: i32, pub to_mm: i32, pub zone: usize, pub bands: Vec<PlanBand>, pub ok: bool }
  pub struct PlanBand { pub uid: u32, pub kind: usize, pub material: usize, pub width_mm: i32 } // section order = top to bottom
  pub struct PlanBoundary { pub at_mm: i32, pub zone: usize, pub taper: Option<(i32, i32)>, pub movable: bool }
  pub enum PlanThing { Curb { side: Side, cell: usize }, Stop { side: Side, cell: usize }, Calming { cell: usize }, Green { i: usize } }
  pub struct PlanObject { pub thing: PlanThing, pub from_mm: i32, pub to_mm: i32, pub side: Option<Side>, pub label: Said, pub chosen: bool }
  ```
  `zone` in `PlanRun` is the run's first zone (for `select`); a boundary's `zone` is the zone it starts (for
  `move_boundary`). `cut` needs the city's end info: add `cut: [bool; 2]` to `EndInfo` use or derive from `Editor.ends`
  (the builder reads `with_ends`). Objects come from the existing `View.curb`, `View.stops`, `View.calming`,
  `View.green` cells, positions converted with `left_mm`.

- [ ] **Step 1: Write the failing tests:** `runs_are_left_to_right_with_their_sections`;
  `a_street_anchored_at_its_last_end_is_drawn_first_end_left` (runs reversed in order and positions);
  `identical_neighbouring_zones_are_one_run_with_no_boundary`;
  `a_boundary_with_a_transition_carries_its_taper_at_true_length_on_the_host_side`;
  `objects_sit_at_their_places_on_their_sides`; `a_street_with_no_anchor_has_no_length_and_no_objects`;
  `the_pick_is_given_in_left_positions_and_none_for_the_whole_street`; `cut_ends_are_marked`.
- [ ] **Step 2:** run → fail. **Step 3:** implement. **Step 4:** `cargo test street::model::plan` → pass.
- [ ] **Step 5:** Commit "The plan's data: runs, boundaries and things along the street, first end left".

### Task 7: The plan as SVG

**Files:**
- Create: `crates/cityloom_editor/src/street/view/plan/mod.rs` (`plan_svg`), `view/plan/scale.rs`
- Modify: `web/editor.css` (plan classes, `--plan-pick` tint, `--plan-pick-edge`), `shared/contrast.rs` (new pairs)

**Interfaces:**
- Produces: `pub fn plan_svg(p: &PlanView, width_px: f64, i18n: &I18n, units: Units) -> String` and
  `pub fn mm_per_px(length_mm: i32, width_px: f64) -> f64` and `pub const MIN_PX_PER_M: f64` (the builder picks it;
  below it the plan's own box scrolls sideways) and `pub fn x_px(at_mm: i32, mm_per_px: f64) -> f64`.
- Markup contract (tests and e2e read it): root `<svg class="plan" data-length-mm=…>`; per run `<g class="plan-run"
  data-zone=…>` with one `<rect class="plan-band k-<kind-id> mat-<material>">` per band; boundaries `<g
  class="plan-boundary" data-zone=…>` (a taper as `plan-taper`); objects `<g class="plan-thing" data-thing="curb-left-0">`
  using the existing cell ids' pattern (`curb-left-0`, `stop-right-1`, `calming-2`, `green-3`); pick `<rect
  class="plan-pick">` with `plan-handle` at each end; cut ends `plan-cut`. Ground mm to px is linear; a schematic plan
  (no length) draws the shown run full width with both `plan-cut`. Uses `--mat-*` / `k-*` tokens already in `web/app.css`.

- [ ] **Step 1: Write the failing tests** (`view/plan/mod.rs` tests, string assertions as `iso` tests do):
  `one_band_per_piece_per_run`; `a_run_starts_where_its_left_boundary_is_in_px`; `the_pick_is_drawn_with_two_handles`;
  `no_pick_draws_no_handles`; `a_taper_is_drawn_at_its_true_length`; `things_carry_their_cell_ids`;
  `a_schematic_plan_has_no_length_attribute_and_two_cut_marks`; `a_long_street_keeps_the_minimum_scale`.
  In `shared/contrast.rs`: `plan_pick_edge_has_3_to_1_on_every_band_material`.
- [ ] **Step 2:** run → fail. **Step 3:** implement. **Step 4:** run → pass.
- [ ] **Step 5:** Commit "Draw the street's plan, first end left, to scale".

### Task 8: Picking from the view-model, by pointer and by keyboard

**Files:**
- Create: `crates/cityloom_editor/src/street/vm/plan.rs` (pure pointer machine), `street/vm/text.rs` additions
- Modify: `street/vm/keys.rs` (`on_plan`), `street/vm/mod.rs` (commands), `street/i18n/{en,fr}.ftl`

**Interfaces:**
- Consumes: `Editor::{pick_whole, pick_stretch, pick_run_at, left_mm, anchored_mm}` (Task 3), `PlanView` (Task 6),
  existing `select_cell::<R>`, `select_calming`, `select_green`, `begin_*_drag`/`drag_*_to`/`end_*_drag`,
  `begin_boundary_drag`/`drag_boundary_to`/`end_boundary_drag`.
- Produces:
  ```rust
  // vm/keys.rs
  pub enum PlanKey { Move(i32), To(bool /* last */), Extend(i32), ExtendTo(bool), PickRun, PickWhole, NudgeBoundary(i32) }
  // NudgeBoundary moves the boundary at the pick's left end (else its right end), as Alt+arrow moves a zone's start today: keyboard access to boundaries is kept
  pub fn on_plan(key: &str, shift: bool, alt: bool) -> Option<PlanKey>; // ←/→ ±1000, PageUp/PageDown ±10000, Home/End, Enter, Escape; Alt+←/→ (Shift+Alt 10 m) → NudgeBoundary
  // vm/plan.rs
  pub enum PlanPress { Street { at_mm: i32 }, Handle { left: bool }, Boundary { zone: usize }, Thing(PlanThing) }
  pub enum PlanGesture { Pending(PlanPress, f64), Picking { anchor_mm: i32 }, Handle { fixed_mm: i32 }, Boundary, Thing }
  pub fn press(p: PlanPress, x_px: f64) -> PlanGesture;
  pub fn moved(g: &PlanGesture, x_px: f64) -> bool; // past the drag threshold (`strip::is_boundary_drag`'s, moved here)
  // StreetVm (all positions LEFT mm)
  pub fn plan_point(&self) -> i32;                       // the keyboard point, outside history
  pub fn plan_key(&self, k: PlanKey);
  pub fn pick_stretch(&self, a_mm: i32, b_mm: i32) -> bool; // announces `stretch-picked` or the refusal
  pub fn pick_run_at(&self, at_mm: i32);
  pub fn pick_whole(&self);
  pub fn picked(&self) -> Option<(i32, i32)>;            // tracked, for the views
  ```
- Messages: `stretch-picked` "From { $from } to { $to }, { $length } long." / `stretch-picked-differs` "From { $from }
  to { $to }, { $length } long: { $what }." (what differs from the neighbours, from `matching` on the two sections:
  "bike lane added on the right", builder words a short list) / `stretch-whole` "The whole street, { $length } long." /
  `st-plan-label` "The street from its first end to its last, { $length } long" / `st-stretch` "From { $from } to
  { $to } · { $length } long" / `st-varies` "Varies along the stretch". Lengths via `Arg::Length` (never `format!`).
- Adding along the street: `add_curb`, `add_stop`, `add_calming` (and the Add menu's items for them) add at the start
  of the picked stretch when no free stretch is chosen in the rows: picking a stretch chooses, on each row, the free
  stretch containing the pick's left end (`Editor::select_curb(side, at)`, `select_stop`, `select_calming` with that
  anchored position), so today's "start of the chosen free stretch" is the pick's start. Green keeps
  `add_green_clear` (first clear place from the street's start) unchanged.
- After an edit, `Editor::missed()` non-empty → announce `edit-missed` after the edit's own announcement.
- Clicking the Street step of the panel path (`show_level(Level::Street)`) calls `pick_whole` (Phase 3 removes the
  Zone level; here it only adds the pick).

- [ ] **Step 1: Write the failing tests** (native, `test_ports()`):
  `on_plan_keys` (table: ArrowRight → Move(1000), Shift+PageDown → Extend(10000), Home → To(false), Enter → PickRun,
  Escape → PickWhole, Alt+ArrowLeft → NudgeBoundary(-1000), others None);
  `alt_arrow_moves_the_boundary_at_the_picks_left_end_and_the_pick_follows_it`;
  `picking_a_stretch_chooses_the_free_stretch_at_its_start_so_adding_a_stop_lands_there`;
  `a_change_that_missed_a_stretch_says_where`;
  `a_press_then_release_without_moving_picks_the_run_and_announces_it`;
  `a_drag_picks_from_press_to_release_in_left_positions_on_a_street_anchored_at_its_last_end`;
  `dragging_a_handle_moves_that_end_only`; `a_too_short_pick_says_why_and_keeps_the_pick`;
  `shift_arrows_extend_the_pick_from_the_point_and_escape_picks_the_whole_street`;
  `pressing_a_thing_selects_its_cell_and_opens_its_level`;
  `the_street_step_of_the_path_picks_the_whole_street`.
- [ ] **Step 2:** run → fail. **Step 3:** implement. **Step 4:** run → pass.
- [ ] **Step 5:** Commit "Pick a stretch from the view-model by pointer and keyboard".

### Task 9: The `Plan` component replaces the zone strip; the rows fold under it

**Files:**
- Create: `crates/cityloom_editor/src/street/view/plan/component.rs` (`#[component] pub fn Plan(vm: Rc<StreetVm>)`)
- Modify: `street/mod.rs` (`mount_street_page`: mount `Plan` into `#zones-slot` instead of `ZoneStrip`), `web/street.html`
  (wrap `#curb-slot`, `#calming-slot`, `#green-slot` in `<div class="plan-rows">`), `web/editor.css`
  (`.plan-rows` visually hidden unless `:focus-within`, then shown as the rows are today), `street/view/tests.rs`,
  `street/i18n_guard` lists, delete `street/view/strip.rs` (move `is_boundary_drag`, `pair_boundary_mm` to `view/plan/`)
- Test: `e2e/tests/fixtures.ts`, `e2e/tests/street.spec.ts` and every spec listed below

**Interfaces:**
- Consumes: `plan_svg` (Task 7), `PlanGesture`/`press`/`moved`/`on_plan` and the `StreetVm` pick commands (Task 8).
- DOM contract: `#plan` (`role="group"`, `aria-label` from `st-plan-label`) holding the SVG from `plan_svg` (set with
  `inner_html`, keyed so pointer capture survives a redraw) and `#plan-point` (`role="slider"`, the one tab stop,
  `tabindex=0`, `aria-valuemin=0`, `aria-valuemax` = length in m, `aria-valuenow` = point, `aria-valuetext` = the
  picked stretch's words, `aria-keyshortcuts`), `#plan-picked` (the visible `st-stretch` line). Pointer: press →
  `press`, move → `moved`/drag commands, up → click or end drag, Escape cancels a drag. Things: press selects
  (`select_cell::<R>` etc.), drag moves through the row's existing drag commands.
- e2e helpers in `fixtures.ts`: `streetLength(page)` reads `#plan svg[data-length-mm]`;
  `pickStretch(page, fromM, toM)` (focus `#plan-point`, Home, ArrowRight × fromM, Shift+ArrowRight × (toM − fromM));
  `pickWhole(page)`; `splitInto` is replaced by `makeStretches(page, count)` which picks each stretch and changes its
  speed so it differs (the specs that only needed boundaries keep working). `showZoneLevel` stays until Phase 3.

- [ ] **Step 1: Write the failing component tests** in `view/tests.rs`: `the_plan_mounts_in_place_of_the_zone_strip`
  (no `#zone-strip`, one `#plan`); `the_plan_point_is_one_tab_stop_with_slider_semantics`;
  `the_rows_sit_under_the_plan_in_a_plan_rows_wrapper`.
- [ ] **Step 2:** `cargo test street::view` → fail. **Step 3:** implement. **Step 4:** pass.
- [ ] **Step 5:** e2e, rewritten to the plan: in `street.spec.ts` the "the zones of a street" describe becomes "changing
  part of a street" with: drag picks a stretch (pointer, `page.mouse` on `#plan`), a click picks the run, Shift+arrows
  pick from the keyboard, an edit in the stretch makes a boundary that the plan shows, editing back joins it, a reload
  keeps the stretches, the plan at 375 px wide has no horizontal page scroll. Update the zone uses in `a11y`, `curb`,
  `calming`, `green`, `stops`, `speed`, `templates`, `junction`, `map`, `restrictions`, `report`, `editors` specs to
  the helpers (`grep -n -i zone e2e/tests/*.ts`). The pointer tests that drag row cells now drag `.plan-thing`s; the
  keyboard tests keep driving the row listboxes (shown on focus).
- [ ] **Step 6:** `just e2e-ui` and `just e2e` → pass. `just check` → pass.
- [ ] **Step 7:** Commit; bump minor version; open PR 2 ("A plan of the whole street replaces the zone strip").

---

## Phase 3 (PR 3): no Zone level, no zone words on the street page

### Task 10: The Zone level and zone commands go

**Files:**
- Delete: `street/view/zone_tools.rs`; `ZoneEntry` in `street/view/panel_path.rs`
- Modify: `street/vm/level.rs` (`Level::Zone` removed, `Piece` parent is `Street`, `Level::ALL` of 6),
  `street/vm/mod.rs` (remove `select_zone*`, `split_zone`, `begin_merge`/`pending_merge*`/`finish_merge`/`cancel_merge`,
  `name_zone`, `reset_zone`, `use_for_street`, `zone_entry`, the `merge` signal; `nudge_boundary` stays, used by
  `PlanKey::NudgeBoundary`), `street/mod.rs` (no
  `zone-entry-slot`/`zone-tools-slot` mounts), `web/street.html` (remove `#p-zone` and the `.zone-entry` row),
  `street/model/zones.rs` (`split_zone`, `merge_zone`, `name_zone`, `reset_zone`, `use_for_street`, `Keep`,
  `merge_needs_choice` become `#[cfg(test)]` or are deleted where nothing uses them; `move_boundary` stays)
- Test: `street/vm` tests, `view/tests.rs`, `e2e/tests/fixtures.ts` (`showZoneLevel` removed),
  `e2e/tests/speed.spec.ts` ("the details panel's path")

- [ ] **Step 1: Failing tests:** `the_path_is_street_then_piece_with_no_zone_step`;
  `the_details_panel_has_no_zone_level_markup`; `a_piece_level_goes_back_to_the_street_level_and_picks_the_whole_street`.
- [ ] **Step 2–4:** run → fail; implement; `just test` → pass; delete the model tests of removed commands only where the
  command is gone (their behaviour is covered by Tasks 3–5).
- [ ] **Step 5:** Commit "No Zone level: the path is the street, then what is chosen".

### Task 11: Street page wording names places, not zones

**Files:**
- Modify: `street/i18n/{en,fr}.ftl`, `street/vm/text.rs`, `street/model/zones.rs` (`zone_label*`, `label_of` →
  `stretch_said(z) -> Said` using left positions), `street/model/mod.rs` (`zone_checks`, `transition_checks`),
  `street/model/zone_speed.rs`, `school.rs`, `calming.rs` (`calming-instance-zone`), `street/view/page.rs`
  (`StreetSummary`: no `.zone-mini`, no zone count, `st-stretch` when the pick is not whole), `street/view/speed_view.rs`
  (hint → `st-speed-stretch-hint` "For the stretch from { $from } to { $to }."; `st-varies` when the picked runs differ),
  `street/model/mod.rs` `SegView.varies: bool` (the piece's width differs between its counterparts in the picked runs)
  and `street/view/inspector.rs` (`st-varies` under the width when `varies`),
  `i18n_guard.rs` lists, `docs/glossary-fr-ca.md` (tronçon)

**Interfaces:**
- Produces: `pub(super) fn stretch_said(&self, z: usize) -> Said` → `Said::new("stretch-mid").with("from", Arg::Length(l0)).with("to", Arg::Length(l1))`
  ("from 49 m to 82 m"; fr "de 49 m à 82 m"), positions via `left_mm`.
- Message changes (ids per the spec's **Wording** list): `check-zone-in` → `check-stretch-in` ("{ $check } from
  { $from } to { $to }"); `rev-zone-speed*` → `rev-stretch-speed*` ("Speed limit { $kmh } km/h { $stretch }"); `rev-zone-move`
  → `rev-boundary-move` ("Move a change along the street to { $to }"); `rev-zone-split|merge|name|unname|reset|use`,
  `zone-unnamed*`, `zone-selected`, `zone-refused-*` except `-dragging`→ removed or replaced by `pick-refused-*`;
  `zones-elsewhere` → `stretches-elsewhere` ("The changes along this street were drawn when it was { $drawn } long;
  here it is { $loaded }."); `svg-iso-label-zone|joint`, `svg-iso-transition`, `check-detail-transition-room`,
  `calming-instance-zone`, `template-school-*` (keep "school zone"), `transform-templates-hint` ("…the picked
  stretch…"), `src-applies-transition` ("Applies where a street's section changes along it: …"), `st-zones-*`,
  `st-zone-*`, `st-row-zones`, `st-street-sub*` (no zone count), `st-path-hint` ("Choose a piece, or pick a stretch on
  the plan, to change it."), `st-path-zone-entry` (removed), `zone-cut-off` → `plan-cut` ("Cut by the edge of the map").

- [ ] **Step 1: Failing tests:** `a_piece_whose_width_differs_across_the_picked_runs_says_it_varies` (model) and,
  in `street/vm/text.rs` tests (or `i18n` tests):
  `no_street_message_says_zone_except_school_and_loading_zones` — iterate every message id of `street::RESOURCES` in
  both languages, format with sample args, assert no `zone`/`Zone` outside the allowed phrases ("school zone", "loading
  zone", "zone scolaire", "zone de livraison"; fr checks `zone` too).
- [ ] **Step 2:** run → fails, listing the offending ids.
- [ ] **Step 3:** reword and rewire until it passes; parity and `i18n_guard` tests pass.
- [ ] **Step 4:** `just test`, `just e2e` (spec assertions on old wording updated: `grep -n -i "zone" e2e/tests/*.ts`).
- [ ] **Step 5:** Commit; bump minor; open PR 3 ("The street page says stretches, not zones").

---

## Phase 4 (PR 4): wording elsewhere and the docs

### Task 12: Map, junction, report, timeline and docs

**Files:**
- Modify: `map/i18n/{en,fr}.ftl` and its callers (`map-zones`, `map-street-sub-zones*`, `map-label-street-zones*` removed;
  the non-zone variants used instead; `map-report-place-zones` → "This street changes along its length. The figures
  show it at its first end."; `map-report-place-other` without "zones"), `junction/i18n/{en,fr}.ftl` and callers
  ("end stretch" for "end zone"; the next stretch named by `stretch_said`-style positions, not a zone label),
  `timeline/i18n` (`timeline-aspect-zones` → "changes along the street"), `website/docs/intro.md` (street row: no Zone
  step, the plan, picking a stretch), `website/i18n/fr/...` counterpart if present
- Keep: map ticks and bands (`overlay::zone_ticks`, `EdgeView.zone_breaks_mm`, `zone_bands_mm`); internal names.

- [ ] **Step 1: Failing test:** extend Task 11's test to every slice's `RESOURCES` (map, junction, timeline, city,
  shell): `no_message_anywhere_says_zone_except_school_and_loading_zones`.
- [ ] **Step 2:** run → fails listing ids. **Step 3:** reword and rewire. **Step 4:** `just test`, `just e2e`,
  `just docs-build` → pass.
- [ ] **Step 5:** Commit; bump minor; open PR 4 ("No zones in the map, junction, report, timeline or docs").

### Task 13: Folder notes

- [ ] Update `crates/cityloom_editor/src/street/CLAUDE.md` (Street zones → "Stretches and runs", the plan, the pick, the
  replay and join, no Zone level), `src/map/CLAUDE.md` and `src/junction/CLAUDE.md` where they describe zone wording.
  Commit with PR 4.
