# Junction editor layout (CLM-32, step 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The junction page takes the street page's layout: details on the right, the notes as the shell's tray under the drawing, the title, scenarios and tools fixed at the top.

**Architecture:** The junction page reuses the street page's markup structure, its layout CSS (moved from `.street-page` to a shared `.tray-page` class) and the shell's `Tray` component, filled by a new `junction::view::notes_tray::JunctionNotes`. The drawing sizes itself to the room the column leaves it, measured as the street drawing does; the compare page keeps its window-based sizing.

**Tech Stack:** Rust (Leptos 0.8, wasm-bindgen, web-sys), CSS, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-22-junction-editor-layout-design.md` (section "Step 1: the layout").

Worktree: `/Users/alex/src/cityloom/worktrees/clm-32`, branch `alex/clm-32-bring-the-street-editors-overhaul-to-the`. Never share a `target/` with another checkout.

## Global Constraints

- Test-driven: the failing test first; one commit per task once its tests pass; `just format` before each commit.
- `RUSTFLAGS=-D warnings`: no new compiler warning.
- No new message is expected. Any new string goes in both `junction/i18n/{en,fr}.ftl`, OQLF rules (`docs/glossary-fr-ca.md`).
- Every element id a spec, the shell or `mount_page` uses stays: `#details-panel`, `#inspector`, `#inspector-toggle`, `#notes`, `#notes-toggle`, `t-space`/`p-space`, `t-checks`/`p-checks`, `t-changes`/`p-changes`, `#fit`, `#title-block`, `#scenario-slot`, `#history`, `#scroll`, `#plan-key`, `#across`, `#turns`, `#conflicts`, `#conflict-note`, `#checks`, `#checks-fold-slot`, `#goal-slot`, `#atlas-source`, `#revs`.
- Read-only (#222) unchanged: edits locked, History not drawn, turns' buttons disabled.
- WCAG 2.2 AA: focus never hidden under the sticky bar or the tray handle; `[` and `]` keep `aria-keyshortcuts`.
- The compare page (`junction::view::iso::iso_pair`) does not change.
- Version: `0.111.0` → `0.112.0` in `crates/cityloom_editor/Cargo.toml`, then `cargo update -p cityloom_editor`.

## Review Focus

1. **A 1,280 × 800 window with the tray open**: the drawing shrinks to fit; the tray handle and the head stay in view; nothing scrolls the page sideways. Pinned in Task 3 (the tray tests run at 1,280 × 800 as well as 1,440 × 900).
2. **A person whose remembered choice was notes open** (`cityloom-notes=open` from the old sidebar): the tray opens at the start, not a broken sidebar. Pinned in Task 3 (open after reload test).
3. **The stuck page** (a junction that cannot be drawn): no empty tray, no dangling `]` toggle, the message in view, both languages. Pinned by `language.spec.ts`'s existing test, plus an assertion added in Task 3 that `#notes-toggle` has count 0 there.
4. **A read-only junction** (`&view=1`): the scenarios in the head keep their read-only behaviour; no History in the head. Pinned in Task 3 (one read-only test on the new layout).
5. **Switching language with the tray open**: the tray's tab names and headings follow (they are now `tr` in Rust, not `data-i18n`). Pinned in Task 1 (French render test) and by `language.spec.ts`.

---

### Task 1: The junction's notes tray component

**Files:**
- Create: `crates/cityloom_editor/src/junction/view/notes_tray.rs`
- Modify: `crates/cityloom_editor/src/junction/view/mod.rs` (add `pub mod notes_tray;`)
- Modify: `crates/cityloom_editor/src/i18n_guard.rs` (`MIGRATED`; `NOT_MESSAGE_IDS` for the new kebab-case ids the guard names; widen the existing `t-space`… reasons to say "the street or junction page's notes")
- Test: `crates/cityloom_editor/src/junction/view/tests.rs`

**Interfaces:**
- Consumes: `crate::shell::tray::{Tray, TrayPage, TrayTab}`; `JunctionVm::i18n()`.
- Produces: `#[component] pub fn JunctionNotes(vm: Rc<JunctionVm>) -> impl IntoView`, in `junction::view::notes_tray`. Renders `<aside class="notes tray" id="notes">` with status slot `fit-slot`, three tabs in this order and the body's children after the panels:
  - `TrayTab { id: "t-space", panel: "p-space", label: "jn-page-tab-streets", badge: None }`: panel = the three sections now in `intersection.html`'s `#p-space` (headings `h-across`/`jn-page-across`, `h-turns`/`jn-page-turns` with the `jn-page-turns-hint` paragraph, `h-conf`/`jn-page-conflicts`), with the empty `#across`, `#turns`, `#conflicts` tables and `#conflict-note`.
  - `TrayTab { id: "t-checks", panel: "p-checks", label: "ui-tab-checks", badge: None }`: panel = today's `.checks-split` (`h-checks`/`jn-page-does-it-work`, `#checks`, `#checks-fold-slot`; side: `h-goal`/`goal-heading`, `#goal-slot`; `h-atlas`/`src-atlas-junction-label`, `#atlas-source`).
  - `TrayTab { id: "t-changes", panel: "p-changes", label: "ui-tab-changes", badge: None }`: panel = `h-rev`/`jn-page-your-changes`, `<ol class="rev-cards" id="revs">`.
  - Children: `<p class="fine">` with `jn-page-fine-print`, then `<div class="title-block" id="title-block" aria-label=jn-page-about>`.
  Same shape as `street/view/notes_tray.rs` (a function per tab returning `TrayPage`; a `Words` stored value; `tr`).

- [ ] **Step 1: Write the failing tests** in `junction/view/tests.rs`:

```rust
#[test]
fn the_junction_notes_are_the_shells_tray_with_its_three_tabs_in_order() {
    let s = shared(0);
    let h = html(|| view! { <notes_tray::JunctionNotes vm=s.clone()/> }.into_any());
    assert!(h.contains(r#"class="notes tray""#) && h.contains(r#"id="notes""#));
    let at = |id: &str| h.find(&format!(r#"id="{id}""#)).unwrap_or_else(|| panic!("no #{id}"));
    assert!(at("t-space") < at("t-checks") && at("t-checks") < at("t-changes"));
    for (tab, panel) in [("t-space", "p-space"), ("t-checks", "p-checks"), ("t-changes", "p-changes")] {
        assert!(h.contains(&format!(r#"aria-controls="{panel}""#)) && h.contains(&format!(r#"aria-labelledby="{tab}""#)));
    }
    assert!(h.contains(">Streets<") && h.contains(">Checks<") && h.contains(">Changes<"));
    assert_eq!(count(&h, "hidden"), 2, "only the first panel starts shown");
}

#[test]
fn every_slot_mount_page_fills_in_the_notes_is_inside_the_tray() {
    let s = shared(0);
    let h = html(|| view! { <notes_tray::JunctionNotes vm=s.clone()/> }.into_any());
    for id in ["fit-slot", "across", "turns", "conflicts", "conflict-note", "checks", "checks-fold-slot", "goal-slot", "atlas-source", "revs", "title-block"] {
        assert!(h.contains(&format!(r#"id="{id}""#)), "no #{id}");
    }
    assert!(h.find(r#"id="title-block""#) > h.find(r#"id="p-changes""#), "the title block follows the panels");
}

#[test]
fn the_junction_notes_speak_french() {
    // build `shared(0)` with its locale set to fr-CA as the other French tests in this file do
    // assert: ">Rues<" (jn-page-tab-streets in fr.ftl), the fr text of jn-page-turns and of jn-page-fine-print, and no ">Streets<"
}
```

The French test's exact strings are the fr values of `jn-page-tab-streets`, `jn-page-turns` and `jn-page-fine-print` in `junction/i18n/fr.ftl` and `ui-tab-checks` in `shell/i18n/fr.ftl`; read them, do not invent them.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p cityloom_editor junction::view::tests::the_junction_notes -- --nocapture` and `cargo test -p cityloom_editor every_slot_mount_page_fills`
Expected: compile error, `notes_tray` not found.

- [ ] **Step 3: Implement `JunctionNotes`** in `junction/view/notes_tray.rs` and register the module. Add the file to `MIGRATED` in `i18n_guard.rs`; add each new kebab-case literal the guard reports to `NOT_MESSAGE_IDS` with its reason ("the id of … in the junction page's notes (`junction::view::notes_tray`)").

- [ ] **Step 4: Run the tests and the guard**

Run: `cargo test -p cityloom_editor junction::view::tests && cargo test -p cityloom_editor i18n_guard`
Expected: PASS.

- [ ] **Step 5: Commit** — `Give the junction page's notes the shell's tray (CLM-32)` with the attribution lines.

---

### Task 2: The drawing's height from the room the column leaves

**Files:**
- Modify: `crates/cityloom_editor/src/junction/view/frame.rs`
- Modify: `crates/cityloom_editor/src/junction/view/drawing.rs` (`viewport_of`, `watch_size`)

**Interfaces:**
- Produces: `pub const AROUND: f64 = 555.0;` replacing the literal in `Frame::fit` (what the old page put around the drawing; the compare page keeps it), and `pub fn window_leaving(room: f64) -> f64` on `Frame`: the window height for which `Frame::fit` gives a drawing `room` high, i.e. `room + AROUND`, and `f64::INFINITY` stays `INFINITY` (no column: the clamp to 760 applies).
- `drawing.rs` keeps the `viewport: RwSignal<(f64, f64)>` and `iso_svg(…, width, window, …)` call unchanged; only what is put in the second number changes: on the junction page it is `Frame::window_leaving(room)`, where `room` is measured as `street/view/mod.rs::watch_size` measures it: `column = wrap.closest(".stage-scroll")`, `card = wrap.closest(".drawing")`; if the column's computed `overflow-y` is `auto` or `scroll`, `room = column.client_height - (card.offset_height - wrap.offset_height) - 4.0`, else `f64::INFINITY`. Observe the scroller, the wrap and the column (it changes height as the tray opens and closes). Keep the window resize listener.

- [ ] **Step 1: Write the failing test** in `frame.rs`'s tests:

```rust
#[test]
fn the_window_leaving_a_room_gives_a_drawing_that_high_within_the_bounds() {
    assert_eq!(Frame::fit(1000.0, Frame::window_leaving(500.0)).height, 500.0);
    assert_eq!(Frame::fit(1000.0, Frame::window_leaving(200.0)).height, 340.0);
    assert_eq!(Frame::fit(1000.0, Frame::window_leaving(f64::INFINITY)).height, 760.0);
    // the compare page's drawing, from the window itself, is as before
    assert_eq!(Frame::fit(1000.0, 1000.0).height, 445.0);
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p cityloom_editor junction::view::frame`
Expected: compile error, `window_leaving` not found.

- [ ] **Step 3: Implement `AROUND` and `Frame::window_leaving`**, then the measuring in `drawing.rs` (wasm-only code; its behaviour is pinned by Task 3's browser tests). Doc comments say the compare page still passes the window's height.

- [ ] **Step 4: Run the tests and the wasm build**

Run: `cargo test -p cityloom_editor junction::view && just test-wasm`
Expected: PASS, no warning.

- [ ] **Step 5: Commit** — `Size the junction's drawing to the room its column leaves (CLM-32)`.

---

### Task 3: The page's layout

**Files:**
- Modify: `web/intersection.html`, `web/street.html` (body class), `web/editor.css`
- Modify: `crates/cityloom_editor/src/junction/mod.rs` (`mount_page`, `Editor::mount_shell`, `mount_bare_shell`)
- Modify: `crates/cityloom_editor/src/junction/view/tests.rs` (markup tests), `crates/cityloom_editor/src/shell/vm.rs` (the `NotesAtStart` test's message)
- Test: `e2e/tests/editors.spec.ts`, `e2e/tests/junction.spec.ts`, `e2e/tests/language.spec.ts`, any spec the run shows failing on the old sidebar

**Interfaces:**
- Consumes: `JunctionNotes` (Task 1); the measured drawing (Task 2).
- Produces: `body class="tray-page"` on the junction page, `class="tray-page street-page"` on the street page.

Markup (`intersection.html`), following `street.html`:
- `<body class="tray-page">`; link `v/{{build}}/tray.css` after `editor.css`.
- Remove the `#inspector-toggle` left edge tab, the `#notes-toggle` right edge tab, the left `aside`, and the Notes `aside` with its tabs and panels and the fine print and title block (all now in the tray).
- `main.stage > .stage-main` holds: `.stage-head` (`.title-card.with-view > #street`, then `#scenario-slot.scenario-slot`, then `.tools > #history`), then `.stage-scroll` holding the drawing `section.drawing.plan-drawing` (`#h-drawing`, `#scroll`, `#plan-key`), then `<div id="notes-slot" style="display: contents"></div>`. The `.statusbar` goes (`#fit` is in the tray's handle).
- After `main`: `<button id="inspector-toggle" class="edge-tab right" …>` with the same attributes and messages as today's (`jn-page-details`, `jn-page-details-toggle-title`, `aria-keyshortcuts="["`), `aria-controls="details-panel"`; then `<aside class="panel details" id="details-panel" …>` with today's label and `h2`, holding `<div class="panel-body inspector" id="inspector"></div>`. Then the footer, `#live`, the scripts.

CSS (`editor.css`):
- Rename the layout rules' selector `.street-page` → `.tray-page` at today's lines 364, 368, 382, 398 and 3543, and rewrite the comment over the block (lines 357–362) to say the street and junction pages share it. The `.street-page #inspector …` rules (2382–2460) stay as they are.
- The junction's `.panel.left` rules in `editor.css` (the inspector's `border-top`, ~2015; the narrow layout's `order`, ~3565): give `#details-panel` the same top border as today (the junction's colour `--kc`) under `.tray-page:not(.street-page)` or move it to a selector both pages already want, and drop the `.panel.left` narrow rule if `.panel.details`' (3546) now covers it. `app.css`'s `.panel.left`/`.panel.right` stay (the map).

Rust:
- `mount_page`: mount `JunctionNotes` into `notes-slot` first, before every other mount (they fill slots inside it).
- `Editor::mount_shell`: `NotesAtStart::Closed`. Update the `NotesAtStart::Closed` doc and the test message in `shell/vm.rs:903` to say the street and junction pages.
- `mount_bare_shell`: remove `.plan-drawing` only (there is no `.statusbar`); keep `NotesAtStart::OpenWhenRoomy` (there is no notes toggle on that page, which `bind_sidebars` skips).

- [ ] **Step 1: Write the failing browser tests.** In `editors.spec.ts`, turn `test.describe("the street editor's panel and tray", …)` into a loop over both pages, not a copy:

```ts
for (const [name, open, title, tool] of [
  ["street", openStreet, "#street-name", "#add-btn"],
  ["junction", openJunction, "#street h1", "#undo"],
] as const) {
  test.describe(`the ${name} editor's panel and tray`, () => {
    // the six tests as today, using `open(page)`, `title` and `tool` where they name the street's elements;
    // the street-only scroll test (#plan, the strips) stays street-only (`test.skip(name !== "street")`)
  });
}
```

Read `junction/view/page.rs::Header` for the junction title's real selector before using `#street h1`. Add in the same file:
  - In the narrow describe, replace "the junction's start closed and open when asked, and stay open" by the street's tray version run on the junction (`#notes #fit` visible, `#p-space` hidden, toggle `aria-expanded=false`; click; `#p-space` visible; reload; still visible).
  - `test("the junction's whole drawing and the tray handle are in view at 1,280 × 800 with the tray open")`: open the junction, click `#t-space`, expect `#drawing` and `#notes #notes-toggle` and `#undo` in the viewport, and `document.documentElement.scrollWidth <= clientWidth`.
  - `test("a read-only junction keeps its scenarios in the head and has no history")`: open with `&view=1`; `#scenario-slot` inside `.stage-head`; `#undo` count 0.
  In `language.spec.ts`'s stuck-page test, add `await expect(page.locator("#notes-toggle")).toHaveCount(0);`.

