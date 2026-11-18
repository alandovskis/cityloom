# Transform Dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the street page's Transform menu with a modal dialog (groups, choices, a before/after preview with what changes, Apply/Cancel).

**Architecture:** The view-model gains the dialog's choices, a pure dialog state machine and a `preview(choice)` that runs the very edit Apply runs on a clone of the `Editor`. The 2.5D renderer gains a pair drawing at one shared scale. A new view component `street/view/transform.rs` (a native `<dialog>`) binds to them and replaces `page::Transform`.

**Tech Stack:** Rust, Leptos 0.8 (CSR, wasm32), Fluent `.ftl`, Playwright (e2e).

**Spec:** `docs/superpowers/specs/2026-11-18-transform-dialog-design.md`

Work in the worktree `/Users/alex/src/cityloom/transform-dialog` (branch `transform-dialog`). Paths below under `src/` are `crates/cityloom_editor/src/`.

## Global Constraints

- Nothing changes the street until Apply; the preview must come from running the same edit as Apply, on a clone of the real `Editor`, never an approximation.
- One undoable step per Apply, announced exactly as today (`StreetVm::apply` / `apply_measure` / `apply_template`); after Apply the focus goes to `#wrap`; after Cancel/Escape/close it returns to `#transform-btn`.
- Which choices are offered does not change: pedestrian kinds while `View.has_motor`; measures `m.can_apply && !m.present`, in the Notes table's order; all five templates.
- Every new string in `src/street/i18n/en.ftl` and `fr.ftl` with the same id (parity test), whole sentences, fr-CA per `docs/glossary-fr-ca.md` (`transport en commun`, never `transport collectif`). New ids and CSS classes that are not messages go in `src/i18n_guard.rs` `NOT_MESSAGE_IDS` with a reason; dead entries removed.
- No new colour pair without a `src/shared/contrast.rs` check; reuse existing tokens.
- View-model code never calls `street::view::*`.
- `RUSTFLAGS=-D warnings`: no new warnings. Run `just format` before each commit.
- Version: `crates/cityloom_editor/Cargo.toml` `0.92.1` → `0.93.0`, then `cargo update -p cityloom_editor`.
- Commit messages end with:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01XBf9xoyf5ttAHaYzjZi9CA
  ```

## Review Focus

1. A street shown at a time other than noon, with a piece that changes type through the day: the preview must equal what Apply gives (Task 2, `preview_matches_apply_at_the_time_shown`).
2. School template on a street with a zone frame: the preview shows the school zone, speed and raised table lines and equals Apply (Task 2, `preview_of_school_carries_its_report_and_matches_apply`).
3. Pointing at choices and then moving the mouse away: the preview returns to the chosen choice, or to the empty line when none is chosen (Task 3, `leaving_the_list_shows_the_chosen_again`).
4. Apply then Undo: one Undo gives back the street exactly as before (Task 2, `applying_a_choice_is_one_step_that_undo_reverses`).
5. A phone-width window (375 px): the dialog fits with no horizontal scroll and the columns stack (Task 7, `the dialog fits a phone-width window`).

---

### Task 1: `Editor` is `Clone`

**Files:**
- Modify: `src/street/model/mod.rs:365` (`pub struct Editor`), and any field type that is not yet `Clone` (add `#[derive(Clone)]` there).
- Test: `src/street/model/mod.rs` (its `tests` module)

**Interfaces:**
- Produces: `impl Clone for Editor` (derived).

