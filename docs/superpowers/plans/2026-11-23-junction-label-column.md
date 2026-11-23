# Junction details: a label column, as the street's — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The junction's six detail levels (junction, street, crossing, corner, lane, bus and bike symbols) use the street piece pane's label column, with each level's actions under its name.

**Architecture:** The `.street-page #inspector` rules of `web/editor.css` become rules of one class, `.insp-columns`, which the street's `#inspector` and a wrapper around each junction level (not the signal plan) carry. Lists span both columns through `.insp-wide`. A level's buttons that act on the whole level move into an `.insp-actions` row under `head(…)`.

**Tech Stack:** Rust, Leptos 0.8 (CSR), CSS, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-23-junction-label-column-design.md`

## Global Constraints

- Not the signal plan level: `Inspector`'s `plan` branch (`signal_overview`) stays unwrapped.
- No change to what a level offers, to its wording, or to the model. No new message (en and fr-CA unchanged).
- The street's piece pane looks as it does today: its existing specs pass unchanged.
- Section ids, `h3` ids and `aria-labelledby` stay as they are.
- Read-only: every action that edits stays inside `shared::locked::locked`; the lane's way back to its street (navigation) does not.
- Targets at least 24 px; nothing overflows the panel at 1,440 px or a 280 px window, in English and French.
- Version 0.113.0 → 0.114.0, `cargo update -p cityloom_editor`.
- `junction/CLAUDE.md` and `street/CLAUDE.md` name the shared class.
- Run with `PATH=/opt/homebrew/opt/node@22/bin:/Users/alex/src/cityloom/main/.tools/bin:$PATH`, `E2E_PORT=8147`, `DATABASE_URL=postgres://postgres:postgres@127.0.0.1:55432/cityloom`, `RUSTFLAGS="-D warnings"`.

## Review Focus

1. A phase card's pattern radio group (`.opts.phase-patterns[role="radiogroup"]`, inside the street level's signal section) turned into tiles by the shared tile rule: the tile rules match only `.insp-sec > .opts[role="radiogroup"]` (a direct child), so the phase cards keep their look. Pinned in Task 4's e2e (the phase pattern radio's border width stays 0).
2. Read-only, the actions row sits inside a `fieldset.read-only-controls` (`display: contents`): it must still lay out as one row and its buttons be disabled. Pinned in Task 3's component test.
3. The street level after its last-but-one street: Remove is disabled and its reason (`jn-insp-min-arms`) still shows, now under the actions row. Pinned in Task 3.
4. The crossing level after its crossing is removed: the row offers Mark, and no empty `i-h-cross` section is left. Pinned in Task 3.
5. Long French names in column 1 (« Rayon de bordure », « Distance du passage ») wrap inside 76 px, never overflow into the control. Pinned in Task 4's e2e (fr-CA, every label's right edge left of its control).

---

### Task 1: The label column becomes a class both pages carry

**Files:**
- Modify: `web/editor.css` (the `.street-page #inspector …` rules, ~2338–2416, and the comment above `.insp-actions`, ~2299)
- Modify: `web/street.html:133` (`<div class="inspector insp-columns" id="inspector">`)
- Test: `e2e/tests/street.spec.ts` (existing specs, unchanged)

**Interfaces:**
- Produces: CSS classes `.insp-columns` (the container), `.insp-wide` (a section spanning both columns). Every later task relies on them.

