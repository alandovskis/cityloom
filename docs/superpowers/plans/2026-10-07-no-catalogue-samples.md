# No Catalogue Samples Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove `SAMPLES`, `JUNCTION_SAMPLES` and `Names` from the product so that streets and junctions come only from the network, and a street carries a `StreetClass` instead of a template index.

**Architecture:** Node titles are derived from the incident streets. `StreetClass` (in `shared`) replaces every index into `SAMPLES`. The two template tables, and the constructors built on them (`Editor::new(n)`, `Street::sample`, `Junction::new(n)`), leave production code and live on only as `#[cfg(test)]` fixtures inside the model modules, so the roughly 230 existing test call sites keep their shape.

**Tech Stack:** Rust 2024 workspace, Leptos 0.8 (CSR), wasm-bindgen, serde, Playwright for e2e. `just test` runs `cargo test --workspace` plus the wasm32 build; CI runs with `RUSTFLAGS=-D warnings`.

**Spec:** `docs/superpowers/specs/2026-10-07-no-catalogue-samples-design.md`

## Spec deviations to confirm before executing

1. **Test fixtures.** The spec says tests build places through the importer from an `osm_network` fixture, and that `Layout::sample()`, `Street::sample` and `CityStore::sample` go. That is practical for the product code but not for the tests: `Editor::new(n)` has about 90 call sites, `Junction::new(n)` about 140, and the hand-made 21-node test city sits under dozens of assertions on exact widths, numbers and names. All of it is already `#[cfg(test)]`, so none of it ships. This plan therefore keeps the four street profiles and four junction layouts as `#[cfg(test)]` child modules of `street::model` and `junction::model`, and leaves `Layout::sample()`, `City::new()` and `CityStore::sample` as they are (adapted to `class` in Task 2). Replacing the hand-made city by an imported network is a separate, test-only change and is not in this plan.
2. **Roads with no lanes.** `Layout::from_network` currently gives a lane-less road a template section (`Street::sample(class)`). With no templates there is nothing to give it, so such a road is left out of the layout, like the roads already left out for having the same end twice. Task 2 pins this with a test.
3. **No `city/text.rs`.** The spec says the "end of X" phrasing moves to `text.rs`, but the city slice has none. The phrase builder stays beside `node_name` in `city/model.rs`, which already builds `edge_name`.

## Global Constraints

- Test-first: write the failing test, watch it fail for the expected reason, then implement. Commit each change separately once the tests pass.
- Run `just format` before each commit. CI runs `just format-check`, `just test` and `just e2e` with `RUSTFLAGS=-D warnings`, so a new warning (an unused import, a dead function) fails the build.
- `shared` must not depend on any slice. `StreetClass` therefore lives in `shared/catalogue.rs`.
- A slice may use another slice's `model`, never its `vm`, views or text.
- Logic stays in Rust. Do not add logic to `web/*.js`.
- Stored data is discarded, not migrated: bump `SAVE_VERSION` (`city/model.rs`) from 1 to 2 so an older save is left behind.
- Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Work in the worktree `.claude/worktrees/no-catalogue-samples`, branch `worktree-no-catalogue-samples`.

## Review Focus

- A road with no lanes at all: it must not panic and must not appear as an editable street (Task 2).
- An unnamed road (OpenStreetMap gives it no `name`) gets "Unnamed <highway>" and still derives sensible node titles (Task 1).
- A meeting of 3 to 5 streets the junction editor cannot draw is made a plain connection and must be titled "Connection on X", not the joined names (Task 1).
- A save written by the previous version (version 1, with `sample` fields) must load as a freshly imported city, not crash and not half-load (Task 2).
- A motorway arm at a junction: the "an arm may not be a freeway" rule must still read the street's class (Task 2).
- A street whose name equals another street's at the same node must be listed once in the title (Task 1).

---

### Task 1: Derive node titles from the incident streets; remove `Names`

**Files:**
- Modify: `src/city/model.rs` (`Names` at 25-31, `NodeDef.names` at 39, `junction_at`/`gate_at` at 42-48, `node_name` and `end_name` near 266-281)
- Modify: `src/city/import.rs` (`join` at 59-67, the `names` block at 166-184, the `use` at 7)
- Test: `src/city/import.rs` (`mod tests`)

