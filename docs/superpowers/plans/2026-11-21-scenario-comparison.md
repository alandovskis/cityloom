# Scenario comparison Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A read-only page, `compare.html`, that shows two designs (baseline or scenarios) of one street or junction side by side, with a summary of what differs.

**Architecture:** The street and junction slices each own their half (`street/{model,vm,view}/compare.rs`, `junction/{model,vm,view}/compare.rs`); `shared/compare.rs` and `shared/compare_view.rs` hold the URL rules, the pick state, the summary type and the frame (pickers, columns, summary) every half draws through a `CompareHost` trait; `lib.rs::mount_compare` picks the half the URL names.

**Tech Stack:** Rust, Leptos 0.8 CSR, wasm-bindgen, Fluent (`.ftl`), Playwright.

**Spec:** `docs/superpowers/specs/2026-11-21-scenario-comparison-design.md`

## Global Constraints

- Work in the worktree `/Users/alex/src/cityloom/worktrees/clm-9-compare`, branch `alex/clm-9-compare-two-scenarios-of-a-place-part-of-62`. Never push main.
- The page writes nothing: no record, scenario, history step, sync mark, nor the "opened last" project. Resolve the project with `project::context::peek`, never `enter`/`open_page`.
- Dependency rule unchanged: the street half uses only street code and `shared`/`city::binding`/`city::store`/`standards`; the junction half only junction code, the street *model*, and the same.
- Layout guard: under `street/` and `junction/` only `mod.rs`, `i18n/`, `CLAUDE.md` at the root; new files go in `model/`, `vm/`, `view/`.
- Every string in both `.ftl` files, same id and variables; whole sentences with named placeables; plurals by selector; fr-CA by OQLF (no-break space before `:` and inside `« »`), terms from `docs/glossary-fr-ca.md`. Lengths through `Units`/`Arg::Length`, never `format!`.
- New files of translated slices go in `i18n_guard.rs` `MIGRATED`; kebab-case ids that are not messages in `NOT_MESSAGE_IDS` with a reason.
- Drawings of a kind in the two columns at one scale.
- WCAG 2.2 AA: labelled pickers, an accessible name for every drawing, the summary as headings and lists, a pick change said in the live region (`#live`); new colour pairs in `shared/contrast.rs`.
- `RUSTFLAGS=-D warnings`; `just format` before every commit; one commit per task, each ending with the two attribution lines:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01XBf9xqvTepDxFiQRRcjUaA`.
- Version: minor bump of `crates/cityloom_editor/Cargo.toml` and `cargo update -p cityloom_editor` (Task 11).

## Review Focus

1. A place with no scenarios: both pickers offer only the baseline, A = B = baseline, the summary says the designs are the same; no panic. (Task 5 and Task 8 tests.)
2. `a`/`b` naming a deleted scenario, `a=abc`, or `a=` empty: falls back to the baseline and says `scenario-missing` once. (Task 1 parse tests, Task 5 vm test.)
3. Both pickers on the same design: allowed, summary "the same", no "Also differs in". (Task 5 test.)
4. Two designs equal except for uids / order of curbside uses or stops: no line, no "Also differs". (Task 3 test.)
5. Read-only (`view=1`) project: page opens, links keep `view=1`, nothing is written. (Task 5 test comparing `MemoryStorage` before/after; Task 9 link test.)

---

### Task 1: URL rules, pick state and summary type (`shared/compare.rs`), Page port `replace_path`

**Files:**
- Create: `crates/cityloom_editor/src/shared/compare.rs`
- Modify: `crates/cityloom_editor/src/shared/mod.rs` (add `pub mod compare;`)
- Modify: `crates/cityloom_editor/src/shared/ports.rs` (trait `Page`: `fn replace_path(&self, path: &str) {}` default no-op; `FakePage` records it into its `path`; `BrowserPage` calls `history.replace_state_with_url`)
- Modify: `crates/cityloom_editor/src/i18n_guard.rs` (`MIGRATED` if `shared/` is listed there; else nothing)

**Interfaces:**
- Produces:
  - `pub enum Pick { A, B }` (Copy, Eq, Debug)
  - `pub fn parse_pick(search: &str, key: &str, ids: &[u32]) -> Picked` where `pub enum Picked { Absent, Baseline, Scenario(u32), Missing }`. `base` → `Baseline`; an id in `ids` → `Scenario`; key absent → `Absent`; anything else (unknown id, non-number, empty) → `Missing`.
  - `pub fn with_picks(path: &str, a: Option<u32>, b: Option<u32>) -> String`: replaces or appends `a=`/`b=` (`base` for None), keeps every other parameter and its order, keeps `#…`.
  - `pub struct DiffGroup { pub heading: &'static str, pub lines: Vec<Said> }`; `pub struct Summary { pub groups: Vec<DiffGroup>, pub also: Vec<Said> }` (Clone, Debug, PartialEq) with `pub fn is_same(&self) -> bool` (no group has a line and `also` is empty) and `pub fn headline(&self, a: &str, b: &str) -> Said` → `Said::new("cmp-same")` with `a`,`b` when same, else `Said::new("cmp-differs")` with `a`, `b`, `n` = total lines + `also.len()`.
  - `#[derive(Clone)] pub struct CompareState` over `ArcRwSignal`s: `items: Vec<ScenarioItem>` (from `shared::scenarios`), `a: Option<u32>`, `b: Option<u32>`, `refusal: Option<Said>`; getters `items()`, `pick(Pick) -> Option<u32>` (tracked), `pick_now(Pick)`, `refusal()`; setters `list(Vec<(u32, String)>)`, `set(Pick, Option<u32>)`, `refuse(Said)`, `settle()`; `pub fn name_of(&self, id: Option<u32>, baseline: &str) -> String` (untracked).
  - `pub trait CompareHost { fn i18n(&self) -> Rc<I18n>; fn compare_state(&self) -> &CompareState; fn choose(&self, side: Pick, id: Option<u32>); fn summary(&self) -> Summary; fn back_href(&self) -> String; fn back_key(&self) -> &'static str; fn place_name(&self) -> Said; }`
  - `pub fn initial_picks(search: &str, ids: &[u32]) -> (Option<u32>, Option<u32>, bool)`: A defaults to baseline, B to the first scenario (else baseline); a `Missing` pick falls back to the baseline and sets the bool (say `scenario-missing`).