- [ ] **Step 1: Write the failing test** `a_clone_is_edited_apart_from_the_editor_it_was_cloned_from`: `let e = Editor::new(0).with_frame(Some(ZoneFrame { length_mm: 33_000, from_end: ZoneEnd::Last }));` `let mut c = e.clone();` assert `c.make_pedestrian(kind_index("sidewalk").unwrap())`; assert `e.snapshot() != c.snapshot()`; assert `e.view().has_motor` and `!c.view().has_motor`; assert `serde_json::to_value(e.clone().view()).unwrap() == serde_json::to_value(e.view()).unwrap()`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor a_clone_is_edited_apart` — Expected: compile error, `Editor` has no `clone`.
- [ ] **Step 3: Add `#[derive(Clone)]` to `Editor`** and to each field type the compiler names as not `Clone`.
- [ ] **Step 4: Run** the same test — Expected: PASS.
- [ ] **Step 5: Commit** `Make the street Editor Clone, for a preview tried on a copy`.

---

### Task 2: The dialog's choices, preview and apply in the view-model

**Files:**
- Create: `src/street/vm/transform.rs` (declared `pub mod transform;` in `src/street/vm/mod.rs`)
- Modify: `src/street/vm/mod.rs` — split `fn findings(&self)` into `fn findings_for(&self, street: &Street) -> Vec<Finding>` (it keeps the empty-areas shortcut) used by `findings()`.
- Modify: `src/street/vm/watch.rs:408-420` — add `transform_choices`, `preview`, `apply_choice` forwarding to the VM (`transform_menu` stays until Task 5).
- Modify: `src/street/i18n/{en,fr}.ftl` — `transform-refused` (copy below).
- Test: `src/street/vm/transform.rs` (`#[cfg(test)] mod tests`, using `test_ports_with_time()` like the rigs in `vm/mod.rs:1504-1512` and `2866-2870`).