**Interfaces:**
- Consumes: `Layout::edges_at(node: usize) -> Vec<usize>`, `EdgeDef.name: Option<String>`, `NodeDef.junction: bool`.
- Produces: `pub(super) fn node_name(&self, node: usize) -> String`, `pub(super) fn end_name(&self, node: usize) -> String` on `Layout`, now computed. `NodeDef` has no `names` field.

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/city/import.rs`:

```rust
    #[test]
    fn a_places_title_follows_the_names_of_the_streets_that_meet_there() {
        let mut layout = Layout::from_network(&crossing(), "Testville");
        let junction = layout.nodes.iter().position(|n| n.junction).unwrap();
        assert_eq!(layout.node_name(junction), "Side Road and Main Street");
        assert_eq!(layout.end_name(junction), "Side Road and Main Street");
        let gate = layout.nodes.iter().position(|n| !n.junction).unwrap();
        let street = layout.edges[layout.edges_at(gate)[0]].name.clone().unwrap();
        assert_eq!(layout.node_name(gate), format!("End of {street}"));
        assert_eq!(layout.end_name(gate), format!("the end of {street}"));
        // the title is not kept: renaming a street changes it
        for e in &mut layout.edges {
            if e.name.as_deref() == Some("Side Road") {
                e.name = Some("Renamed Road".into());
            }
        }
        assert_eq!(layout.node_name(junction), "Renamed Road and Main Street");
    }

    #[test]
    fn a_meeting_that_cannot_be_drawn_is_titled_as_a_connection_on_its_street() {
        // three roads all leaving to one side: no junction drawing fits them
        let city = City::from_network(&star(&[(100.0, 0.0), (100.0, 10.0), (100.0, 20.0)]), "Fan");
        let v = city.view(0);
        let node = v.nodes.iter().find(|n| n.x_mm == 0 && n.y_mm == 0).unwrap();
        assert!(!node.junction);
        assert!(node.name.starts_with("Connection on "), "{}", node.name);
    }
```

The first test calls `node_name` and `end_name` from a sibling module, so they have to be `pub(super)` (Step 3).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test city::import::tests::a_places_title -- --nocapture`
Expected: FAIL to compile with "method `node_name` is private". After making it `pub(super)` the first assertion fails on the renamed street, because the title was cached at import.

- [ ] **Step 3: Implement**

In `src/city/model.rs` delete the `Names` struct and the `names` field of `NodeDef`, and drop `, names: None` from `junction_at` and `gate_at`. Replace `node_name` and `end_name` and add the helpers:

```rust
    /// The distinct names of the streets meeting at `node`, in the order the edges are held.
    fn street_names_at(&self, node: usize) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for e in self.edges_at(node) {
            if let Some(n) = &self.edges[e].name {
                if !names.contains(n) {
                    names.push(n.clone());
                }
            }
        }
        names
    }

    /// Who the node's streets are, in a title: "Main Street and Side Road".
    fn joined(names: &[String]) -> String {
        match names {
            [] => String::new(),
            [one] => format!("{one} junction"),
            [a, b] => format!("{a} and {b}"),
            [a, b, c] => format!("{a}, {b} and {c}"),
            [a, b, rest @ ..] => format!("{a}, {b} and {} more", rest.len()),
        }
    }

    pub(super) fn node_name(&self, node: usize) -> String {
        let names = self.street_names_at(node);
        match (self.nodes[node].junction, names.first()) {
            (true, Some(_)) => Self::joined(&names),
            (true, None) => format!("Junction {}", self.junction_number(node)),
            (false, Some(street)) if self.edges_at(node).len() == 1 => format!("End of {street}"),
            (false, Some(street)) => format!("Connection on {street}"),
            (false, None) => "Edge of the map".to_string(),
        }
    }

    pub(super) fn end_name(&self, node: usize) -> String {
        let names = self.street_names_at(node);
        match (self.nodes[node].junction, names.first()) {
            (true, Some(_)) => Self::joined(&names),
            (true, None) => format!("Junction {}", self.junction_number(node)),
            (false, Some(street)) if self.edges_at(node).len() == 1 => format!("the end of {street}"),
            (false, Some(street)) => format!("a connection on {street}"),
            (false, None) => "the edge of the map".to_string(),
        }
    }
```

