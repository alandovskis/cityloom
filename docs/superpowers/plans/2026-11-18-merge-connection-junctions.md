# One Road per Block Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The importer makes one road per block, each OSM piece a zone. The editor opens those zones, and every
street and junction edit saved before the change is discarded, with a one-time notice.

**Architecture:** A new pure pass in `citymodel_builder::importer` collapses runs of roads joined at plain
connection nodes into one zoned `Road`. `Layout::from_network` turns a road's zones into street zones. Saved
records gain an `import` stamp. One rule, `city::edits::is_stale`, keeps unstamped records from being applied,
synced, imported from an archive or restored from a milestone. The shell discards them when a project's page opens
and says so once.

**Tech Stack:** Rust (workspace), Leptos 0.8 CSR / wasm, serde_json, Fluent (`.ftl`), osm2streets.

**Spec:** `docs/superpowers/specs/2026-11-18-merge-connection-junctions-design.md`

## Global Constraints

- `cityloom_model::IMPORTER_VERSION`: 10 → 11.
- Stale cutoff: `pub const STALE_BEFORE_IMPORT: u32 = 11;` in `city/edits.rs`. It is fixed and never tied to `IMPORTER_VERSION`.
- New edits document version `pub const IMPORT_STAMP_VERSION: u32 = SCENARIO_VERSION + 1;` (17); `NEWEST` points at it.
- Product version `crates/cityloom_editor/Cargo.toml` 0.92.1 → 0.93.0, then `cargo update -p cityloom_editor`.
- Every new string goes in both `en.ftl` and `fr.ftl` with the same id. French follows the OQLF rules and `docs/glossary-fr-ca.md`; check the app's existing French word for "junction" (grep `junction/i18n/fr.ftl`) and use it.
- No new compiler warnings (`RUSTFLAGS=-D warnings` in CI). Run `just format` before every commit.
- Test first. Commit each task separately with the attribution lines from the session.
- Never set `CARGO_TARGET_DIR` to another worktree's target.

## Review Focus

1. Two pieces whose touching ends are not the same point: zone starts must still sit at each piece's first point along the merged line (test in Task 1).
2. A run whose two outer ends are the same node (a block that loops back to its own intersection): it must not become a road from a node to itself, which the editor drops (test in Task 1).
3. A record stamped by a newer build (`import: 12`) must not be stale (test in Task 3).
4. A stale record in a project never opened on this device must not be pushed by the sync (test in Task 4).
5. A zoned street opened when its first end is cut at the area's box: zones measured from the last end, in the right order (test in Task 2).

---

### Task 1: Merge connection nodes in the importer

**Files:**
- Create: `crates/citymodel_builder/src/importer/blocks.rs` (module doc: "Roads split at a plain connection: the pieces of one block are one road, each piece a zone.")
- Modify: `crates/citymodel_builder/src/importer/mod.rs` (`mod blocks;`, call it in `read_network` right after `merge::merge_dual_carriageways(&mut network);`)
- Modify: `crates/cityloom_model/src/importer.rs:37` (`IMPORTER_VERSION = 11`)
- Create: `crates/citymodel_builder/tests/importer/split_block.osm` (hand-written OSM XML): two ways named "Rue Test", `highway=residential`, sharing a middle node that has no other way. Way 1 has `lanes=2`, `maxspeed=30`; way 2 has `lanes=4`, `maxspeed=40`. A crossing way "Rue Croisée" meets each outer end, so both ends are real intersections. Copy the header and bounds style from `crossing.osm`.
- Test: `blocks.rs` `#[cfg(test)] mod tests`, `tests/importer/import.rs`

**Interfaces:**
- Produces: `pub fn merge_blocks(net: &mut Network)` in `importer::blocks` (crate-visible like `merge`).

The rule, verbatim from the spec. A node is merged away when all of these hold:
- `!junction`;
- `control == Control::None`;
- exactly two road ends meet at it (count `edge.from`/`edge.to` over `net.roads`);
- the two roads have equal `name` and equal `road_type`.

A run of roads joined by such nodes becomes one road, with these exceptions:
- A run that closes on itself with no outer end is left alone.
- A run whose two outer nodes are the same node is left alone.