- [ ] **Step 1:** Rewrite each `.street-page #inspector X` selector as `.insp-columns X`, values unchanged. `.insp-columns` itself: `display: flex; flex-direction: column; gap: 0; min-width: 0` (the street's `#inspector` already is a flex column, so its look holds).
- [ ] **Step 2:** Scope the tile rules to direct children: `.insp-columns .insp-sec > .opts[role="radiogroup"]` and `.insp-columns .insp-sec > .opts[role="radiogroup"] .opt[role="radio"]` (and its `[aria-checked="true"]`, `.tick`, `.swatch`). The street's three groups (`street/view/inspector.rs:310,439,470`) are direct children, so they match as before.
- [ ] **Step 3:** Add `.insp-wide`: `.insp-columns .insp-sec.insp-wide > * { grid-column: 1 / -1 }` and its `.note-h` on its own row (`grid-row: auto; padding: 0 0 2px`).
- [ ] **Step 4:** `grep -c "street-page #inspector" web/editor.css` → `0`. Run `just e2e tests/street.spec.ts tests/street-a11y.spec.ts` (whichever street specs exist): all pass.
- [ ] **Step 5:** Commit "Make the street's label column a class the junction can share (CLM-51)".

### Task 2: Each junction level inside the label column, lists spanning both columns

**Files:**
- Modify: `crates/cityloom_editor/src/junction/view/inspector.rs` (`Inspector` ~3394, `section` 168, `arm_panel` lanes section, `junction_panel` streets section, `lane_panel` serves section, `signal_section` `Some(true)` section, `transit_section`)
- Modify: `web/editor.css` (junction-only rules after the `.insp-columns` block)
- Test: `crates/cityloom_editor/src/junction/view/tests.rs`

**Interfaces:**
- Consumes: `.insp-columns`, `.insp-wide` (Task 1).
- Produces: `fn wide_section(w: Watch, id: &'static str, heading: &'static str, body: impl IntoView) -> impl IntoView` (as `section`, with `class="insp-sec insp-wide"`).

- [ ] **Step 1: Write the failing tests** (in `tests.rs`, inspector part; bus and cycle set up as `the_bus_lane_and_cycle_track_have_their_own_panels` does):

```rust
#[test]
fn each_level_of_the_details_is_in_the_street_s_label_column_and_the_signal_plan_is_not() {
    // junction, Arm(n), Crossing(n), Lane(n, 0), Corner(n), Bus, Cycle: panel(&s) contains `class="insp-columns"` exactly once
    // s.show_plan(): panel(&s) does not contain "insp-columns"
}

#[test]
fn a_level_s_lists_span_both_columns() {
    // the section holding id="i-h-streets" (junction), "i-h-lanes" (Arm), "i-h-serves" (Lane), "i-h-transit" (Arm),
    // and "i-h-signal" on a signalised Arm with a phase, is `class="insp-sec insp-wide"`;
    // the section holding "i-h-control" and "i-h-bearing" is `class="insp-sec"` without insp-wide
}
```

Find a section's class by the text between the last `<section` before the heading id and the id.

- [ ] **Step 2:** `cargo test -p cityloom_editor junction::view::tests::each_level_of_the_details junction::view::tests::a_level_s_lists` → both FAIL (no `insp-columns`).
- [ ] **Step 3:** In `Inspector`, the non-plan branch renders `{path(…)}<div class="insp-columns">{panel_of(…)}</div>`. Use `wide_section` for the streets list; give `class="insp-sec insp-wide"` to the lanes, serves, transit and phase (`Some(true)`) sections.
- [ ] **Step 4:** Junction-only CSS (scoped `#inspector > .insp-columns`, which the street's `#inspector` cannot match): a paragraph directly in the column (`.jn-tip`, the corner's note, `#calls-dropped`) gets `padding: 10px 0 0`; `.btn.jn-go` directly in a section keeps `width: 100%` (it is a way into a level, as wide as its column); a fold's first section has no top rule (the summary's rule is above it).
- [ ] **Step 5:** Run both tests → PASS; `cargo test -p cityloom_editor junction::` → all pass.
- [ ] **Step 6:** Commit "Lay each junction level out in the street's label column (CLM-51)".

### Task 3: A level's actions under its name