In `src/city/import.rs` delete `join`, delete the whole `let names = if junction { ... };` block and the `names: Some(names),` line, and remove `Names` from the `use super::model::{...}` line. The `distinct` vector built just above the block is then unused: delete it and the `incident` line too, and keep `let junction = ...`. Make `fn edges_at`, `fn junction_number` unchanged.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test city`
Expected: PASS, including the existing `a_street_has_the_lanes_and_name_of_its_road` and `names_say_where_a_street_goes`.

- [ ] **Step 5: Format, full test, commit**

```bash
just format && just test
git add -A src/city
git commit -m "refactor(city): a place's title comes from the streets that meet there

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `StreetClass` replaces the template index on streets and edges

**Files:**
- Modify: `src/shared/catalogue.rs` (add `StreetClass`; `Sample` and `SAMPLES` stay until Task 5)
- Modify: `src/street/model.rs` (`Street.sample` at 156, `imported` 186-201, `title` 211-213, `sample` 216-222, `is_sound` 264, `Editor.sample` 284, `snapshot` 379, `from_street` 396, `SAMPLES[self.sample].freeway` at 836, 1017, 1028, `Street` view struct at 1242)
- Modify: `src/city/model.rs` (`EdgeDef.street` 55-56, `trim_at` 573, `freeway` 597, `edge_name` 292, `junction_fits` 347, `City::on` 417, `SAVE_VERSION` 20, `load_on` 448, `keep_street` 506)
- Modify: `src/city/import.rs` (`class_of` 16-27, `Layout::from_network`)
- Modify: `src/junction/model.rs` (freeway checks at 907 and 1021, `Arm::street_index` 315)
- Test: `src/city/import.rs`, `src/city/model.rs`, `src/street/model.rs`

**Interfaces:**
- Produces: `shared::catalogue::StreetClass { Motorway, Arterial, Collector, Local }` with `fn is_freeway(self) -> bool`, `Serialize`/`Deserialize` as lowercase strings, `Clone, Copy, Debug, PartialEq, Eq, Hash`.
- Produces: `Street.class: StreetClass`, `EdgeDef.class: StreetClass`, `Editor.class: StreetClass` (private), `Street::imported(class: StreetClass, side: Side, pieces: &[Piece]) -> Street`.
- Changes: `EdgeDef.name` becomes `String`; a road without lanes is left out of the layout.

- [ ] **Step 1: Write the failing tests**

In `src/city/import.rs` `mod tests`:

```rust
    #[test]
    fn a_road_is_classed_by_its_highway() {
        use crate::shared::catalogue::StreetClass::*;
        for (highway, class) in [
            ("motorway", Motorway),
            ("motorway_link", Motorway),
            ("trunk", Arterial),
            ("primary_link", Arterial),
            ("secondary", Collector),
            ("tertiary_link", Collector),
            ("residential", Local),
            ("service", Local),
            ("footway", Local),
        ] {
            assert_eq!(class_of(highway), class, "{highway}");
        }
    }

    #[test]
    fn a_road_with_no_lanes_is_not_a_street() {
        let mut net = crossing();
        net.roads[0].lanes.clear();
        let city = City::from_network(&net, "Bare");
        assert_eq!(city.view(0).edges.len(), 3, "the lane-less road is left out");
    }

    #[test]
    fn a_motorway_is_a_freeway_and_a_residential_street_is_not() {
        let mut net = crossing();
        net.roads[1].highway = "motorway".into();
        let v = City::from_network(&net, "Fast").view(0);
        let freeways = v.edges.iter().filter(|e| e.freeway).count();
        assert_eq!(freeways, 1);
    }
```

In `src/city/model.rs` `mod tests`:

```rust
    #[test]
    fn a_save_from_before_street_classes_is_left_behind() {
        let old = r#"{"version":1,"streets":{"1":{"sample":0,"name":null,"row_mm":1,"side":"Right","segments":[],"next_uid":1}},"junctions":{}}"#;
        let fresh = City::new().view(0);
        assert_eq!(City::load(old).view(0).edited, fresh.edited);
        assert_eq!(City::load(old).view(0).failing, fresh.failing);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test city::import::tests::a_road`