- [ ] **Step 1: Write the failing tests** (in `shared/compare.rs` `mod tests`)

```rust
#[test] fn a_pick_is_base_an_id_of_the_place_or_missing() {
    assert_eq!(parse_pick("street=7&a=base", "a", &[1, 2]), Picked::Baseline);
    assert_eq!(parse_pick("street=7&a=2", "a", &[1, 2]), Picked::Scenario(2));
    assert_eq!(parse_pick("street=7", "a", &[1, 2]), Picked::Absent);
    for bad in ["a=9", "a=abc", "a="] { assert_eq!(parse_pick(bad, "a", &[1, 2]), Picked::Missing, "{bad}"); }
}
#[test] fn picks_are_written_into_the_address_keeping_the_rest() {
    assert_eq!(with_picks("/compare.html?street=7&project=p1&view=1", None, Some(2)), "/compare.html?street=7&project=p1&view=1&a=base&b=2");
    assert_eq!(with_picks("/compare.html?a=1&street=7&b=2#x", Some(2), None), "/compare.html?a=2&street=7&b=base#x");
}
#[test] fn the_first_scenario_is_b_by_default_and_a_missing_pick_is_the_baseline() {
    assert_eq!(initial_picks("street=7", &[3, 5]), (None, Some(3), false));
    assert_eq!(initial_picks("street=7", &[]), (None, None, false));
    assert_eq!(initial_picks("street=7&a=5&b=9", &[3, 5]), (Some(5), None, true));
}
#[test] fn a_summary_with_nothing_in_it_is_the_same_and_counts_its_lines_otherwise() {
    let same = Summary { groups: vec![DiffGroup { heading: "cmp-h-space", lines: vec![] }], also: vec![] };
    assert!(same.is_same());
    let two = Summary { groups: vec![DiffGroup { heading: "cmp-h-space", lines: vec![Said::new("x")] }], also: vec![Said::new("y")] };
    assert_eq!(two.headline("Baseline", "Wider").args_num("n"), Some(2)); // or compare against a built Said
}
```
(Use whatever accessor `Said` offers to read an argument, or compare `two.headline(..)` with a `Said` built the same way.)

- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider shared::compare` → FAIL (module missing).
- [ ] **Step 3: Implement** the items listed under Produces, plus `replace_path` on the three `Page` impls.
- [ ] **Step 4: Run** the same command → PASS; `cargo test -p cityloom_editor --features test-provider shared::ports` → PASS.
- [ ] **Step 5: Commit** "Compare: the address, the pick and the summary of a comparison (CLM-9)".

### Task 2: The frame (`shared/compare_view.rs`) and the frame's messages

**Files:**
- Create: `crates/cityloom_editor/src/shared/compare_view.rs` (+ `pub mod compare_view;`)
- Modify: `crates/cityloom_editor/src/city/i18n/en.ftl`, `fr.ftl` (frame messages beside `scenario-*`)
- Modify: `crates/cityloom_editor/src/i18n_guard.rs` (`MIGRATED`: `shared/compare_view.rs`), `shared/testing.rs` only if a helper is missing
- Test: component tests in `shared/compare_view.rs` `mod tests` (render to HTML with `shared::testing`)

**Interfaces:**
- Consumes: Task 1 (`CompareHost`, `CompareState`, `Pick`, `Summary`).
- Produces: `#[component] pub fn CompareFrame<H: CompareHost + 'static>(host: Rc<H>, column_a: ChildrenFn, column_b: ChildrenFn) -> impl IntoView` (or two `Fn() -> AnyView` closures, whichever Leptos 0.8 makes simpler), drawing:
  - `h1#compare-h` "Compare designs · {place}" (`cmp-title` with `place`), a back link `a#compare-back` (`back_key()` message, `back_href()`).
  - Two labelled selects `#compare-a`, `#compare-b` (labels `cmp-pick-a` "Design A", `cmp-pick-b` "Design B"; options: `scenario-baseline` first with value `base`, then each item by id); `on:change` → `host.choose(side, id)`.
  - `#compare-refusal` (`role="status"`-free paragraph; the live region carries the announcement) showing `refusal()`.
  - `div.compare-cols` with two `section.compare-col` (`aria-labelledby` `compare-a-h` / `compare-b-h`; each `h2` "A: {name}" / "B: {name}" = `cmp-col-a`/`cmp-col-b`) holding the columns.
  - `section#compare-summary` with `h2` `cmp-summary-h` "What differs in B"; when `is_same()` one `p` `cmp-same`; else per non-empty group an `h3` (group heading key) and `ul` of lines, then `p.compare-also` `cmp-also` "Also differs in: {list}" (list joined with the existing list-joining message used elsewhere, e.g. `jn-list-comma`'s pattern; if none fits add `cmp-also` with a `list` placeable built from `Said` items joined by the locale's comma).
- Messages (both languages): `cmp-title`, `cmp-pick-a`, `cmp-pick-b`, `cmp-col-a`, `cmp-col-b`, `cmp-summary-h`, `cmp-same`, `cmp-differs` (plural on `n`), `cmp-also`, `cmp-back-street` "Back to the street", `cmp-back-junction` "Back to the junction", `cmp-h-space`, `cmp-h-lanes`, `cmp-h-pieces`, `cmp-h-checks`, `cmp-h-control`, `cmp-h-arms`, `cmp-h-signal`, `cmp-only-passes` "Passes only in B: {check}", `cmp-only-fails` "Fails only in B: {check}", and one name per "also" category: `cmp-other-speed`, `-stretches`, `-curbside`, `-stops`, `-calming`, `-green`, `-ring`, `-raised`, `-bus`, `-corners`.

