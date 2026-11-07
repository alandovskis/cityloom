# Raised tables Implementation Plan (issue #147, PR 1 of 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raised tables and raised crossings as saved, undoable street elements with strip, tools, design checks and 2.5D rendering.

**Architecture:** A new `Street.calming: Vec<Calming>` copies the curbside-uses and transit-stops pattern file for file: model (`street/model/calming.rs`), shown instances (`calming_shown.rs`), editor operations on `Editor` with one revision per edit, a `CalmingRow` on the shared `row.rs`, a strip and tools view, 2.5D drawing in `street/iso/calming.rs`, and a new edits-document version.

**Tech Stack:** Rust, Leptos 0.8, the repo's own diagram layers, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-06-raised-tables-design.md`. Templates to read and follow, in this order: `src/street/model/curb.rs` and `stop.rs` (model, refusals, `curb_frame`, checks), `curb_shown.rs`, `src/street/row.rs`, `curb_strip.rs`, `curb_tools.rs`, `stop_tools.rs`, `src/street/iso/curbside.rs` and `stops.rs`, `src/city/edits.rs`; CLAUDE.md's passages 'Curbside uses', 'Transit stops', 'Persistence'.

## Global Constraints

- Placeholders (all UNVERIFIED, so marked in doc comments): `TABLE_HEIGHT_MM` 100, `RAMP_MM` 1500, `MIN_TABLE_MM` 6000 (plateau at least 3000), `MAX_TABLE_SPEED_KMH` 50, `CALMING_STOP_CLEAR_MM` 5000, `CALMING_JUNCTION_CLEAR_MM` 5000, default length 7000, snap `SNAP_MM` 100.
- `Street.calming` is `#[serde(default, skip_serializing_if = "Vec::is_empty")]`; a street without calming is saved, shown and drawn byte-identically to before (the 2.5D markup pins stay as they are).
- Edits-document version: `CALMING_VERSION = SIGNAL_VERSION + 1` (11, provisional; renumbered at rebase), written only while some street record holds `calming` (`holds_calming`, first in `version_for`), `NEWEST` follows; a newer document is neither read nor written over; sync needs no code change.
- No wording in `.rs` files (`src/i18n_guard.rs`): messages in `src/street/i18n/{en,fr}.ftl`, whole sentences with named placeables, plurals by selector, fr-CA per `docs/glossary-fr-ca.md` (add the new terms there). Lengths through `Units::*_in(.., locale)`.
- Dependency rule: only the street slice and `city::edits` change; the map and junction are untouched.
- `RUSTFLAGS=-D warnings`; run the WHOLE `cargo test` (with `i18n_guard` and the parity test) and `just format-check` before each commit that touches wording or CSS; commit each task separately. Version: a minor bump of the root `Cargo.toml` plus `cargo update -p cityloom_editor` in the last task (the number is settled at rebase).
- Disk: the worktree has its own `target/`; do not touch other worktrees' folders.

## Review Focus

- A table on a street whose two ends are both cut by the map's box: refused with the no-anchor reason, nothing saved.
- A table across a zone boundary: judged in each zone it overlaps, the failing zone named.
- A table on a street with a bus lane and a stop: allowed; a table over a stop's boarding area fails `calming-stop` naming the stop; five metres clear passes.
- Undo, redo, Start over and a cancelled drag restore the element and the selection exactly.
- The element's keyboard path: every action (choose, move, add, remove, change kind and length) works without a pointer and is announced.
- A street with no speed set: a note, never a fail.

---

### Task 1: The data, constants and persistence

**Files:** Create `src/street/model/calming.rs`; modify `src/street/model.rs` (field, re-exports), `src/city/edits.rs`; tests in both.

**Interfaces — produces:** `pub struct Calming { pub from_mm: i32, pub length_mm: i32, pub kind: Kind }` (serde), `pub enum Kind { Table, Crossing }` with `Kind::ALL: [Kind; 2]`, `id()` (`table`, `crossing`), the constants above, `pub enum CalmingRefusal { NoAnchor, Elsewhere, Dragging, Outside, Overlap, TooShort { min_mm: i32 }, Freeway, NoRoadway, TimeWindows }` with `said(self) -> Said` (keys `calming-refused-*`), `Street.calming: Vec<Calming>`, `city::edits::CALMING_VERSION`, `holds_calming(raw: &Value) -> bool`.

- [ ] **Step 1: Failing tests:** serde round trip of a street with `calming` (JSON shape pinned like the curb golden test); `a_street_without_calming_serialises_as_before` (no `calming` key); `kind_ids_are_stable`; edits: `a_record_with_calming_is_written_as_calming_version`, `a_document_without_calming_keeps_its_version`, `a_document_above_newest_is_not_read`, `a_calming_record_cannot_be_read_under_an_older_version_number`, the pinned-constants test updated (`CALMING_VERSION == 11`, `NEWEST == CALMING_VERSION`).
- [ ] **Step 2: Run** `cargo test street::model::calming city::edits`; expected: compile failure.
- [ ] **Step 3: Implement** the types, `Street.calming`, the version constant, `holds_calming`, `version_for` and `written_version` (calming first), mirroring how `holds_stops`/`STOP_VERSION` are done.
- [ ] **Step 4: Run** the covering tests and the whole `cargo test`; expected: pass.
- [ ] **Step 5: Commit** `feat(street): the data of a raised table and its saved version (#147)`.