The merged road is built as follows:
- **Id and direction:** those of the run's lowest `RoadId`. Pieces running against it are turned round (reuse the logic of `merge::reversed`; make it `pub(super)` and do not copy it).
- **Points:** concatenated, dropping the next piece's first point when it equals the previous piece's last point.
- **Zones:** one per piece, `from` = the length of the merged line before that piece's first point, with the piece's lanes and `speed_kmh`. Neighbouring zones with equal `lanes` and equal `speed_kmh` merge.
- **Provenance:** the union of the pieces' (`Provenance::extend`).

Then:
- The merged-away junctions are removed.
- A `Control::Priority` naming a piece names the merged road, once. Reuse the remap pattern in `merge.rs`, then `drop_unknown_priority` (make it `pub(super)`).
- `net.roads` stays sorted by id.

Keep the pass near-linear: an index from node to road ends, and a walk from each unvisited eligible road outwards.

- [ ] **Step 1: Write the failing unit tests in `blocks.rs`**

Helpers in the style of `merge.rs` tests (`node`, `lane`, a `two_way(id, name, from, to, points, lanes, speed)` road). Tests and their assertions:
- `two_pieces_of_one_block_are_one_road_of_two_zones`: nodes 1(x0) J, 2(x100) connection, 3(x250) J. Road 10: 1→2 with 2 driving lanes at 30. Road 11: 2→3 with 4 driving lanes at 40. Two cross roads at 1 and at 3 make them junctions. Assert:
  - one "Main" road, id 10, edge 1→3;
  - `zones.len() == 2`, `zones[1].from.metres() == 100.0`;
  - `zones[0].speed_kmh == Some(30)`, `zones[1].speed_kmh == Some(40)`;
  - lanes equal to the inputs;
  - node 2 is gone;
  - provenance has both ways.
- `a_chain_of_three_pieces_is_one_road_of_three_zones`: zone starts are `[0, 100, 250]`.
- `a_piece_drawn_the_other_way_is_turned_round`: road 11 stored 3→2 with lanes `[Forward sidewalk, Backward driving, Forward driving]`. The merged second zone has the lanes reversed and the directions swapped.
- `neighbouring_pieces_with_the_same_lanes_and_speed_are_one_zone`: `zones.len() == 1`, and the points are the whole line.
- `pieces_whose_ends_do_not_touch_keep_both_points`: piece 1 ends at (100,0) and piece 2 starts at (100,1). Points count = sum of both, and `zones[1].from.metres()` equals the length up to (100,1).
- `left_alone` cases, each asserting the network is unchanged (`assert_eq!(net, before)`):
  - `a_change_of_name_is_two_roads`
  - `a_change_of_road_type_is_two_roads`
  - `a_signalled_connection_keeps_its_node` (`control = Control::Signals`)
  - `a_node_where_three_roads_meet_is_kept` (junction false but degree 3: a fork)
  - `a_junction_with_two_named_roads_is_kept` (`junction: true`, degree 2)
  - `a_ring_of_connections_is_left_alone`
  - `a_block_that_comes_back_to_its_own_end_is_left_alone`
- `a_priority_naming_a_piece_names_the_merged_road`: node 3 `Control::Priority(vec![RoadId(11), RoadId(20)])` becomes `[RoadId(10), RoadId(20)]`.
- `merging_twenty_thousand_blocks_takes_under_two_seconds`: 10,000 two-piece blocks. Copy the timing style of `merge.rs`'s twenty-thousand test, and its `#[ignore]` or debug guard if it has one.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p citymodel_builder blocks`
Expected: compile error, `merge_blocks` not found.

- [ ] **Step 3: Implement `merge_blocks` and wire it into `read_network`; set `IMPORTER_VERSION = 11`**

- [ ] **Step 4: Run them to see them pass**

Run: `cargo test -p citymodel_builder blocks`
Expected: all PASS.

- [ ] **Step 5: Write the failing import tests in `tests/importer/import.rs`**

- `a_block_split_where_its_lanes_change_is_one_road_with_a_zone_for_each_piece`: import `split_block.osm`. Assert:
  - exactly one road named "Rue Test";
  - 2 zones;
  - the zones' driving-lane counts differ (2 then 4, or 4 then 2, depending on the road's direction);
  - the speeds are 30 and 40 in the same order;
  - provenance holds both way ids.