- [ ] **Step 1: Failing component tests** with a fake `CompareHost` (two items, a summary of one group with one line and `also` of one):
  - the HTML holds `id="compare-a"` and `id="compare-b"`, each with `<option value="base"` first and an option per item;
  - a `<label for="compare-a">Design A</label>`;
  - `What differs in B`, the group heading, the line, and `Also differs in:`;
  - with an empty summary: `cmp-same`'s English text and no `compare-also`;
  - French render: `Design A`'s fr text and the no-break space before `:` in `cmp-also`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider shared::compare_view` → FAIL.
- [ ] **Step 3: Implement** the component and the messages.
- [ ] **Step 4: Run** it and `cargo test -p cityloom_editor --features test-provider i18n` (parity, guard) → PASS.
- [ ] **Step 5: Commit** "Compare: the frame two designs are shown in (CLM-9)".

### Task 3: Street differences beyond the impact summary (`street/model/compare.rs`), and uses on both sides

**Files:**
- Create: `crates/cityloom_editor/src/street/model/compare.rs` (+ `pub mod compare;` in `street/model/mod.rs`)
- Modify (only if Step 1's impact test fails): `crates/cityloom_editor/src/street/vm/impact.rs` `diff` uses the union of both sides' uses (a use missing on one side is 0 mm there)

**Interfaces:**
- Produces (pure, no wording but `Said`):
  - `pub struct LaneCount { pub direction: usize, pub a: usize, pub b: usize }` — `direction` indexes `DIRECTIONS`; counts pieces of the base section whose kind requires a direction (a driving lane; the catalogue says which), only where `a != b`.
  - `pub enum PieceChange { Added { kind: usize, width_mm: i32 }, Removed { kind: usize, width_mm: i32 }, Kind { from: usize, to: usize }, Width { kind: usize, a_mm: i32, b_mm: i32 } }`, with `pub fn said(&self) -> Said` (`cmp-piece-added`, `-removed`, `-kind`, `-width`; kinds through `shared::catalogue::kind_key`, widths `Arg::Length`).
  - `pub fn lanes(a: &Street, b: &Street) -> Vec<LaneCount>`
  - `pub fn pieces(a: &Street, b: &Street) -> (bool, Vec<PieceChange>)` — base sections matched by `uid`, in B's order then A's removed; bool = the two designs' stretches (`zones` from/count) differ.
  - `pub enum Other { Speed, Stretches, Curbside, Stops, Calming, Green }` with `pub fn key(self) -> &'static str` (`cmp-other-*`).
  - `pub fn others(a: &Street, b: &Street) -> Vec<Other>` — equality per category after normalising: uids set to 0 and lists sorted by position (and kind) where order carries no meaning (curb, stops, calming, green); `Speed` compares `speed`; `Stretches` compares `zones`' `from_mm` and `segments` with uids cleared.
  - Messages in `street/i18n/{en,fr}.ftl`: `cmp-piece-added`, `cmp-piece-removed`, `cmp-piece-kind`, `cmp-piece-width`, `cmp-lanes` ("{direction}: {a} → {b}", direction via the existing direction words), `cmp-stretches-differ`.

- [ ] **Step 1: Failing tests**
  - `impact`: `a_use_b_lacks_is_listed_as_dropping_to_zero` — `diff(Cause::Edit, &Input{uses: vec![(Mode::Parking, 2_400)],..}, &Input{uses: vec![],..})` has a `UseChange { before_mm: 2_400, after_mm: 0, .. }`. (If it already passes because `outcomes.share` lists every mode, delete this test's fix step and keep the test.)
  - `compare`: built from `Street::sample()` (StreetExt) clones:
    - `identical_streets_differ_in_nothing`: `lanes`, `pieces().1`, `others` all empty, `pieces().0 == false`.
    - `a_widened_and_a_removed_piece_are_listed`: B widens segment 0 by 500 mm and removes the last segment → `[Width{..a_mm, b_mm: a_mm + 500}, Removed{..}]`.
    - `a_lane_turned_around_changes_the_counts_each_way`.
    - `curbside_uses_in_another_order_with_other_uids_are_the_same`: shuffle `curb` and renumber uids → `others` empty.
    - `a_moved_stop_and_a_new_tree_are_named`: `others == [Stops, Green]`.
    - `a_new_speed_is_named`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider street::model::compare street::vm::impact` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** "Street: what differs between two designs of a street (CLM-9)".

### Task 4: Two street plans at one scale

**Files:**
- Modify: `crates/cityloom_editor/src/street/view/plan/mod.rs` (+ `scale.rs` if the across-scale lives there)

**Interfaces:**
- Produces: `pub fn plan_pair(a: &PlanView, b: &PlanView, width_px: f64, i18n: &I18n, units: Units) -> (String, String)` — both along the street at the mm-per-px of the longer, and across at the scale that fits the widest run of either into `BANDS_H`. `plan_svg` keeps its output byte-for-byte (it becomes the one-plan case of a shared inner function taking both scales).

- [ ] **Step 1: Failing tests**: `a_pair_of_equal_plans_is_drawn_as_one_plan_is` (`plan_pair(p, p, 600.0, ..).0 == plan_svg(p, 600.0, ..)`); `a_wider_design_draws_the_narrower_one_thinner` (B with segment 0 widened by 3 m: band height of A's segment 0 in A's markup equals A alone's height times `widest_a / widest_b`, read from the `height=` of its `data-uid` rect, or whatever attribute the bands carry).
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider street::view::plan` → FAIL. Existing plan tests must stay green.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** "Street: two plans drawn at one scale (CLM-9)".

### Task 5: The street comparison's view-model (`street/vm/compare.rs`)

**Files:**
- Create: `crates/cityloom_editor/src/street/vm/compare.rs` (+ `pub mod compare;` in `street/vm/mod.rs`)

**Interfaces:**
- Consumes: Tasks 1, 3; `impact::{Input, diff, Cause}`; `areas_holding` (private in `street/vm/mod.rs`: reachable from the child module); `CityBinding::{street_editor, scenarios}`.
- Produces: `pub struct StreetCompareVm` with `pub fn new(ports: Ports, i18n: Rc<I18n>, binding: CityBinding) -> Option<Rc<Self>>` (None when the baseline cannot be opened); `pub fn views(&self) -> (Rc<View>, Rc<View>)` (tracked); `pub fn units(&self) -> Units`; `impl CompareHost`. Summary groups in order: `cmp-h-space` (impact uses' `said()`, then the class line), `cmp-h-lanes`, `cmp-h-pieces` (with `cmp-stretches-differ` first when the bool is set), `cmp-h-checks` (`cmp-only-passes` per `now_passing`, `cmp-only-fails` per `now_failing`); `also` from `others`. `choose` sets the pick, reopens that side's design, rewrites the address (`ports.page.replace_path(&with_picks(&ports.page.path(), a, b))`) and says `summary().headline(name_a, name_b)` with `say_now`. `back_href` = `link_for(&format!("street.html?street={edge}"), &*ports.storage)`; `back_key` = `cmp-back-street`.

- [ ] **Step 1: Failing tests** (rig: `test_ports_with_page()` + `CityStore::sample`, a street edge, `binding.act(Action::Create(..))`, edits written with `binding.keep_street_in(s, Some(id))`):
  - `it_opens_on_the_baseline_and_the_first_scenario` (path `/compare.html?street=E`): picks `(None, Some(first))`.
  - `a_wider_bike_lane_in_b_is_in_space_pieces_and_checks` (scenario widens a bike lane): Space has the cycling line, Pieces a `Width` line; checks lines when a check flips (pick a width that flips `bike-lane-width` or whichever check id the sample has).
  - `a_place_with_no_scenarios_compares_the_baseline_with_itself`: picks `(None, None)`, `summary().is_same()`.
  - `the_same_design_twice_is_the_same`: `choose(Pick::A, Some(id))`, `choose(Pick::B, Some(id))` → same, `also` empty.
  - `a_deleted_scenario_in_the_address_falls_back_and_is_said`: path `…&b=99` → B None, `refusal()` is `scenario-missing`.
  - `choosing_rewrites_the_address_and_says_the_headline`: page path ends `a=base&b=2`; announcer took `cmp-differs`' English text.
  - `opening_and_choosing_write_nothing`: snapshot all `MemoryStorage` keys/values before `new` and after two `choose` calls → equal.
  - `read_only_back_link_keeps_view`: storage read-only (`view=1` in path) → `back_href()` contains `view=1`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider street::vm::compare` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** "Street: the comparison of two designs of a street (CLM-9)".

### Task 6: The street comparison's view (`street/view/compare.rs`) and `street::mount_compare_street`

**Files:**
- Create: `crates/cityloom_editor/src/street/view/compare.rs`
- Modify: `crates/cityloom_editor/src/street/view/mod.rs`, `street/mod.rs` (`pub fn mount_compare_street(ports: Ports, i18n: Rc<I18n>, edge: u32) -> bool`, mounting into `#compare-slot`; false when the street cannot be opened)
- Modify: `crates/cityloom_editor/src/street/view/tests.rs`, `i18n_guard.rs`

**Interfaces:**
- Consumes: Tasks 2, 4, 5; `street::view::iso::iso_pair`, `iso_label`, `plan_pair`.
- Produces: `#[component] pub fn StreetCompare(vm: Rc<StreetCompareVm>) -> impl IntoView` = `CompareFrame` whose columns are each: `figure.compare-plan` (the plan SVG, `role="img"`, `aria-label` = the plan's accessible text as the street page gives it) then `figure.compare-iso` (iso SVG, `aria-label` = `iso_label`). Width: a fixed drawing width of 560 px scaled by CSS (`width: 100%; height: auto`); no resize observer.

- [ ] **Step 1: Failing component test** in `street/view/tests.rs`: rendering `StreetCompare` over the Task 5 rig gives two `compare-plan` and two `compare-iso` figures, each with `role="img"` and a non-empty `aria-label`, and the summary heading.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider street::view::tests::compare` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS; `cargo build -p cityloom_editor --target wasm32-unknown-unknown` → OK.
- [ ] **Step 5: Commit** "Street: the comparison drawn side by side (CLM-9)".

### Task 7: Junction differences and a 2.5D pair at one scale

**Files:**
- Create: `crates/cityloom_editor/src/junction/model/compare.rs` (+ `pub mod compare;`)
- Modify: `crates/cityloom_editor/src/junction/view/iso/mod.rs` (`iso_pair`), `junction/i18n/{en,fr}.ftl`

**Interfaces:**
- Produces:
  - `pub fn iso_pair(a: &JView, b: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> (IsoSvg, IsoSvg)` — both through one frame scale (the smaller of the two that fit), like `figure::figures` does with `flatten_fit_pair`; selection outline and press targets left out (a pair is not pressed). `iso_svg` unchanged.
  - `pub struct ArmChange { pub uid: u32, pub street: Said, pub entering: (usize, usize), pub leaving: (usize, usize), pub crossing: (bool, bool), pub bike_box: (bool, bool) }` with `pub fn lines(&self) -> Vec<Said>` (one per field that differs: `cmp-jn-entering`, `cmp-jn-leaving`, `cmp-jn-crossing-added`/`-removed`, `cmp-jn-bike-box-added`/`-removed`, each naming the street). Entering/leaving counted as the inspector counts an arm's lanes.
  - `pub fn control(a: &JView, b: &JView) -> Vec<Said>` (control word changed: `cmp-jn-control`; turbo layout changed: `cmp-jn-turbo`)
  - `pub fn arms(a: &JView, b: &JView) -> Vec<ArmChange>` (by `uid`; only arms with a difference)
  - `pub fn signal(a: &JView, b: &JView) -> Vec<Said>` (phase count `cmp-jn-phases`, cycle `cmp-jn-cycle`; a signal added/removed is the control line's)
  - `pub fn checks(a: &JView, b: &JView) -> (Vec<Said>, Vec<Said>)` — (passes only in B, fails only in B) by `Check.id`, with the same rules as `impact::diff` (a check gone that was failing passes; a new failing check fails).
  - `pub enum JOther { Ring, Raised, Bus, Corners }` with `key()`; `pub fn others(a: &JView, b: &JView) -> Vec<JOther>` (ring present/size, `raised`, `bus`, corners' radii by arm uid).
- [ ] **Step 1: Failing tests** over `Junction::new(..)` / `CityStore::sample` junction clones: identical → all empty; control changed to signal → control line + signal lines; an arm's corner radius changed → `others == [Corners]` and nothing else; a crossing removed on one arm → one `ArmChange` with `crossing == (true, false)`; `iso_pair` of equal views equals in size and both drawn at the same frame scale (compare `frame` fields).
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider junction::model::compare junction::view::iso` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** "Junction: what differs between two designs, drawn at one scale (CLM-9)".

### Task 8: The junction comparison (`junction/vm/compare.rs`, `junction/view/compare.rs`, `junction::mount_compare_junction`)

**Files:**
- Create: `crates/cityloom_editor/src/junction/vm/compare.rs`, `junction/view/compare.rs`
- Modify: `junction/vm/mod.rs`, `junction/view/mod.rs`, `junction/mod.rs`, `junction/view/tests.rs`, `i18n_guard.rs`

**Interfaces:**
- Consumes: Tasks 1, 2, 7; `CityBinding::{junction_editor, scenarios}`.
- Produces: `JunctionCompareVm` mirroring `StreetCompareVm` (`new(ports, i18n, binding) -> Option<Rc<Self>>`, `views() -> (Rc<JView>, Rc<JView>)`, `impl CompareHost`; groups `cmp-h-control`, `cmp-h-arms`, `cmp-h-signal`, `cmp-h-checks`; back `intersection.html?junction={node}`, `cmp-back-junction`); `#[component] JunctionCompare(vm)` with one `figure.compare-iso` per column (`iso_pair`, `aria-label` = `iso_label`); `pub fn mount_compare_junction(ports: Ports, i18n: Rc<I18n>, node: u32) -> bool`.
- [ ] **Step 1: Failing tests**: vm — opens on baseline + first scenario; a scenario with a changed control has the control line; no scenarios → same; writes nothing (storage snapshot). View — two `compare-iso` figures with `role="img"` and `aria-label`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor --features test-provider junction::vm::compare junction::view::tests::compare` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** "Junction: the comparison of two designs of a junction (CLM-9)".

### Task 9: The Compare link on the editors

**Files:**
- Modify: `shared/scenarios.rs` (`ScenarioHost::compare_href(&self) -> Option<String>`, default None), `shared/scenarios_view.rs` (`a#scenario-compare.btn` "Compare" (`scenario-compare`) after the picker row, only while `items()` is non-empty and `compare_href` is Some), `street/vm/mod.rs` and `junction/vm/mod.rs` (implement it: `link_for(&format!("compare.html?street={edge}{b}"), &*storage)` with `&b={id}` when a scenario is shown, nothing on the baseline), `shared/project_id.rs` (`PROJECT_PAGES` gains `"compare.html"`, length 6), `city/i18n/{en,fr}.ftl` (`scenario-compare`)
- Test: `street/view/tests.rs`, `junction/view/tests.rs`, `shared/project_id.rs` tests

- [ ] **Step 1: Failing tests**: street page with no scenarios has no `scenario-compare`; after `create_scenario("Wider")` it has `href="compare.html?street=E&amp;b=1&amp;project=…"` (exact form `link_for` gives); read-only keeps `view=1`; same for the junction with `junction=N`; `link("compare.html?street=7", Some("p1")) == "compare.html?street=7&project=p1"`.
- [ ] **Step 2: Run** the three test modules → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** "Scenarios: a link to compare them (CLM-9)".

### Task 10: The page (`web/compare.html`, `compare.js`, `compare.css`, `lib.rs::mount_compare`)

**Files:**
- Create: `web/compare.html` (copy `web/timeline.html`'s head, boot banner, bar, footer, `#live`; title "CityLoom comparison"; skip link to `#compare-h` with `data-i18n="cmp-skip"`; `<main class="compare-page"><div id="compare-slot"></div></main>`; stylesheets `app.css`, `editor.css` (the iso classes), `compare.css`), `web/compare.js` (like `timeline.js`: `wasm.mount_compare()?.mount_shell()`), `web/compare.css`
- Modify: `crates/cityloom_editor/src/lib.rs` (`ComparePage { i18n, project }` with `mount_shell` like `TimelinePage`; `pub fn mount_compare() -> Option<ComparePage>`), messages `cmp-skip` in `city/i18n`, `shared/contrast.rs` if `compare.css` adds a colour pair, the server's static file list if it names pages (grep `timeline.html` in `crates/cityloom_server` and `build.sh`/`justfile` and mirror every hit)

**Interfaces:**
- `mount_compare`: `init_wasm_bindgen()`; `peek(browser_ports())`, else `navigator.replace(project::context::MISSING_HREF)` and None; `standards::store::activate`; `CityStore::current`; query `street=E` → `street::mount_compare_street`, `junction=N` → `junction::mount_compare_junction`; a place that cannot be opened → `navigator.replace(&link_for("map.html", storage))` and None (as `open_street_in`).
- CSS: `.compare-cols` grid, one column under 900 px and two from 900 px (`minmax(0, 1fr)`); figures `max-width: 100%`; no horizontal scroll at 280 px.

- [ ] **Step 1: Failing test** (`lib.rs` tests or `shell/tests.rs`, whichever tests the other pages' markup): `web/compare.html` has `aria-current`-free nav with Projects and About, `#compare-slot`, `#live`, the skip link, and every `data-i18n` key exists in both languages (mirror the existing page-markup test, e.g. the one that reads `web/timeline.html`).
- [ ] **Step 2: Run** that test → FAIL.
- [ ] **Step 3: Implement** the page, script, CSS and `mount_compare`.
- [ ] **Step 4: Run** `just test` → PASS (needs `DATABASE_URL`: `just db`).
- [ ] **Step 5: Commit** "Compare: the page (CLM-9)".

### Task 11: Browser spec, CI wiring, documentation, version

**Files:**
- Create: `e2e/tests/compare.spec.ts`
- Modify: `e2e/tiers.json` (`ui` gains `compare`), `scripts/ci/spec-map.json` (`cross` gains `compare`), root `CLAUDE.md` (pages list gains `web/compare.html`), `src/city/CLAUDE.md` (replace "Not yet: comparison (issue 62 part c)" with a line on the page), `src/shared/CLAUDE.md` (compare), `src/street/CLAUDE.md`, `src/junction/CLAUDE.md`, `website/docs/architecture.md` + its fr-CA twin if they list pages, `crates/cityloom_editor/Cargo.toml` (minor), `Cargo.lock`

- [ ] **Step 1: Write the spec** (follow `e2e/tests/scenarios.spec.ts`'s setup to open a street of the e2e fixtures and make a scenario): make "Wider bike lane", widen a bike lane, press `#scenario-compare`; expect `compare.html`, `#compare-a` = `base`, `#compare-b` = the scenario, two `.compare-iso` and two `.compare-plan`, the summary has a Space line; change `#compare-a` to the scenario → `cmp-same` text and the URL has `a=1&b=1`; `#live` has the headline; axe has no violations (use the helper `a11y.spec.ts` uses); at 280 px wide `document.documentElement.scrollWidth <= 280`; switch to Français → `Comparer` title text; the same for a junction with one scenario (control changed) — one `.compare-iso` per column.
- [ ] **Step 2: Run** `just e2e tests/compare.spec.ts` → FAIL before the wiring in tiers/spec-map, PASS after; `just test-scripts` → PASS.
- [ ] **Step 3: Documentation and version**: the CLAUDE.md lines above; bump minor; `cargo update -p cityloom_editor`.
- [ ] **Step 4: Run** `just format`, `just check` → PASS (with `DATABASE_URL`).
- [ ] **Step 5: Commit** "Compare: browser spec, documentation and version (CLM-9)".