### Task 2: Editing a street's calming elements

**Files:** Modify `src/street/model/calming.rs`, `src/street/model.rs` (Editor state: `State.calming_move`, settings `calming_sel`), `src/street/model/curb.rs` only if a shared `curb_frame` needs a small generalisation; tests.

**Interfaces — consumes:** Task 1's types; the editor's `curb_frame(allow_drag)`, `rev-*` revision labels, `State`. **Produces** on `Editor`: `add_calming(&mut self, kind: Kind, from_mm: i32, length_mm: i32) -> Result<usize, CalmingRefusal>` (index), `move_calming(&mut self, i: usize, from_mm: i32) -> Result<(), CalmingRefusal>`, `set_calming_length(i, length_mm)`, `set_calming_kind(i, kind)`, `remove_calming(i)`, `calming_free_stretches(&self) -> Vec<(i32, i32)>`, `calming_sel(&self) -> Option<usize>`, `select_calming(Option<usize>)` (a setting outside history), `View.calming: Vec<CalmingView>` (`from_mm`, `length_mm`, `kind`, `name: Said`, `selected: bool`, `elsewhere: bool`).

- [ ] **Step 1: Failing tests:** add snaps to 100 mm and keeps the list ordered by `from_mm`; each refusal (`NoAnchor` on a street with no real end, `Elsewhere`, `Dragging`, `Outside`, `Overlap`, `TooShort`, `Freeway`, `NoRoadway`, `TimeWindows`) with its exact said key; one revision per edit with a `rev-calming-*` label; `undo_redo_and_start_over_restore_the_elements_and_the_selection`; `a_cancelled_move_restores_the_place`; `an_elsewhere_street_shows_but_does_not_rearrange`; `a_reversed_street_keeps_the_same_physical_place` (`Street::reversed`).
- [ ] **Step 2: Run** `cargo test street::model::calming`; expected: fails.
- [ ] **Step 3: Implement** the operations the way `add_stop`/`move_stop`/`stop_replace` and `curb_sel`/`stop_choice` are written (history state, settings outside history).
- [ ] **Step 4: Run** the street model tests and the whole `cargo test`; expected: pass.
- [ ] **Step 5: Commit** `feat(street): add, move, change and remove raised tables (#147)`.

### Task 3: The design checks

**Files:** Modify `src/street/model/calming.rs`, `src/street/model.rs` (`View` assembly: `street.extend(self.calming_checks())`), `src/street/i18n/{en,fr}.ftl`; tests.

**Interfaces — produces:** `Editor::calming_checks(&self) -> Vec<Check>` with ids `calming-length`, `calming-speed`, `calming-stop`, `calming-junction`, present only when the street has an element, judged per zone overlapped, `check-label-calming-*`, `check-detail-calming-*` messages; uses `Street::end_stops`-style stop data and `EndInfo.reach_mm` as `stop-junction` does.