- `streets_of_two_names_meeting_end_to_end_share_a_corridor`: same file with way 2 renamed "Rue Autre" (a second fixture `split_name.osm`). Assert:
  - two roads;
  - a driver corridor (`corridors_of(UserClass::Driver)`) containing both road ids.

- [ ] **Step 6: Run them; they should pass with Task 1's code** (Step 5's tests describe behaviour Step 3 already built; if one fails, fix the code, not the test)

Run: `cargo test -p citymodel_builder --test importer`
Expected: PASS, including `real_city` and `clip_agrees`. If a threshold in those fails, investigate: the change may lower the share of places that follow (fewer, longer roads). Record the measured value and the reason in the commit message before changing any threshold.

- [ ] **Step 7: Add a cut test in `crates/cityloom_model/src/cut.rs` tests**

Test `a_zoned_road_cut_by_a_frame_keeps_each_zone_where_it_lies`. Copy the existing zoned-road tests near line 619. Use a 300 m road with a zone starting at 100 m, cut at x = 150. Each piece's zones start where the original's did, relative to the piece. If this passes at once, keep it as a pin.

Run: `cargo test -p cityloom_model cut`
Expected: PASS.

- [ ] **Step 8: Update `crates/citymodel_builder/CLAUDE.md`**

In the `importer` bullet, add the `blocks.rs` step: what is merged, the run order (after carriageways, before lanes and corridors), and that it raised `IMPORTER_VERSION` to 11.

- [ ] **Step 9: Format, commit**

```bash
just format && git add -A crates/citymodel_builder crates/cityloom_model && git commit -m "Merge the pieces of a block, split at plain connections, into one road with a zone per piece (importer 11)"
```

---

### Task 2: The editor opens a road's zones

**Files:**
- Modify: `crates/cityloom_editor/src/city/import.rs` (`Layout::from_network`, around lines 89–118)
- Test: `crates/cityloom_editor/src/city/import.rs` tests

**Interfaces:**
- Consumes: `cityloom_model::Road.zones` (Task 1), `Layout::zone_frame(&self, edge: usize) -> Option<ZoneFrame>` (`city/model.rs:378`, `pub(super)`), `Street::settle_zones(&mut self)`, `cityloom_model::street::Zone { from_mm, segments, name, speed }`.
- Produces: an imported `Street` whose fields are:
  - `zones` non-empty for a road of two or more zones;
  - `from_end`/`length_mm` from `zone_frame`;
  - each zone's `segments` = `pieces(zone.lanes)` with fresh uids from `next_uid`;
  - each zone's `speed` = `Speed::from_kmh(zone.speed_kmh)`;
  - `name: None`.

  After `settle_zones`, a one-zone road's street is exactly as today.

`zone_frame` needs the built layout, so fill zones in a second pass after `Layout { … }` is made: for each edge whose road has more than one zone, compute the frame, then set the section's zones. Positions:
- `from_mm = round(zone.from.metres() / road.length_m() * frame.length_mm)`.
- With `ZoneEnd::Last`, measure from the other end: the zone that starts at road distance `d` and ends at `e` starts at `length − e`. The list is ordered from that end, so the road's last piece comes first.
- When `zone_frame` is `None` (both ends cut), leave the street with its first zone only (known limit).

- [ ] **Step 1: Write the failing tests**

- `a_road_of_two_zones_opens_as_one_street_of_two_zones`. Build a `Network` with a 200 m road from an OSM-node end to an OSM-node end, zone 0 with 2 lanes at 30, and zone 1 from 80 m with 4 lanes at 40. Assert:
  - `layout.edges[0].section.zones.len() == 2`;
  - `zones[1].from_mm` ≈ 80 000 × `length_mm` / 200 000 (±1);
  - the base `segments` are the 2-lane pieces;
  - `zones[1].segments` is `Some` with the 4-lane pieces;
  - `zones[1].speed == Speed::Kmh(40)`.
- `a_road_of_one_zone_opens_as_before`: `zones.is_empty()`, `length_mm == None`.
- `a_zoned_road_whose_first_end_is_cut_is_measured_from_its_last_end`: give the first junction empty provenance. Assert:
  - `from_end == ZoneEnd::Last`;
  - zone 0 holds the road's last piece (4 lanes);
  - `zones[1].from_mm` ≈ (200 − 80) m scaled.