**Interfaces:**
- Consumes: `Editor: Clone` (Task 1); `impact::{Input, diff, Cause, Impact}`; `TemplateReport`; `TemplateRefusal::said(Template) -> Said`.
- Produces (all `pub` in `street::vm::transform`):
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
  pub enum Choice { Pedestrian(usize), Measure(&'static str), Template(Template) }
  impl Choice { pub fn key(self) -> String } // "ped-<kind id>", "measure-<code lowercased>", "template-<template id>"

  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Group { Pedestrian, Transit, Templates }

  #[derive(Clone, Debug, PartialEq)]
  pub struct TransformChoices {
      pub pedestrian: Vec<usize>,
      /// One entry per Atlas family with a choice, in order of first appearance: the code's letter, lowercased
      /// ('a'..='f'), and its codes in the Notes table's order.
      pub transit: Vec<(char, Vec<&'static str>)>,
      pub templates: Vec<TemplateView>,
  }
  impl TransformChoices {
      pub fn groups(&self) -> Vec<Group>;            // non-empty groups, in the order Pedestrian, Transit, Templates
      pub fn in_group(&self, g: Group) -> Vec<Choice>; // flattened, families in order
      pub fn is_empty(&self) -> bool;
  }

  #[derive(Clone, Debug)]
  pub struct Preview { pub before: Rc<View>, pub outcome: Outcome }
  #[derive(Clone, Debug)]
  pub enum Outcome {
      Changed { after: Rc<View>, impact: Impact, report: Option<TemplateReport> },
      Refused(Said),
  }
  impl Preview { pub fn can_apply(&self) -> bool } // Outcome::Changed

  impl StreetVm {
      pub fn transform_choices(&self) -> TransformChoices; // tracked (reads self.view())
      pub fn preview(&self, choice: Choice) -> Preview;    // untracked
      pub fn apply_choice(&self, choice: Choice) -> bool;
  }
  // SheetWatch: transform_choices(&self) -> TransformChoices, preview(&self, Choice) -> Preview, apply_choice(&self, Choice) -> bool
  ```
  `View` needs `Debug` for `Preview: Debug`; if it lacks it, drop `Debug` from `Preview`/`Outcome` rather than adding it to `View`.

- [ ] **Step 1: Write the failing tests** in `vm/transform.rs`:
  - `the_choices_group_the_measures_by_atlas_family`: on `rig(0)`, `transform_choices().transit` flattened equals `view.measures.iter().filter(|m| m.can_apply && !m.present).map(|m| m.code)` collected, and each entry's char equals the first letter of each of its codes lowercased, and no char appears twice.
  - `the_groups_leave_out_an_empty_one`: after `apply(|e| e.make_pedestrian(sidewalk))`, `groups()` does not contain `Group::Pedestrian` and `in_group(Group::Templates).len() == 5`.
  - `preview_matches_apply` — for every choice of `rig(0).vm.transform_choices()` whose preview `can_apply()`: on a fresh `rig(0)`, take `preview(c)`, then `apply_choice(c)`; assert `serde_json::to_value(&*after) == serde_json::to_value(&*vm.view_now())`; advance the scheduler past `IMPACT_DELAY_MS` and assert `vm.impact_now() == Some(impact)` with the impact's `headline` excepted (compare `uses`, `class`, the check lists and `verdict`).
  - `the_preview_leaves_the_street_alone`: snapshot and `view_now().revisions` are equal before and after `preview(c)` for every choice; nothing was announced (`said.take().is_empty()`).
  - `preview_matches_apply_at_the_time_shown`: a street with a piece that has a time-of-day variant (build it as the variant tests near `street/model/mod.rs:2560` do), `set_time` to a time inside the variant's window via `vm.edit`, then the same equality as `preview_matches_apply` for `Choice::Pedestrian(sidewalk)`.
  - `preview_of_school_carries_its_report_and_matches_apply`: `rig_of(&street(&PARKED-like pieces), Some(LENGTH), Locale::En)` as at `vm/mod.rs:2866` (copy that rig and its `street` helper), `preview(Choice::Template(Template::School))` is `Changed` with `report == Some(r)` where `r.applied` is non-empty; then `apply_choice` and the after-view equality holds.
  - `a_template_that_does_not_fit_previews_its_reason`: on a street narrower than `Template::Commercial.min_mm()`, `preview(Choice::Template(Commercial))` is `Refused(s)` with `s == TemplateRefusal::TooNarrow{..}.said(Commercial)` as `view.templates` gives it; `apply_choice` returns false.
  - `the_preview_draws_no_selected_piece`: select a piece with `vm.edit(|e| { e.select(Some(uid)); true })`; both `before.selected` and `after.selected` are `None`.
  - `applying_a_choice_is_one_step_that_undo_reverses`: snapshot `s0`; `apply_choice(Choice::Measure(first offered code))`; `vm.apply(|e| e.undo())` (or the VM's undo command); snapshot equals `s0`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor street::vm::transform` — Expected: compile errors (module missing).
- [ ] **Step 3: Implement** `transform.rs`:
  - `preview`: `let mut e = self.core.read(|e| e.clone()); e.select(None);` before view `Rc::new(e.view())`, before input `Input::of(&before, &self.findings_for(&e.snapshot()))`; run the edit (`make_pedestrian(k)` / `apply_measure(code)` / `apply_template(t)`); on success after view and `diff(Cause::Edit, &before_input, &after_input)`; on failure `Refused`: template → `refusal.said(t)`, measure → `Said::new("measure-refused")`, pedestrian → `Said::new("transform-refused")`.
  - `apply_choice`: `Pedestrian(k)` → `self.apply(|e| e.make_pedestrian(k))`; `Measure(c)` → `self.apply_measure(c)`; `Template(t)` → `self.apply_template(t)`.
  - Messages: en `transform-refused = This change does not suit this street.`; fr `transform-refused = Ce changement ne convient pas à cette rue.`
- [ ] **Step 4: Run** `cargo test -p cityloom_editor street::` — Expected: PASS.
- [ ] **Step 5: Commit** `Preview a whole-street change on a copy of the street before applying it`.

---

### Task 3: The dialog's state machine

**Files:**
- Modify: `src/street/vm/transform.rs`
- Test: same file

**Interfaces:**
- Consumes: `Choice`, `Group`, `TransformChoices` (Task 2).
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum ListKey { Up, Down, Home, End }
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub struct DialogState { pub group: Group, pub chosen: Option<Choice>, pub pointed: Option<Choice> }
  impl DialogState {
      pub fn open(choices: &TransformChoices) -> Option<DialogState>; // first group, nothing chosen or pointed
      pub fn shown(&self) -> Option<Choice>;                          // pointed, else chosen
      pub fn point(&mut self, c: Option<Choice>);
      pub fn choose(&mut self, c: Choice);
      pub fn set_group(&mut self, g: Group);                          // clears chosen and pointed when g differs
      pub fn step_choice(&mut self, choices: &TransformChoices, k: ListKey); // within in_group(group); from none: Down/Home → first, Up/End → last; clamps at the ends; clears pointed
      pub fn step_group(&mut self, choices: &TransformChoices, k: ListKey);  // over groups(), same rules, then set_group
  }
  ```
- [ ] **Step 1: Write the failing tests:** `opening_selects_the_first_group_and_nothing_else`; `leaving_the_list_shows_the_chosen_again` (choose A, point B → shown B; point None → shown A; with nothing chosen point B then None → shown None); `arrows_move_the_choice_and_stop_at_the_ends` (Down from none → first; End → last; Down at last stays last; Home → first); `a_disabled_template_is_still_reached_by_the_arrows`; `switching_group_clears_the_choice`; `step_group_skips_empty_groups` (after Make pedestrian, Down from Transit goes to Templates).
- [ ] **Step 2: Run** `cargo test -p cityloom_editor street::vm::transform` — Expected: FAIL (missing items).
- [ ] **Step 3: Implement** the methods with the signatures above.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** `Add the Transform dialog's state: group, chosen and pointed choice, arrow keys`.

---

### Task 4: Before and After drawn at one scale

**Files:**
- Modify: `src/street/view/iso/projection.rs` (add `Projection::fit_sharing`), `src/street/view/iso/mod.rs` (thread a projection through `diagram`, add `iso_pair`)
- Test: `src/street/view/iso/projection.rs` tests, `src/street/view/iso/mod.rs` tests

**Interfaces:**
- Produces:
  ```rust
  impl Projection {
      /// `fit(width, shared_span_mm)`'s scale, horizon and height, centred on a street `span_mm` wide.
      pub fn fit_sharing(width: f64, span_mm: i32, shared_span_mm: i32) -> Projection;
  }
  /// Before and After drawn `width` wide at one scale: the scale that fits the wider of the two.
  pub fn iso_pair(before: &View, after: &View, i18n: &I18n, width: f64) -> (IsoSvg, IsoSvg);
  ```
- [ ] **Step 1: Write the failing tests:** `fit_sharing_with_its_own_span_is_fit` (`fit_sharing(1050.0, 12_000, 12_000) == fit(1050.0, 12_000)`; derive `PartialEq` on `Projection` if absent); `fit_sharing_takes_the_wider_scale_and_its_own_centre` (`fit_sharing(1050.0, 12_000, 20_000)`: `scale`, `horizon`, `height` equal `fit(1050.0, 20_000)`'s, `centre_mm == 6_000.0`); `a_pair_of_one_street_is_the_street_twice` (`iso_pair(v, v, ..) == (iso_svg(v, ..), iso_svg(v, ..))` on markup, width, height); `a_pair_draws_the_narrower_at_the_wider_scale` (sample street and the same street after a template that widens or narrows `total_mm`: both heights equal and equal `iso_svg(wider, ..).height`).
- [ ] **Step 2: Run** `cargo test -p cityloom_editor street::view::iso` — Expected: FAIL.
- [ ] **Step 3: Implement:** `diagram` and `iso_svg` take the projection from one place; `iso_svg` keeps `Projection::fit(width, span_mm(v))`; `iso_pair` uses `fit_sharing(width, span_mm(x), max(span_mm(before), span_mm(after)))` for each. `diagram_and_projection` and the report keep their behaviour (existing tests unchanged).
- [ ] **Step 4: Run** `cargo test -p cityloom_editor street::` — Expected: PASS, including the existing `street::view::drawing` and iso tests.
- [ ] **Step 5: Commit** `Draw a street before and after a change at one scale`.

---

### Task 5: The Transform dialog component

**Files:**
- Create: `src/street/view/transform.rs` (`pub mod transform;` in `src/street/view/mod.rs`)
- Modify: `src/street/vm/mod.rs` and `src/street/vm/watch.rs` — remove `TransformMenu` and `transform_menu`; move their two tests (`vm/mod.rs:1949`, `1985`) to `vm/transform.rs` against `transform_choices`, keeping their assertions.
- Modify: `src/street/view/page.rs` — delete `Transform`, `menu_group`, `menu_item`, `template_item`, `TRANSFORM_MENU_WIDTH` and their now-unused imports.
- Modify: `src/street/view/notes.rs:194` — extract the body of `LastChange` after the headline into `pub fn impact_details(i18n: &I18n, units: Units, impact: &Impact) -> AnyView` (the "no effect" line included); `LastChange` calls it.
- Modify: `src/street/mod.rs:105` — mount `view::transform::TransformDialog` in `transform-slot`.
- Modify: `crates/cityloom_editor/Cargo.toml` web-sys features: add `"HtmlDialogElement"`.
- Modify: `src/street/i18n/{en,fr}.ftl`, `src/i18n_guard.rs`, `src/street/view/tests.rs` (replace the Transform-menu tests at `tests.rs:1276` onwards), `web/editor.css` (replace the "the Transform menu" block at line ~2964).

**Interfaces:**
- Consumes: Task 2 (`transform_choices`, `preview`, `apply_choice`, `Choice::key`), Task 3 (`DialogState`, `ListKey`), Task 4 (`iso_pair`, `iso_svg`), `drawing::alt`, `impact_details`, `text::template_report`'s per-line wording (render `report.applied` then `report.skipped` with `say`).
- Produces: `#[component] pub fn TransformDialog(vm: Rc<StreetVm>) -> impl IntoView`. Markup contract (the e2e tests of Task 6 rely on it):
  - `button#transform-btn.btn` with `aria-haspopup="dialog"`, label `transform-button`; shown only while `!choices.is_empty()`.
  - `dialog#transform-dialog` with `aria-labelledby="transform-dialog-h"`; `h2#transform-dialog-h` (`transform-dialog-title`); `button#transform-close` with `aria-label` from `transform-close`.
  - Groups: `div[role=tablist][aria-orientation=vertical]` with one `button[role=tab]#transform-tab-{ped|transit|templates}` per group, `aria-selected`, `aria-controls="transform-choices"`, text = the group's name (`make-pedestrian-button`, `st-transit-heading`, `transform-templates-heading`) and a count (`.count`: the number of choices; templates: `transform-templates-fit` with `fit` and `all`).
  - Choices: `div#transform-choices[role=listbox][tabindex=0]` with `aria-labelledby` the selected tab, `aria-activedescendant` the chosen option; first the group's hint as `p.hint` outside the listbox (`make-pedestrian-hint`, `transform-transit-hint`, `transform-templates-hint`); for transit, one `div[role=group][aria-labelledby=transform-family-{x}-h]` per family with its heading `transform-family-{x}`. Each option: `div[role=option]#transform-opt-{Choice::key}` with `aria-selected`, classes `transform-item` and `ped-item` / `transit-item` (with `span.dirtag` code) / `template-item` (`data-template=<id>`, `class:template-disabled`, `aria-disabled="true"` and the reason `p.template-reason` under it when it does not fit).
  - Preview: `section#transform-preview[aria-live=polite]` labelled by its `h3` (the shown choice's name; measures `"{code} · {name}"` through `measure-arrange-label`'s name or a new message); with nothing shown `p.transform-empty` (`transform-preview-empty`); otherwise `transform-now` and `transform-after` figures (`div.transform-figure` with `role="img"` and `aria-label` = `drawing::alt`, `inner_html` from `iso_pair`; on `Refused` only the Now figure and `p.transform-refused` with the reason); `h4` `transform-what-changes` then `impact_details` and the template report lines as `ul.transform-report`.
  - Footer: `button#transform-cancel` (`transform-cancel`), `button#transform-apply.btn-primary` (`transform-apply`), `disabled` unless `chosen` is some and its preview `can_apply()`.
  - Behaviour: the button calls `showModal()` and sets `DialogState::open`; `mouseenter` on an option → `point(Some)`, `mouseleave` on the listbox → `point(None)`; click → `choose`; listbox keys Up/Down/Home/End → `step_choice`, Enter → Apply when enabled; tablist keys → `step_group`; Apply → `vm.apply_choice(chosen)`, close, focus `#wrap`; Cancel / close / the dialog's `cancel` event (Escape) → close, focus `#transform-btn`. Preview width: the preview column's width, from a `ResizeObserver` or a fixed `TRANSFORM_PREVIEW_WIDTH: f64 = 360.0` (builder's call).
- New messages (en / fr):
  ```
  transform-dialog-title = Transform this street        / Transformer cette rue
  transform-close = Close                                / Fermer
  transform-cancel = Cancel                              / Annuler
  transform-apply = Apply                                / Appliquer
  transform-preview-empty = Choose a change to see what it does to the street. / Choisissez un changement pour voir ce qu’il fait à la rue.
  transform-now = Now                                    / Maintenant
  transform-after = After                                / Après
  transform-what-changes = What changes                  / Ce qui change
  transform-templates-fit = { $fit } of { $all }         / { $fit } sur { $all }
  transform-family-a = Transit streets                   / Rues de transport en commun
  transform-family-b = Centre-running lanes              / Voies centrales
  transform-family-c = Edge-running lanes                / Voies latérales
  transform-family-d = Offset lanes                      / Voies décalées
  transform-family-e = Curb-adjacent lanes               / Voies en bordure de trottoir
  transform-family-f = Contraflow lanes                  / Voies à contresens
  ```
  Add the six family rows to `docs/glossary-fr-ca.md` under "Transit priority measures".
- [ ] **Step 1: Write the failing component tests** in `street/view/tests.rs` (render with `street_html`, helpers as the existing Transform tests use): `the_transform_button_opens_a_dialog` (`aria-haspopup="dialog"`, `dialog#transform-dialog` labelled by `transform-dialog-h`, contains `>Transform this street<`); `the_dialog_has_a_tab_per_group_with_its_count` (sample street 0: three tabs, transit count equals the offered measures' number, templates text `"4 of 5"`-shaped from `transform-templates-fit`); `the_transit_choices_are_grouped_by_family` (one `role="group"` per family of `transform_choices().transit` once the Transit tab is selected — render with a `DialogState` whose group is Transit if the component takes an initial state for tests, else test the family markup through a pure `fn choices_markup(...)`); `every_option_is_named_and_a_disabled_template_says_why` (each `role="option"` has an id `transform-opt-*`; an unfit template has `aria-disabled="true"` and its `template-reason`); `the_preview_is_empty_until_a_choice_is_shown`; `apply_is_disabled_until_a_choice_that_fits_is_chosen`.
- [ ] **Step 2: Run** `cargo test -p cityloom_editor street::view::tests` — Expected: FAIL.
- [ ] **Step 3: Implement** `TransformDialog` and its pure helpers in `transform.rs`, the messages, the CSS (dialog max-width `min(960px, 100vw - 32px)`, three columns by grid, stacking to one column under 720 px; reuse existing tokens; `::backdrop` with an existing overlay token), the guard entries (`transform-dialog`, `transform-choices`, `transform-preview`, `transform-tab-*`, `transform-opt-*` prefixes as the guard allows, `transform-figure`, `transform-report`, `transform-empty`, `transform-refused` class vs message — give the class another name, e.g. `transform-no`, to avoid clashing with the message id), remove guard entries that became dead (`ped-item`/`transit-item` stay, their reason reworded to "the Transform dialog").
- [ ] **Step 4: Run** `cargo test -p cityloom_editor` then `just test` — Expected: PASS (i18n parity, i18n guard, contrast, wasm32 build).
- [ ] **Step 5: Commit** `Replace the street page's Transform menu with a dialog that previews each change`.

---

### Task 6: Browser specs follow the dialog

**Files:**
- Modify: `e2e/tests/street.spec.ts` (~293, 410–460), `templates.spec.ts`, `impact.spec.ts:90`, `map.spec.ts:466`, `network.spec.ts:234`, `junction.spec.ts:365`, `street-iso.spec.ts` (its Transform uses).
- Modify: `e2e/tests/fixtures.ts` — add `export async function transform(page: Page, option: string): Promise<void>` that clicks `#transform-btn`, clicks `#transform-dialog ${option}` (switching to the owning tab first when the option is in another group: pass the tab id as an optional second argument `tab?: "ped" | "transit" | "templates"`), and clicks `#transform-apply`.

**Interfaces:**
- Consumes: the markup contract of Task 5.

- [ ] **Step 1: Rewrite** every "click `#transform-btn`, click a `#transform-menu` item" to `transform(page, ".ped-item", "ped")` etc.; template tests use `getByRole("tab", { name: /Street templates/ })` and `getByRole("option")`; the Escape test (street.spec.ts ~440) asserts `#transform-dialog` is hidden and `#transform-btn` focused; the focus-after-choice tests assert `#wrap` focused after Apply.
- [ ] **Step 2: Add** to `street.spec.ts`: `pointing at a choice previews it and nothing changes until Apply` (hover a `.transit-item`: `#transform-preview` shows `Now` and `After` figures; the page's section is unchanged; Cancel; still unchanged; open, click it, Apply: changed, one Undo gives it back); `an unfit template previews its reason and cannot be applied` (Apply disabled).
- [ ] **Step 3: Run** `just e2e tests/street.spec.ts tests/templates.spec.ts tests/impact.spec.ts tests/map.spec.ts tests/network.spec.ts tests/junction.spec.ts tests/street-iso.spec.ts` — Expected: all pass.
- [ ] **Step 4: Commit** `Drive the Transform dialog in the browser specs`.

---

### Task 7: Accessibility and narrow window in the browser; docs; version

**Files:**
- Modify: `e2e/tests/a11y.spec.ts` (the street page's axe check with the dialog open), `e2e/tests/street.spec.ts`
- Modify: `src/street/CLAUDE.md` ("Pedestrian streets, Transform menu, templates" section, lines ~184–190), `crates/cityloom_editor/Cargo.toml` (`0.93.0`), `Cargo.lock`

- [ ] **Step 1: Add** `the dialog fits a phone-width window` (viewport 375×740: open the dialog; `document.documentElement.scrollWidth <= 375` and the dialog's `scrollWidth <= clientWidth`; Apply visible after scrolling the dialog) and the axe check with the dialog open and a choice previewed, in light and dark (`page.emulateMedia({ colorScheme })`).
- [ ] **Step 2: Run** `just e2e tests/street.spec.ts tests/a11y.spec.ts` — Expected: pass; fix any finding in the CSS or markup.
- [ ] **Step 3: Rewrite** the `street/CLAUDE.md` section for the dialog: `street::vm::transform` (choices, families, `DialogState`, `preview` on an `Editor` clone, `apply_choice`), `street::view::transform::TransformDialog` and its ids, `iso_pair`/`fit_sharing`.
- [ ] **Step 4: Bump** the version to `0.93.0`, run `cargo update -p cityloom_editor`, `just format`, `just check` — Expected: all green.
- [ ] **Step 5: Commit** `Document the Transform dialog and raise the version to 0.93.0`.