- [ ] **Step 1: Failing tests:** one per check passing and failing with its detail (length below `MIN_TABLE_MM`; speed 60 fails, 50 passes, no speed gives a note not a fail; stop overlap fails naming the stop, 5 m clear passes, 4.9 m fails; junction: inside the junction's reach plus clearance fails, outside passes); `a_zone_boundary_names_the_failing_zone`; `no_calming_adds_no_checks`; the same checks are given when the street is anchored at either end; English and fr-CA wording.
- [ ] **Step 2: Run** `cargo test street::model::calming`; expected: fails.
- [ ] **Step 3: Implement** the checks mirroring `stop_checks`/`curb_checks`; add the messages in both languages; glossary entries.
- [ ] **Step 4: Run** the whole `cargo test` (parity and `i18n_guard` included) and `just format-check`; expected: pass.
- [ ] **Step 5: Commit** `feat(street): design checks for raised tables (#147)`.

### Task 4: The view-model, strip and tools

**Files:** Create `src/street/calming_strip.rs`, `calming_tools.rs`; modify `src/street/row.rs` (`CalmingRow`), `vm.rs`, `text.rs`, `watch.rs`, `keys.rs` only if the strip keys need a case, `page.rs` (mount), `web/street.html` (slots `#calming-slot`, `#calming-tools-slot`), `web/editor.css`, `src/shared/contrast.rs` for any new colour pair, `src/i18n_guard.rs` lists, both `.ftl`; tests in `street/tests.rs`.

**Interfaces — consumes:** Task 2's editor API; `Row`, `StripView`, `Choice`, `drag_span`. **Produces:** `StreetVm::{add_calming, select_calming, move_calming, nudge_calming, begin_calming_drag, drag_calming_to, end_calming_drag, set_calming_length, set_calming_kind, remove_calming}` each announcing what it did or why it refused, `CalmingStrip` (`#calming-strip`, cells `#calming-<i>` and free stretches, `aria-activedescendant`), `CalmingTools` (`#calming-kind`, `#calming-length`, `#calming-add`, `#calming-remove`), the screen-reader words in `text::calming_text`.

- [ ] **Step 1: Failing tests:** native VM tests against the port fakes (each command, announcements, refusal words, drag one undoable step with Escape cancel, structure/drag refusals do not collide with the zone, curb and stop drags); component tests render the strip and tools (cells, ids, one tab stop, selection, free stretches, kind select values in `Kind::ALL` order, default length 7.0 m in the locale's units); `keys::on_strip` cases reused; a11y: cell size at least 24 px in CSS test pattern if one exists.
- [ ] **Step 2: Run** `cargo test street::tests::calming`; expected: fails.
- [ ] **Step 3: Implement** the VM commands, strip and tools components following `curb_strip.rs`/`curb_tools.rs` and `row.rs`, mount them, CSS, i18n.
- [ ] **Step 4: Run** the whole `cargo test`, `just format-check`, and the wasm32 workspace build; expected: pass.
- [ ] **Step 5: Commit** `feat(street): a strip and tools for raised tables (#147)`.

### Task 5: The 2.5D drawing

**Files:** Create `src/street/model/calming_shown.rs`, `src/street/iso/calming.rs`; modify `src/street/iso/mod.rs`, `raised.rs`, `surface.rs`, `markings.rs`, `scene/*` as needed, `web/editor.css` and `app.css` for any new tokens, `src/shared/contrast.rs`; tests in `street/iso/*`.

**Interfaces — produces:** `View.calming_shown: Vec<ShownCalming>` (the part of each element inside the 40 m stretch, in the section's direction whatever the anchor: `along_mm` range, `plateau` range, `kind`, `whole: bool`); the drawing: plateau raised by `TABLE_HEIGHT_MM` (drawn at `RISE` like slabs) over the roadway pieces, ramps between, markings stop at the ramp feet, zebra bars on a crossing's plateau, classes `iso-table`, `iso-ramp`, `iso-zebra`, a group titled with the element's name, not pressable; an element cut by the reach edge drawn whole or not at all; `iso_label` gains "N raised tables are shown" (`svg-iso-calming`).

- [ ] **Step 1: Failing tests:** `a_table_raises_the_roadway_tops_over_its_plateau`; `ramps_slope_between_the_levels`; `lane_markings_stop_at_the_ramp_feet`; `a_crossing_paints_zebra_bars`; `a_street_without_calming_draws_byte_identically` (the existing pins plus a golden comparison against the markup before this task); `an_element_cut_by_the_reach_is_whole_or_absent`; `the_table_is_not_pressable_and_has_a_title`; the reversed street gives the mirrored picture; contrast test for any new token; the 2.5D label text in both languages.
- [ ] **Step 2: Run** `cargo test street::iso`; expected: fails.
- [ ] **Step 3: Implement** the shown-instance mapping (same `Stretch` code as curbside uses) and the drawing hooks; keep `diagram()`'s layer order.
- [ ] **Step 4: Run** the whole `cargo test` and `just format-check`; expected: pass; existing 2.5D tests unchanged.
- [ ] **Step 5: Commit** `feat(street): raised tables in the 2.5D (#147)`.

### Task 6: End-to-end, version and docs

**Files:** Create `e2e/tests/calming.spec.ts`; modify `Cargo.toml`/`Cargo.lock` (minor bump), `CLAUDE.md` (a 'Raised tables' bullet in the style of the curbside and stops ones, and the version list in 'Persistence'), `docs/glossary-fr-ca.md`, `website/docs` pages if the street editor's features are listed there (and fr-CA).

- [ ] **Step 1: Failing spec** (open the street from `e2e/fixtures/open-ends.osm.pbf` as the curb and stop specs do): `a raised table is added with the tools and shows in the strip and the drawing`; `the keyboard adds, moves, changes and removes a table and announces each`; `a table over a transit stop fails its check and five metres clear passes`; `undo removes the table and redo brings it back`; `a table survives a reload`; `a street without calming is unchanged`; `the strip and tools follow the language` (fr-CA).
- [ ] **Step 2: Run** `E2E_PORT=8291 just e2e tests/calming.spec.ts`; expected: fails before the work is wired.
- [ ] **Step 3:** Fix what the browser shows (separate commits per bug), bump the version, write the docs.
- [ ] **Step 4: Run** `just format`, `just format-check`, the whole `cargo test`, the wasm32 workspace build with `RUSTFLAGS=-D warnings`, and the e2e specs `calming`, `street`, `shell`, `map`, `a11y`, `delivery` (no database specs); expected: pass.
- [ ] **Step 5: Commit** `feat(street): raised tables and crossings (#147, 0.48.0)`; the number is settled at rebase.