- `a_road_cut_at_both_ends_opens_with_its_first_zone`: `zones.is_empty()`.
- `a_zone_shorter_than_the_editor_minimum_is_kept`: zone 1 from 197 m (3 m long). `zones.len() == 2`.
- `an_unchanged_zoned_street_is_not_saved`. Through a `CityStore` with a test storage holding that network, `store.open().view(0).edited == 0`. Follow `city/store.rs` tests for the setup.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p cityloom_editor city::import`
Expected: the zoned tests FAIL (zones empty).

- [ ] **Step 3: Implement the second pass in `Layout::from_network`**

- [ ] **Step 4: Run to see them pass, then the whole crate**

Run: `cargo test -p cityloom_editor city:: && cargo test -p cityloom_editor`
Expected: PASS.

- [ ] **Step 5: Update `city/CLAUDE.md`**

In the import bullets:
- a zoned road opens as a zoned street, anchored as `zone_frame`;
- the known limit for a road cut at both ends.

- [ ] **Step 6: Format, commit**

```bash
just format && git commit -am "Open each zone of an imported road as a zone of its street"
```

---

### Task 3: Stamp records and never apply stale ones

**Files:**
- Modify: `crates/cityloom_editor/src/city/edits.rs`:
  - `StreetRecord` (64), `JunctionRecord` (91): field `#[serde(default, skip_serializing_if = "Option::is_none")] pub import: Option<u32>` (doc: "The `IMPORTER_VERSION` of the network the place came from; none in a record from before importer 11, which is stale.")
  - constants (16–57), `version_for` (147), the raw call sites (287, 411), `written_version` (124);
  - the golden list (1414–1528).
- Modify: `crates/cityloom_editor/src/city/model.rs`:
  - `street_record` (867), `junction_record` (912) set `import: Some(cityloom_model::IMPORTER_VERSION)`;
  - `apply_streets` (788), `apply_junctions` (810) skip stale records before `assign`.
- Modify: every other `StreetRecord {` / `JunctionRecord {` literal the compiler names (`city/store.rs:377, 390`, `sync/local.rs:357`, the `edits.rs` tests).
- Test: `city/edits.rs` tests, `city/model.rs` tests

**Interfaces:**
- Produces, all in `city::edits`:
  - `pub const STALE_BEFORE_IMPORT: u32 = 11;`
  - `pub const IMPORT_STAMP_VERSION: u32 = SCENARIO_VERSION + 1;`
  - `pub fn is_stale(record: &serde_json::Value) -> bool`: true unless `record["import"]` is a number ≥ `STALE_BEFORE_IMPORT`.
  - `fn holds_import(doc: &Map<String, Value>) -> bool`: any street or junction record with an `import` field. It becomes the first branch of `version_for`, giving `IMPORT_STAMP_VERSION`.

  Typed records are tested through `serde_json::to_value`, so there is one rule.

- [ ] **Step 1: Write the failing tests**

In `edits.rs`:
- `a_record_without_an_import_stamp_or_from_before_importer_11_is_stale`:
  - `is_stale(&json!({}))`;
  - `is_stale(&json!({"import": 10}))`;
  - `!is_stale(&json!({"import": 11}))`;
  - `!is_stale(&json!({"import": 12}))`;
  - `is_stale(&json!({"import": "11"}))`.
- `a_document_holding_a_stamped_record_is_written_as_the_import_stamp_version`: one `street_record()` with `import: Some(11)` gives `written_version() == IMPORT_STAMP_VERSION`. Without it, the version is as before.