Expected: FAIL to compile (`StreetClass` and `class_of` returning a class do not exist yet).

- [ ] **Step 3: Implement**

Add to `src/shared/catalogue.rs` near the other enums:

```rust
/// What sort of street a road is. It decides what a street may be: a motorway is limited-access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreetClass {
    Motorway,
    Arterial,
    Collector,
    Local,
}

impl StreetClass {
    /// A limited-access road: no sidewalks, and a median and shoulders instead.
    pub fn is_freeway(self) -> bool {
        self == StreetClass::Motorway
    }
}
```

(Match the `use serde::...` already at the top of the file.)

Replace `class_of` in `src/city/import.rs`:

```rust
/// The class of a road, by OpenStreetMap's `highway`.
fn class_of(highway: &str) -> StreetClass {
    match highway {
        "motorway" | "motorway_link" => StreetClass::Motorway,
        "trunk" | "trunk_link" | "primary" | "primary_link" => StreetClass::Arterial,
        "secondary" | "secondary_link" | "tertiary" | "tertiary_link" => StreetClass::Collector,
        _ => StreetClass::Local,
    }
}
```

In `Layout::from_network`, filter lane-less roads together with the self-loops: change `let roads: Vec<_> = network.roads.iter().filter(|r| r.from != r.to).collect();` to `... .filter(|r| r.from != r.to && !r.lanes.is_empty()).collect();`. Build `EdgeDef { class, name: street_name(r), section: Some(Street::imported(class, side, &pieces).named(&street_name(r))), ... }`, so `section` and `name` are no longer optional on imported edges.

`src/street/model.rs`: rename `Street.sample: usize` to `pub class: StreetClass`; `Street::imported(class: StreetClass, ...)` stores it; `Street::title()` returns `self.name.clone().unwrap_or_default()`; `is_sound` drops the `self.sample < SAMPLES.len()` test; `Editor.sample` becomes `class: StreetClass`; every `SAMPLES[self.sample].freeway` becomes `self.class.is_freeway()`; `snapshot` and `from_street` copy `class`. `Street::sample(sample, side)`, `Editor::new(sample)` and `load_sample` keep their names for now but set `class` from a `fn class_of_sample(sample: usize) -> StreetClass` table `[Local, Arterial, Local, Motorway]` placed next to `SAMPLES` in `shared/catalogue.rs`; Task 5 moves both under `cfg(test)`.