- [ ] **Step 2: Run them to see them fail**

Run: `just e2e tests/editors.spec.ts tests/language.spec.ts`
Expected: the junction's panel-and-tray tests fail (`.panel.left` present, no `#notes #fit`).

- [ ] **Step 3: Change the markup, the CSS and `junction/mod.rs`** as above.

- [ ] **Step 4: Fix the markup tests** in `junction/view/tests.rs`: `intersection_html_leaves_no_text_without_its_message` finds `"<body"` (the tag now has a class) instead of `"<body>"`; lower the `keys.len() > 42` floor in `every_data_i18n_key_in_intersection_html_exists_in_both_languages` to the new count less a margin, with the comment "The notes' words are the tray's (`junction::view::notes_tray`), not the page's." as in `shell/tests.rs`.

Run: `cargo test -p cityloom_editor junction::view::tests shell::`
Expected: PASS.

- [ ] **Step 5: Run every junction and shell spec**

Run: `just e2e tests/editors.spec.ts tests/junction.spec.ts tests/junction-a11y.spec.ts tests/shell.spec.ts tests/language.spec.ts tests/a11y.spec.ts tests/edits.spec.ts tests/network.spec.ts tests/restrictions.spec.ts`
Expected: PASS. A spec that fails because a notes element is now hidden in the closed tray opens it by its tab first (`await page.locator("#t-space").click()`, or the tab the element is in) — never by dropping an assertion. A spec that read the old sidebars' geometry is updated to the new layout with the same intent. `shell.spec.ts:106`'s comment and expectation say both editors' trays start closed.