In `model.rs`:
- `a_stale_street_record_is_not_applied`: copy an existing applied-edit test, with `import: None`. The street is as imported.
- `a_stale_junction_record_is_not_applied`.
- `a_record_the_city_writes_is_stamped_with_the_importer_version`: `city.street_record(e).unwrap().1.import == Some(IMPORTER_VERSION)`, and the same for junctions.
- `a_record_stamped_by_a_newer_importer_is_applied`: `import: Some(IMPORTER_VERSION + 1)` applies.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p cityloom_editor city::`
Expected: compile errors (`is_stale`, `import`).

- [ ] **Step 3: Implement**

- Add the field and fix the literals; tests that relied on applying old unstamped records get `import: Some(IMPORTER_VERSION)`.
- Add the constants, `is_stale`, `holds_import` and the version rule, following the "To add a version" order:
  - the constant;
  - `NEWEST`;
  - the first branch of `version_for` (new bool argument);
  - `holds_import` at both raw call sites and a typed bool in `written_version`;
  - the comment and doc lists.
- Skip stale records in `apply_*`.
- Add the golden lines `".streets.<key>.import: number"` and `".junctions.<key>.import: number"` at their sorted places in `GOLDEN`.

- [ ] **Step 4: Run to see them pass**

Run: `cargo test -p cityloom_editor`
Expected: PASS (including the golden test and `i18n_guard`).

- [ ] **Step 5: Update `city/CLAUDE.md`**

- Add version 17 `IMPORT_STAMP_VERSION` to the document-version list and make it `NEWEST`.
- Add a bullet: "A street or junction record carries `import`; one without, or below `STALE_BEFORE_IMPORT` (11, fixed), is stale: never applied, never listed or sent by the sync, not imported from an archive, and a milestone holding one cannot be restored. Raising the cutoff discards everyone's edits again: a decision, not a side effect of an importer bump."

- [ ] **Step 6: Format, commit**

```bash
just format && git commit -am "Stamp saved records with the importer they were made against and apply none from before importer 11"
```

---

### Task 4: Discard stale records locally and in the sync

**Files:**
- Modify: `crates/cityloom_editor/src/city/store.rs`
- Modify: `crates/cityloom_editor/src/sync/local.rs`:
  - `ids` (82) and `read` (93) skip a stale street/junction;
  - `write_in` (210) refuses a stale body (returns `true` without writing, so no retry loop; Task 4's vm change decides the tombstone first).
- Modify: `crates/cityloom_editor/src/sync/vm.rs` `apply` (478)
- Modify: `crates/cityloom_editor/src/sync/CLAUDE.md`
- Test: `city/store.rs` tests, `sync/local.rs` tests, `sync/vm.rs` (or `sync/tests.rs`, wherever `apply` is tested now)

**Interfaces:**
- Consumes: `edits::is_stale`, `edits::{STREETS, JUNCTIONS}`, `Draft::{raw_keys, get_raw, remove_raw}`, `sync::model::mark_all(storage, &[(Kind, String)], deleted: bool)`.
- Produces: `pub fn discard_stale(&self) -> usize` on `CityStore`, which:
  - removes every stale record in the store's (project) edits document;
  - writes it once;
  - after a successful write, calls `mark_all(…, true)` with them;
  - returns how many it removed;
  - returns 0, and writes nothing, for an empty, `Future` or damaged document.

In `sync/vm.rs::apply`, before `decide`: a live (`!remote.deleted`) Street/Junction `remote` whose body `is_stale` is answered with a tombstone, and `apply` returns `Applied::Done` without `Change`:
- `local::remove` any local copy;
- `doc.records.insert(id.name(), Entry { edited_at: stamp(storage.now_ms()), pending: true, deleted: true, rev: remote.rev, blocked: false })`.

Write the entry on the in-memory `doc`, not through `mark`, which `merge`/`absorb` would overwrite on save.

- [ ] **Step 1: Write the failing tests**

- `store.rs` `discarding_removes_every_stale_record_marks_each_deleted_and_keeps_the_stamped_ones`:
  - a document with a stale street, a stale junction and a stamped street;
  - `discard_stale() == 2`;
  - the document holds only the stamped one;
  - the sync document (`sync::model::open`) has both stale keys `pending && deleted`;
  - a second `discard_stale() == 0`.
- `store.rs` `discarding_leaves_a_newer_builds_document_alone`: `version: NEWEST + 1` gives 0, unchanged.
- `local.rs` `a_stale_record_is_neither_listed_nor_read`: a stale record in project `first` that was never opened. `ids` lacks it and `read` gives `None`.
- `local.rs` `a_stale_body_is_not_written`: `write` of a stale body leaves the document without it.
- vm `a_pulled_stale_record_is_not_kept_and_is_answered_with_its_removal`. With `FakeWorkSync` serving a live street record without `import`, after a pull:
  - the local document lacks it;
  - the sync entry is `pending && deleted`;
  - the next push sends `deleted: true` for that key.

  Follow the existing pull tests in the sync slice for the setup.
- vm `a_pulled_stamped_record_is_kept_as_before`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p cityloom_editor city::store sync::`
Expected: FAIL / compile error.