`src/city/model.rs`: `EdgeDef.street: usize` becomes `class: StreetClass`, `name: String`, `section: Street`. `City::on` uses `e.section.clone()` directly. `trim_at` uses `self.today_streets[&uid].row_mm / 2` (the street's own width). `EdgeView.freeway` is `e.class.is_freeway()`. `edge_name` uses `e.name`. `junction_fits` drops `a.street < SAMPLES.len()` (the arm field itself goes in Task 3). Set `const SAVE_VERSION: u32 = 2;` and in `load_on` compare `t.class == street.class`. The `#[cfg(test)]` hand-made layout builds its `EdgeDef`s through a helper `fn street(a, b, fixture) -> EdgeDef` that fills `class`, `name` (from `SAMPLES[fixture].name`) and `section: Street::sample(fixture, Side::Right)`; make the helper a plain `fn`, not `const fn`.

`src/junction/model.rs`: the two freeway tests become `street_class(street).is_freeway()` using `class_of_sample(street)` until Task 3 removes them.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace`
Expected: PASS. Then fix any test that read `Street.sample` or `EdgeDef.street` by reading `class` instead.

- [ ] **Step 5: Format, full test, commit**

```bash
just format && just test
git add -A src
git commit -m "refactor(street): a street has a class, not an index into the sample streets

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Remove adding arms, swapping an arm's street, and `Arm.street`

**Files:**
- Modify: `src/junction/model.rs` (`Arm.street` 136-138, `Arm::new`, `fresh_arm`, `profile` 251-256, `street_index` 315, `street_name` 319, `row_mm` 323, `add_arm` 902-960, `set_street` 1016-1035, `Refusal::LinkedNoAdd`, `Refusal::LinkedNoSwap`, tests at 1515-1551, 1802, 1982, 2000)
- Modify: `src/junction/inspector.rs` (street `<select>` near 770-790, `SAMPLES` import at 15)
- Modify: `src/junction/page.rs` (street chip palette near 237 if any remains), `src/junction/vm.rs`, `src/junction/watch.rs` (`add_arm`, `set_street` forwarders)
- Modify: `src/city/model.rs` (`generate`: `Arm::new(i as u32 + 1, self.edges[e].street, b, 0)`, `junction_fits`)
- Test: `src/junction/model.rs`, `src/junction/tests.rs`

**Interfaces:**
- Consumes: `Arm.section: Option<Street>` (set by the city), `Street.class`.
- Produces: `Arm::new(uid: u32, bearing: i32, offset_mm: i32) -> Arm`; `Arm::street_name(&self) -> String` (the section's title, empty when no section); `Arm::class(&self) -> Option<StreetClass>`; no `add_arm`, no `set_street`.

In a city junction these operations already refuse (`Refusal::LinkedNoAdd`, `LinkedNoSwap`): a city junction is always `linked`. Removing them takes nothing away from the product.

- [ ] **Step 1: Write the failing test**

In `src/junction/model.rs` `mod tests`:

```rust
    #[test]
    fn an_arm_is_the_street_it_reads() {
        let city = crate::city::model::City::new();
        let node = city.view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let j = city.junction_editor(node, 0).unwrap();
        for a in &j.current().arms {
            let section = a.section.as_ref().expect("a city arm carries its street");
            assert_eq!(a.street_name(), section.title());
            assert_eq!(a.row_mm(), section.row_mm);
            assert_eq!(a.class(), Some(section.class));
        }
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test junction::model::tests::an_arm_is_the_street_it_reads`
Expected: FAIL to compile, "no method named `class` found for struct `Arm`".

- [ ] **Step 3: Implement**

In `src/junction/model.rs`: delete the `street` field from `Arm` and its argument from `Arm::new`; `Arm::street_name`, `row_mm` and the new `class` read `self.section`:

```rust
    pub fn street_name(&self) -> String {
        self.section.as_ref().map_or_else(String::new, Street::title)
    }

    pub fn row_mm(&self) -> i32 {
        self.section.as_ref().map_or(0, |s| s.row_mm)
    }

    pub fn class(&self) -> Option<StreetClass> {
        self.section.as_ref().map(|s| s.class)
    }
```

Delete `add_arm`, `set_street`, `Refusal::LinkedNoAdd`, `Refusal::LinkedNoSwap` and their texts in `junction/text.rs`, the `street_index` method, and the free function `profile(street, region)`. The cfg(test) `Junction::new` and the fresh-arm helper must now attach a section: have `fresh_arm` take a `Street` (`fn fresh_arm(&self, uid: u32, street: Street, bearing: i32, offset_mm: i32) -> Arm`) and set `arm.section = Some(street)`; the fixture arms in Task 4 supply it. In `src/junction/inspector.rs` remove the whole "Street" `<select>` section (it is shown only for unlinked junctions) and the `SAMPLES` import. In `junction/vm.rs` and `junction/watch.rs` remove the `add_arm` / `set_street` forwarders and their tests. In `src/city/model.rs`, `Arm::new(i as u32 + 1, b, 0)`. Delete the tests that exercise `add_arm`/`set_street` (`add_arm` appears at model.rs 1515, 1525, 1537, 1551, 1802, 1982, 2000).

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test --workspace` and `cargo build --target wasm32-unknown-unknown --workspace`
Expected: PASS (the wasm build catches the Leptos views).

- [ ] **Step 5: Format, full test, commit**

```bash
just format && just test
git add -A src
git commit -m "refactor(junction): an arm is the street it reads, and arms are not added or swapped

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Remove `Junction.sample` and move `JUNCTION_SAMPLES` under `cfg(test)`

**Files:**
- Create: `src/junction/model/fixtures.rs` (`#[cfg(test)]`; declared by `#[cfg(test)] mod fixtures;` in `junction/model.rs`)
- Modify: `src/junction/model.rs` (`JunctionSample` and `JUNCTION_SAMPLES` 440-536ish move to the fixtures file; `Junction::new`, `load_sample`, `sample()` at 546-600 move too; `Junction.sample` field; `from_city` 613)
- Modify: `src/junction/read_model.rs` (861, 1288-1291, 1562, 1754), `src/junction/vm.rs` (175, 241, 358), `src/junction/watch.rs` (94), `src/junction/page.rs` (244), `src/junction/mod.rs` (`junction_catalogue` 103-107)
- Test: existing tests, unchanged call shape

**Interfaces:**
- Produces: in `junction::model::fixtures` (cfg(test)): `pub const JUNCTION_SAMPLES: [JunctionSample; 4]`, `impl Junction { pub fn new(sample: usize) -> Junction; pub fn load_sample(&mut self, sample: usize); pub fn sample(&self) -> usize }`. Fixture arms are built with `section: Some(street_fixture(i))` from Task 5's street fixtures (until Task 5 they use `Street::sample`).
- `Junction` has no `sample` field in production; the fixture module keeps the index in a `#[cfg(test)] sample: usize` field.

- [ ] **Step 1: Write the failing test**

In `src/junction/model.rs` `mod tests`:

```rust
    #[test]
    fn a_junction_from_a_city_is_named_by_its_streets_and_has_no_template() {
        let city = crate::city::model::City::new();
        let node = city.view(0).nodes.iter().find(|n| n.junction).unwrap();
        let j = city.junction_editor(node.uid, 0).unwrap();
        assert_eq!(j.view().name, node.name);
    }
```

- [ ] **Step 2: Make the removal fail the build first**

This task deletes code, so its red step is the compiler. Remove the `sample` field and the `sample()` accessor from `Junction` first, then run `cargo build` and `cargo test --no-run`. Expected: errors at every use (`read_model.rs`, `vm.rs`, `watch.rs`, `page.rs`, `mod.rs`). Those errors are the to-do list for Step 3. The new test above guards the one behaviour that changes (the name of a city junction) and must keep passing.

- [ ] **Step 3: Implement**

Move `JunctionSample`, `SampleArm`, `sa`, `JUNCTION_SAMPLES`, `Junction::new`, `Junction::load_sample`, `Junction::sample` verbatim into `src/junction/model/fixtures.rs` under `#![cfg(test)]`-gated `mod fixtures;`. Add `pub use fixtures::JUNCTION_SAMPLES;` under `#[cfg(test)]` so tests that name it keep compiling. Remove the `sample` field from `Junction` and add `#[cfg(test)] sample: usize` instead. `read_model.rs` line 861 becomes `name: self.name().to_string()` (every production junction is linked and named). `JunctionVm::load_sample`, `Watch::load_sample` and `page.rs`'s sample picker are test-only now: gate them with `#[cfg(test)]` or delete them if no test uses them. In `junction/mod.rs` drop the `samples` and `streets` keys from `junction_catalogue()`.

- [ ] **Step 4: Run to verify it passes**

Run: `just test`
Expected: PASS with no new warnings (a leftover `pub` item used only by tests needs `#[cfg(test)]`).

- [ ] **Step 5: Format, commit**

```bash
just format
git add -A src
git commit -m "refactor(junction): the sample junctions are test fixtures, and a junction has no template

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Delete `SAMPLES` from the product

**Files:**
- Create: `src/street/model/fixtures.rs` (`#[cfg(test)]`, declared by `#[cfg(test)] mod fixtures;` in `street/model.rs`)
- Modify: `src/shared/catalogue.rs` (`Sample`, `SAMPLES`, `class_of_sample`; keep `KINDS`, `MATERIALS`, `CURBS`, `REGIONS`, `StreetClass`)
- Modify: `src/street/model.rs` (`Editor::new`, `load_sample`, `Street::sample`, the `SAMPLES` import, the tests at 1408-1420)
- Modify: `src/lib.rs` (the `streets` JSON at 40), `src/street/mod.rs` and `src/street/vm.rs` if they expose samples
- Modify: tests that import `SAMPLES` (`street/model.rs`, `junction/model/fixtures.rs`, `city/…`)

**Interfaces:**
- Produces: in `street::model::fixtures` (cfg(test)): `pub struct StreetSample { pub name, pub class, pub row_mm, pub freeway, pub segments }`, `pub const SAMPLES: [StreetSample; 4]` (moved verbatim, with `class: Local | Arterial | Local | Motorway`), `impl Editor { pub fn new(sample: usize) -> Editor; pub fn load_sample(&mut self, sample: usize) }`, `impl Street { pub fn sample(sample: usize, side: Side) -> Street }`.

- [ ] **Step 1: Write the failing test**

In `src/street/model.rs` `mod tests` replace the two template tests with a guard that the product has no template list:

```rust
    #[test]
    fn a_street_is_what_it_is_given_and_has_no_template_to_fall_back_on() {
        let s = Street::imported(StreetClass::Local, Side::Right, &[]);
        assert_eq!(s.title(), "");
        assert!(!s.class.is_freeway());
        assert!(Street::imported(StreetClass::Motorway, Side::Right, &[]).class.is_freeway());
    }
```

- [ ] **Step 2: Make the removal fail the build first**

This task deletes code, so its red step is the compiler. Delete `Sample` and `SAMPLES` from `shared/catalogue.rs` first, then run `cargo build` (product code only, without `--tests`). Expected: it succeeds, because Tasks 2 to 4 removed every product use. If it fails, the errors name a product use an earlier task missed: fix it here. Then `cargo test --no-run` fails at the test uses, which Step 3 moves into the fixtures module. The guard test above must keep passing.

- [ ] **Step 3: Implement**

Move `Sample`/`SAMPLES` out of `shared/catalogue.rs` into `street/model/fixtures.rs`, with `Editor::new`, `load_sample` and `Street::sample` (they need private `Editor` fields, which is why the file is a child module). Delete `class_of_sample`. Re-export for the other test modules with `#[cfg(test)] pub use fixtures::SAMPLES;` in `street/model.rs`, and point `junction/model/fixtures.rs` at `crate::street::model::SAMPLES`. In `src/lib.rs` remove the `streets` list (line 40) and its `SAMPLES` import. Update `shared/catalogue.rs`'s module doc ("Placeholder segment catalogue, outcome rates and sample streets") to drop "sample streets".

- [ ] **Step 4: Run to verify it passes**

Run: `just test` then `just e2e`
Expected: both PASS. If an e2e spec reads `streets` or `samples` from the exported JSON, remove that part of the spec.

- [ ] **Step 5: Format, commit**

```bash
just format
git add -A src e2e web
git commit -m "refactor(catalogue): the sample streets are test fixtures and no longer ship

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 6: ERD and docs match the code

**Files:**
- Modify: `docs/erd.mmd`, `docs/erd.svg`, `docs/erd.png`
- Modify: `CLAUDE.md` (the `shared/` and `city/` paragraphs that mention samples, the sample city "as `Layout::sample()`")

- [ ] **Step 1: Edit the ERD**

In `docs/erd.mmd` replace `usize sample FK` in `STREET` with `StreetClass class "motorway, arterial, collector or local"`, delete `usize sample FK` from `JUNCTION`, and change the `ARM }o--o| STREET : "edge (uid; 0 = sample)"` label to `"street (uid)"`.

- [ ] **Step 2: Regenerate and check**

```bash
mmdc -i docs/erd.mmd -o docs/erd.svg && mmdc -i docs/erd.mmd -o docs/erd.png -s 2
grep -c sample docs/erd.mmd docs/erd.svg
```
Expected: `0` for both files.

- [ ] **Step 3: Update `CLAUDE.md`**

Remove the sentences that say the sample city is `Layout::sample()` and that the catalogue holds sample streets; say that a street has a `StreetClass` and that test fixtures live in `street::model::fixtures`, `junction::model::fixtures` and `city::testing`.

- [ ] **Step 4: Verify**

Run: `just format-check && just test && just e2e`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add docs CLAUDE.md
git commit -m "docs(erd): a street has a class, and no junction or street is a sample

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```