**Files:**
- Modify: `crates/cityloom_editor/src/junction/view/inspector.rs` (`danger_button` 655, `bus_panel`, `cycle_panel`, `lane_panel`, `arm_panel`'s `remove`, `crossing_section`, `crossing_fields`)
- Test: `crates/cityloom_editor/src/junction/view/tests.rs`

**Interfaces:**
- Produces: `fn actions(w: Watch, edits: bool, buttons: impl IntoView + 'static) -> AnyView` — `<div class="insp-actions">{buttons}</div>`, inside `locked(w.read_only(), …)` when `edits`. `crossing_section(w, uid, on_level: bool)`: with `on_level` its Mark/Remove button goes to the actions row (the street level calls it with `false`, unchanged).

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn a_level_s_actions_sit_under_its_name_before_its_first_section() {
    // For (Arm(n), "Remove this street"), (Crossing(n), "Remove the crossing"), (Lane(n,0), whole-street text),
    // (Bus, "Remove the bus lane"), (Cycle, "Remove the cycle track"):
    //   h.find("insp-actions") is Some, < h.find("<section"), and the button text is between "insp-actions" and the first "<section".
    //   h.find("insp-head") < h.find("insp-actions").
    // Crossing level after j.set_crossing(n, false): the row holds "Mark a crossing" (jn-insp-crossing-mark's English),
    //   and h does not contain `id="i-h-cross"`.
    // A street level whose Remove is refused (remove arms down to MIN_ARMS): the button is `disabled` and the
    //   jn-insp-min-arms sentence follows the row, before the first "<section".
}

#[test]
fn read_only_a_level_s_actions_are_offered_disabled_but_the_way_back_to_the_street_is_not() {
    // read-only vm (as a_junction_shown_read_only_… builds it): Arm, Crossing, Bus, Cycle: the "insp-actions" div is
    // inside `<fieldset disabled class="read-only-controls"`; Lane(n,0): it is not.
}
```

Copy the exact English of each button from `junction/i18n/en.ftl` into the assertions.

- [ ] **Step 2:** Run both → FAIL.
- [ ] **Step 3:** Move the buttons:
  - bus and cycle: `danger_button` becomes the actions row (`edits: true`); the bus panel's note section stays below it.
  - lane: the whole-street button goes to the row (`edits: false`); its section goes.
  - street: the Remove button goes to the row (`edits: true`), and the min-arms sentence (`insp-range`) follows the row.
  - crossing level (`arm_panel(…, true)`): `crossing_section(w, uid, true)`. With a crossing, the row holds Remove and the `i-h-cross` section keeps its range hint. With none, the row holds Mark and there is no `i-h-cross` section. On a street people walk along, there is no row and the section is unchanged.
- [ ] **Step 4:** Run both → PASS; `cargo test -p cityloom_editor junction::` → all pass. Fix any existing test whose selector assumed the old place of a button; it must still check the same thing.
- [ ] **Step 5:** Run `just e2e tests/junction.spec.ts tests/junction-a11y.spec.ts tests/read-only.spec.ts`. Every spec must pass. A spec that found a button by its old section gets its locator changed and keeps its assertions.
- [ ] **Step 6:** Commit "Put a junction level's actions under its name, as the street's (CLM-51)".

### Task 4: Tiles, and a browser check that nothing overflows

**Files:**
- Modify: `web/editor.css` (only if the checks below fail)
- Test: `e2e/tests/junction-a11y.spec.ts` (new tests in its `describe`)

**Interfaces:**
- Consumes: Tasks 1–3's markup.

- [ ] **Step 1: Write the test.** For `lang` of `["en", "fr-CA"]` and `width` of `[1440, 280]`, use `addInitScript(() => localStorage.setItem("cityloom-lang", lang))` and `openJunction(page)`. Then visit four levels in turn: the junction, the first street (`#jn-go-…`), its crossing (`#i-cross-go`) and its first lane (`.lane-pick`). At each level, assert:
  - `#inspector .insp-columns` exists;
  - every descendant of `#inspector .insp-columns` with a box has its right edge ≤ `#inspector`'s right edge plus 0.5;
  - every `.insp-sec:not(.insp-wide) > .note-h` has its right edge ≤ the left edge of the next element sibling with a box;
  - `document.documentElement.scrollWidth <= innerWidth`;
  - every enabled button in the column is at least 24 px high.

  Then make the junction a roundabout (`#inspector select[aria-labelledby="i-h-control"]`), offer turbo, and check:
  - the `i-h-turbo` radios are tiles, three across: the first three share a `y` and the fourth is lower;
  - on a signalised street (`openSignalFolds` is not needed: the phase cards sit on the street level), the computed `border-top-width` of a `.phase-patterns .opt[role="radio"]` is `0px`.

  Name it "each level of the details is a label column that fits the panel, in English and French, at 1440 and 280 px".
- [ ] **Step 2:** Run `just e2e tests/junction-a11y.spec.ts -g "label column"`. Fix the CSS of whatever fails, in `#inspector > .insp-columns` rules, never the street's, until it passes. Kinds of fix to expect:
  - an `.opt` with no glyph keeping its 44 px glyph column;
  - a `select` wider than its column.

  If turbo is not offered on the fixture junction, assert the tiles through a component check instead and note it.
- [ ] **Step 3:** Run `just e2e tests/junction.spec.ts tests/junction-a11y.spec.ts tests/street.spec.ts`. All must pass.
- [ ] **Step 4:** Commit "Check the junction's label column fits, in both languages and at 280 px (CLM-51)".

### Task 5: Version and docs

**Files:**
- Modify: `crates/cityloom_editor/Cargo.toml` (0.114.0), `Cargo.lock`
- Modify: `crates/cityloom_editor/src/junction/CLAUDE.md`: the inspector's description. Each level except the signal plan sits in `.insp-columns`, lists are `.insp-wide`, and the actions row sits under the name. Update the read-only paragraph to say where the buttons now sit.
- Modify: `crates/cityloom_editor/src/street/CLAUDE.md:75`: replace `.street-page #inspector` with `.insp-columns`, and "The junction's inspector keeps its own look" with "the junction's levels share it (CLM-51)".

- [ ] **Step 1:** Bump the version and run `cargo update -p cityloom_editor`. Then `just format`.
- [ ] **Step 2:** Run `just format-check && just test && just e2e`. Each must exit 0.
- [ ] **Step 3:** Commit "Say the junction shares the street's label column; 0.114.0 (CLM-51)".