- [ ] **Step 3: Implement `discard_stale`, the `local` filters and the `apply` tombstone**

- [ ] **Step 4: Run to see them pass, then the crate**

Run: `cargo test -p cityloom_editor`
Expected: PASS.

- [ ] **Step 5: Update `sync/CLAUDE.md`**

Under "Record" or "Failures", add: "A stale street or junction record (`city::edits::is_stale`) is not listed or read; one pulled is not written and is answered with a tombstone."

- [ ] **Step 6: Format, commit**

```bash
just format && git commit -am "Discard stale records from a project's edits and answer a pulled one with its removal"
```

---

### Task 5: No stale record from an archive or a milestone

**Files:**
- Modify: `crates/cityloom_editor/src/archive/model.rs` `section` (278) or `read_edits` (290): drop stale Street/Junction records, so plans, counts and `apply` never see them.
- Modify: `crates/cityloom_editor/src/timeline/model.rs:70–82`: add `/// The milestone holds street or junction work from before importer 11, which would only be removed.` `Stale` after `Unusable`.
- Modify: `crates/cityloom_editor/src/timeline/work.rs:180` `restore`: right after the snapshot is read (182), `Err(Refused::Stale)` if any Street/Junction record of `records_of(&target)` `is_stale`.
- Modify: `crates/cityloom_editor/src/timeline/text.rs:44` `refused_key`: `Stale → "timeline-refused-stale"`.
- Modify: `crates/cityloom_editor/src/timeline/i18n/en.ftl`, `fr.ftl` (beside `timeline-refused-*`):
  - en: `timeline-refused-stale = This milestone was saved before the map was rebuilt with one street per block, so it cannot be restored. You can still compare it with now.`
  - fr: `timeline-refused-stale = Ce jalon a été enregistré avant que la carte soit reconstruite avec une rue par îlot ; il ne peut donc pas être restauré. Vous pouvez toujours le comparer avec l’état actuel.`

  Check `jalon`/`restaurer` against the existing `timeline` fr.ftl and the glossary; OQLF: no space before `;`. Fix the French to the existing terms if they differ.
- Test: `archive/` tests, `timeline/` tests

**Interfaces:**
- Consumes: `city::edits::is_stale`. Timeline may use `city::edits` (its CLAUDE.md); archive already reads `city::edits`.

- [ ] **Step 1: Write the failing tests**