- [ ] **Step 6: Run the whole UI tier and the host tests**

Run: `just test && just e2e-ui`
Expected: PASS.

- [ ] **Step 7: Commit** — `Put the junction's details on the right and its notes in a tray under the drawing (CLM-32)`.

---

### Task 4: Docs, version, final checks

**Files:**
- Modify: `crates/cityloom_editor/src/junction/CLAUDE.md` (the layout; replace the Scenarios paragraph's "at the top of the details panel (left) … 555 px" reason with: in the head, as on the street page, the drawing sized to the column's room), `crates/cityloom_editor/src/street/CLAUDE.md` (its layout section: `.tray-page` shared with the junction page; `.street-page` keeps only the panel's label-column look), `crates/cityloom_editor/src/shell/` doc comments naming "the street page" as the only tray (`tray.rs` module doc, `NotesAtStart::Closed`), `website/docs/intro.md` and its fr-CA translation under `website/i18n/fr/` where they say where the junction's details or notes are.
- Modify: `crates/cityloom_editor/Cargo.toml` (`0.112.0`), `Cargo.lock`.

- [ ] **Step 1: Update the docs** listed above; `grep -rn "two sidebars\|panel.left\|left sidebar" crates/cityloom_editor/src web website/docs` finds nothing left about the junction page.
- [ ] **Step 2: Raise the version** to `0.112.0`; run `cargo update -p cityloom_editor`.
- [ ] **Step 3: Run everything**

Run: `just format && just test && just e2e && just docs-build`
Expected: PASS (`just e2e`'s integration tier needs `DATABASE_URL`; start one with `just db`).

- [ ] **Step 4: Commit** — `Junction layout: docs and version 0.112.0 (CLM-32)`.