- archive `a_stale_record_in_an_archive_is_not_imported`: an archive text with one stale and one stamped street. `parse_and_validate` then `plan_import` lists only the stamped one, and after `apply` the project's document lacks the stale one.
- timeline `a_milestone_holding_stale_work_cannot_be_restored`: a milestone whose snapshot holds a stale street. `restore(...) == Err(Refused::Stale)`, no backup milestone is created, and the documents are unchanged.
- timeline `a_milestone_of_stamped_work_restores_as_before` (existing tests may already cover this once their fixtures are stamped; stamp them).
- timeline text test: `refused_key(Refused::Stale) == "timeline-refused-stale"`, and the message exists in both languages (the parity test covers it).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p cityloom_editor archive:: timeline::`
Expected: FAIL / compile error.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run to see them pass**

Run: `cargo test -p cityloom_editor`
Expected: PASS (ftl parity, `i18n_guard`).

- [ ] **Step 5: Update `timeline/CLAUDE.md`** (`Refused` list, restore order) **and `archive/` docs if they list what is imported**

- [ ] **Step 6: Format, commit**

```bash
just format && git commit -am "Bring no stale record back from an archive, and refuse to restore a milestone holding one"
```

---

### Task 6: Discard at page open and say so once

**Files:**
- Modify: `crates/cityloom_editor/src/shell/vm.rs`: in `ShellVm::new_with`, when `target.project()` is `Some(id)`:
  - build `CityStore::current(Rc::new(ScopedStorage::new(ports.storage.clone(), id)))`;
  - call `discard_stale()`;
  - when it returns > 0, set `edits_cleared: ArcRwSignal<bool>` to true and say `shell-edits-cleared` through `ports.announcer`.

  Add `pub fn edits_cleared(&self) -> bool` (tracked) and `pub fn dismiss_edits_cleared(&self)`.
- Modify: `crates/cityloom_editor/src/shell/view.rs` `mount`: when `edits_cleared`, insert a `<p class="shell-notice" role="status">` holding the message and a `<button type="button">` worded `shell-notice-dismiss` that calls `dismiss_edits_cleared`. Put it before the page's `<main>`. Copy the markup and behaviour of `map::view::ReportNotice` (`map/view/mod.rs:263`).
- Modify: `crates/cityloom_editor/src/shell/i18n/en.ftl`, `fr.ftl`:
  - en `shell-edits-cleared = The map was rebuilt with one street per block, so your earlier street and junction edits were cleared.`
  - fr `shell-edits-cleared = La carte a été reconstruite avec une rue par îlot ; vos modifications antérieures des rues et des intersections ont donc été effacées.`
  - en `shell-notice-dismiss = Dismiss`
  - fr `shell-notice-dismiss = Fermer`

  Use the app's existing words for "junction" and "Dismiss" if they differ (grep `junction/i18n/fr.ftl`, `map/i18n/*.ftl` for the ReportNotice dismiss message and reuse its wording).
- Modify: `web/app.css`: `.shell-notice`, using existing colour tokens only (no new colour pair, so `shared/contrast.rs` is unchanged). If a new pair is unavoidable, add it to `contrast.rs`.
- Test: `crates/cityloom_editor/src/shell/tests.rs`

**Interfaces:**
- Consumes: `CityStore::discard_stale` (Task 4), `ScopedStorage::new` (`shared/scoped.rs:36`), `Target::project` (`shell/vm.rs:26`).

- [ ] **Step 1: Write the failing tests** (native, against `test_ports()`)

- `a_page_of_a_project_with_stale_edits_clears_them_and_says_so_once`. Root storage holds `cityloom-edits@p1` with one stale street; the target's project is `p1`. Assert:
  - `vm.edits_cleared()`;
  - the announcer heard the en message;
  - the scoped document is empty.

  Then build a second `ShellVm` on the same storage: `!edits_cleared()`.
- `a_project_without_stale_edits_shows_nothing`.
- `a_page_in_no_project_discards_nothing`: target project `None`; a bare `cityloom-edits` with a stale record is untouched.
- `dismissing_hides_the_notice`.
- A component test rendering the notice to HTML (`shared/testing.rs` helpers): `role="status"`, the message and the button, in English and in French.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p cityloom_editor shell::`
Expected: FAIL / compile error.

- [ ] **Step 3: Implement the vm, view, messages and CSS**

- [ ] **Step 4: Run to see them pass, then all**

Run: `just test` (needs `DATABASE_URL`: `just db` first)
Expected: PASS, including the wasm32 build.

- [ ] **Step 5: Format, commit**

```bash
just format && git commit -am "Clear a project's stale edits when one of its pages opens and say so once"
```

---

### Task 7: Version, fixtures, browser specs, docs

**Files:**
- Modify: `crates/cityloom_editor/Cargo.toml` (`version = "0.93.0"`), `Cargo.lock` (`cargo update -p cityloom_editor`)
- Modify: `crates/cityloom_editor/src/city/CLAUDE.md`, `crates/citymodel_builder/CLAUDE.md` (already edited; re-read for accuracy)
- Modify: e2e specs only where an expectation counted pieces this change merges.

- [ ] **Step 1: Bump the version**

Run: `cargo update -p cityloom_editor && grep -A1 'name = "cityloom_editor"' Cargo.lock`
Expected: `version = "0.93.0"`.

- [ ] **Step 2: Regenerate the fixtures and run every browser tier**

Run: `just e2e` (with `DATABASE_URL` from `just db`)
Expected: PASS. A failure is investigated with superpowers:systematic-debugging. Change an expectation only when it counted streets that are now one, and say which in the commit.

- [ ] **Step 3: Full check**

Run: `just check`
Expected: PASS with no warnings.

- [ ] **Step 4: Commit**

```bash
git commit -am "Raise the version to 0.93.0"
```
