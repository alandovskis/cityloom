# MVVM Folders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every file of `street/`, `junction/` and `map/` into `model/`, `vm/` or `view/` folders, with no change in behaviour.

**Architecture:** One slice per task: `git mv` each file to the place the spec's table gives, rewrite the paths that
point at it, fix what the move breaks (`super::`, `pub(super)`, `include_str!`), update the i18n guard's lists and the
documentation that names the files. A small layout test, grown one slice per task, keeps a file from landing at a
slice's root again.

**Tech Stack:** Rust (edition of the workspace), Leptos 0.8, wasm32; `just` recipes.

**Spec:** `docs/superpowers/specs/2026-11-14-mvvm-folders-design.md` (read it first: its tables are the mapping, its
*Mechanics* are the rules, its *Known exceptions* are the references left pointing the wrong way on purpose).

## Global Constraints

- Moves only: no logic, wording, rename, split or merge. A diff hunk that is not a path, a visibility, a `mod`
  declaration, an `include_str!` path or a comment naming a path is out of scope.
- Every move is `git mv` (history follows the file).
- No re-export shims (`pub use …` in a slice's `mod.rs` to keep an old path).
- `pub(super)` that narrows after a move becomes `pub(in crate::<slice>)`, never `pub(crate)` or `pub`.
- `super::x` in a moved file that no longer resolves becomes `crate::<slice>::…`.
- CI runs with `RUSTFLAGS=-D warnings`: run the tests with it, so an unused import left by a move fails here too.
- `just test` builds the wasm32 code that `cargo test` skips; it is the gate, not `cargo test` alone.
- Work in `/Users/alex/src/cityloom/mvvm-folders` on branch `mvvm-folders`. `DATABASE_URL` from `just db`.
- Version: patch bump in `crates/cityloom_editor/Cargo.toml` and `cargo update -p cityloom_editor`.
- Paths below are relative to `crates/cityloom_editor/src/` unless they start with a root folder.

## Review Focus

1. **wasm32-only code** (`#[cfg(target_arch = "wasm32")]`, entry points in `mod.rs`) names a moved module: `cargo test`
   passes, the wasm build fails. Every task's gate is `just test`, which builds wasm32.
2. **A widened visibility** hides a design leak: a `pub(super)` turned `pub` compiles everywhere. The review checks
   `git diff origin/main -- '*.rs' | grep '^+.*pub(crate)\|^+pub '` finds no new widening.
3. **A stale path in a comment or doc** misleads the next reader. Task 4 ends with a grep that must find none.
4. **A file left at a slice root** or added there later. The layout test (Task 1, grown in Tasks 2 and 3) fails on it.
5. **A rename recorded as delete + add** loses history. Each task checks `git diff --cached -M --stat` shows renames.

---

### Task 0: Commit the spec

- [ ] **Step 1: Commit**

```bash
git add docs/superpowers/specs/2026-11-14-mvvm-folders-design.md docs/superpowers/plans/2026-11-14-mvvm-folders.md
git commit -m "Design and plan: model, vm and view folders in the large slices"
```

### Task 1: street/

**Files:**
- Create: `layout_guard.rs` (test-only), `street/vm/`, `street/view/`
- Move: per the spec's street table (`model.rs` → `model/mod.rs`, `vm.rs` → `vm/mod.rs`, `view.rs` → `view/mod.rs`, …)
- Modify: `lib.rs` (declare `layout_guard`), `street/mod.rs`, `street/model/mod.rs`, `street/vm/mod.rs`,
  `street/view/mod.rs`, every file in the crate that names a moved street module, `i18n_guard.rs`,
  `street/CLAUDE.md`, comments naming a street file (see the spec's *Docs* list)

**Interfaces:**
- Produces: `layout_guard::LAYERED: &[&str]` (slices checked), the module paths
  `crate::street::model::{measures, pieces}`, `crate::street::vm::{impact, text, keys, watch, row}`,
  `crate::street::view::{page, inspector, notes, panel_tabs, piece_list, speed_view, strip, calming_strip,
  calming_tools, curb_strip, curb_tools, green_strip, green_tools, stop_tools, zone_tools, drawing, iso, tests}`.

- [ ] **Step 1: Write the failing layout test** in `layout_guard.rs`, declared `#[cfg(test)] mod layout_guard;` in `lib.rs`

```rust
//! The large slices keep their files in `model/`, `vm/` and `view/`: at a slice's root only `mod.rs`, `i18n/` and
//! `CLAUDE.md`.

const LAYERED: &[&str] = &["street"];
const ROOT_DIRS: &[&str] = &["model", "vm", "view", "i18n"];
const ROOT_FILES: &[&str] = &["mod.rs", "CLAUDE.md"];

#[test]
fn layered_slices_keep_their_files_in_model_vm_and_view() {
    for slice in LAYERED {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(slice);
        let mut stray = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let allowed = if entry.file_type().unwrap().is_dir() { ROOT_DIRS } else { ROOT_FILES };
            if !allowed.contains(&name.as_str()) {
                stray.push(name);
            }
        }
        stray.sort();
        assert!(stray.is_empty(), "{slice}/ holds {stray:?} at its root; each belongs in model/, vm/ or view/");
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p cityloom_editor layout_guard`
Expected: FAIL, `street/ holds ["calming_strip.rs", …, "zone_tools.rs"]`.

- [ ] **Step 3: Move the files** with `git mv`, per the spec's street table. `street/mod.rs` then declares only
  `pub mod model; pub mod vm; pub mod view;`; each folder's `mod.rs` declares its moved files with the visibility they
  had in `street/mod.rs` (`tests` stays `#[cfg(test)] mod tests;`, now in `view/mod.rs`). The declarations that were
  already in `model.rs` (its `model/*` children) stay as they are.

- [ ] **Step 4: Fix the paths and what the move broke**, by the Global Constraints: every
  `crate::street::<moved>` in the crate (other slices use `pieces`, `measures`, `drawing`, `vm`; `atlas_contract.rs`
  uses `measures`); `super::` in moved files; `pub(super)`; `include_str!` paths (`view/tests.rs` and `view/iso/*` gain
  one `../`; `vm/text.rs`'s `include_str!("model.rs")` becomes `"../model/mod.rs"`).

- [ ] **Step 5: Update `i18n_guard.rs`**: each street path in `MIGRATED` (the name and its `include_str!`) and in the
  reason strings of `NOT_MESSAGE_IDS` / `ALLOWED_LITERALS`.

- [ ] **Step 6: Run the tests**

Run: `RUSTFLAGS="-D warnings" just test`
Expected: PASS, including `layout_guard` and `i18n_guard`.

- [ ] **Step 7: Update the docs naming street files**: `street/CLAUDE.md`; the comments in `web/app.css`,
  `web/editor.css`, `shared/flat.rs`, `shared/iso/ground.rs`, `street/view/iso/projection.rs`, `street/view/notes.rs`,
  `street/vm/text.rs`; the `street/i18n/*.ftl` headers; `docs/glossary-fr-ca.md`. Then `just format`.

- [ ] **Step 8: Check the renames and commit**

Run: `git add -A && git diff --cached -M --stat | grep -c '=>'` — Expected: one line per moved file (≈30 plus
`iso/*`), and no `street/*.rs` listed as deleted.

```bash
git commit -m "Put the street slice's files in model, vm and view folders"
```

### Task 2: junction/

**Files:**
- Create: `junction/vm/`, `junction/view/` (with a new `view/mod.rs` that only declares modules)
- Move: per the spec's junction table
- Modify: `layout_guard.rs` (`LAYERED` gains `"junction"`), `junction/mod.rs`, the folders' `mod.rs`, every file
  naming a moved junction module (city uses `read_model`, `signal`, `turbo`; map uses `figure`, `read_model`),
  `i18n_guard.rs` (paths, and an entry for the new `junction/view/mod.rs`), `junction/CLAUDE.md`, comments naming a
  junction file

**Interfaces:**
- Consumes: `layout_guard::LAYERED` (Task 1).
- Produces: `crate::junction::model::{read_model, geometry, beyond, curbside, stops, signal, turbo, turbo_ring,
  turbo_spiral}`, `crate::junction::vm::{text, keys, watch}`, `crate::junction::view::{drawing, figure, phase_svg,
  iso_marks, frame, turns, inspector, notes, page, iso, tests}`.

- [ ] **Step 1: Add `"junction"` to `LAYERED`**

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p cityloom_editor layout_guard`
Expected: FAIL, `junction/ holds ["beyond.rs", …, "watch.rs"]`.

- [ ] **Step 3: Move the files** with `git mv`, per the spec's junction table; `junction/model.rs` → `model/mod.rs`
  keeps `model/fixtures.rs` under it.

- [ ] **Step 4: Fix the paths and what the move broke**, as Task 1 Step 4. `view/mod.rs` declares
  `#[cfg(test)] pub(in crate::junction) mod tests;` (model and vm tests reach its fixtures). In `view/tests.rs`,
  `include_str!("i18n/en.ftl")` becomes `"../i18n/en.ftl"` (same for `fr.ftl`); the `web/*` paths gain one `../`.

- [ ] **Step 5: Update `i18n_guard.rs`** (paths in `MIGRATED`, reasons naming `junction/iso/…`; add
  `src/junction/view/mod.rs` to the list its coverage test asks for).

- [ ] **Step 6: Run the tests**

Run: `RUSTFLAGS="-D warnings" just test`
Expected: PASS.

- [ ] **Step 7: Update the docs naming junction files** (`junction/CLAUDE.md`, the `junction/i18n/*.ftl` headers,
  the `web/editor.css` comments on `junction/iso/`), then `just format`.

- [ ] **Step 8: Check the renames and commit**, as Task 1 Step 8.

```bash
git commit -m "Put the junction slice's files in model, vm and view folders"
```

### Task 3: map/

**Files:**
- Create: `map/model/` (new `model/mod.rs`), `map/vm/`, `map/view/`
- Move: per the spec's map table
- Modify: `layout_guard.rs` (`LAYERED` gains `"map"`), `map/mod.rs`, the folders' `mod.rs`, every file naming a moved
  map module, `i18n_guard.rs` (paths in `MIGRATED` and `LEFT_OUT` — `src/map/style.rs` becomes
  `src/map/vm/style.rs` — and an entry for the new `map/model/mod.rs`), `map/CLAUDE.md`, comments naming a map file

**Interfaces:**
- Consumes: `layout_guard::LAYERED` (Task 1); `crate::street::view::drawing`, `crate::junction::view::figure` (Tasks 1, 2).
- Produces: `crate::map::model::{network, source_quality, corridor, editable}`, `crate::map::vm::{areas, selection,
  camera, report, report_review, overlay, style}`, `crate::map::view::{areas_view, network_view, selection_view,
  source_view, home, svg, tests}`.

- [ ] **Step 1: Add `"map"` to `LAYERED`**

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p cityloom_editor layout_guard`
Expected: FAIL, `map/ holds ["areas.rs", …, "svg.rs"]`.

- [ ] **Step 3: Move the files** with `git mv`, per the spec's map table.

- [ ] **Step 4: Fix the paths and what the move broke**, as Task 1 Step 4. `report` and `report_review`, private in
  `map/mod.rs` today, are declared `pub(in crate::map) mod …;` in `vm/mod.rs`.

- [ ] **Step 5: Update `i18n_guard.rs`** as listed under *Files*.

- [ ] **Step 6: Run the tests**

Run: `RUSTFLAGS="-D warnings" just test`
Expected: PASS.

- [ ] **Step 7: Update the docs naming map files** (`map/CLAUDE.md`, the `map/i18n/*.ftl` and
  `restriction/i18n/*.ftl` headers, the `web/app.css` comments on `src/map/style.rs`), then `just format`.

- [ ] **Step 8: Check the renames and commit**, as Task 1 Step 8.

```bash
git commit -m "Put the map slice's files in model, vm and view folders"
```

### Task 4: Root documentation, stale paths, version

**Files:**
- Modify: `CLAUDE.md` (root), `crates/cityloom_editor/Cargo.toml`, `Cargo.lock`

- [ ] **Step 1: Update the root `CLAUDE.md`**: the Architecture bullet listing each slice's files (`street/`,
  `junction/`, `map/` now hold `model/`, `vm/`, `view/`; the junction's extra files named under their folders); the
  SVG renderers' paths (`junction/view/iso/`, `street/view/iso/`, `map/view/svg.rs`); and one line in the dependency
  rule: *inside `street/`, `junction/` and `map/`, the layers are the folders `model/`, `vm/` and `view/`; another
  slice may use only `model/`, except the report's figures (`street::view::drawing`, `junction::view::figure`).*

- [ ] **Step 2: Grep for stale paths**

Run:
```bash
git grep -nE '(street|junction|map)/(calming_strip|calming_tools|curb_strip|curb_tools|drawing|green_strip|green_tools|impact|inspector|iso|keys|measures|notes|page|panel_tabs|piece_list|pieces|row|speed_view|stop_tools|strip|text|watch|zone_tools|beyond|curbside|figure|frame|geometry|iso_marks|phase_svg|read_model|signal|stops|turbo|turbo_ring|turbo_spiral|turns|areas|areas_view|camera|corridor|editable|home|network|network_view|overlay|report|report_review|selection|selection_view|source_quality|source_view|style|svg|tests)(\.rs|/)' -- ':!docs/superpowers' ':!target'
git grep -nE '(street|junction|map)::(calming_strip|calming_tools|curb_strip|curb_tools|drawing|green_strip|green_tools|impact|inspector|iso|keys|measures|notes|page|panel_tabs|piece_list|pieces|row|speed_view|stop_tools|strip|text|watch|zone_tools|beyond|curbside|figure|frame|geometry|iso_marks|phase_svg|read_model|signal|stops|turbo|turbo_ring|turbo_spiral|turns|areas|areas_view|camera|corridor|editable|home|network|network_view|overlay|report|report_review|selection|selection_view|source_quality|source_view|style|svg|tests)\b' -- ':!docs/superpowers'
```
Expected: no output (every hit is already under `model/`, `vm/` or `view/`; fix any that is not).

- [ ] **Step 3: Bump the version**: patch in `crates/cityloom_editor/Cargo.toml` (main's version + 0.0.1), then
  `cargo update -p cityloom_editor`; `scripts/check-version.sh` passes.

- [ ] **Step 4: Run everything**

Run: `just format-check && RUSTFLAGS="-D warnings" just test && just e2e`
Expected: all PASS.

- [ ] **Step 5: Check history and widening**

Run: `git log --follow --oneline -3 -- crates/cityloom_editor/src/junction/model/read_model.rs` — Expected: commits
from before this branch.
Run: `git diff origin/main -- '*.rs' | grep -E '^\+.*(pub\(crate\)|^\+\s*pub (fn|struct|enum|mod|const))'` and compare
each hit with the line it replaced — Expected: no item more visible than before.

- [ ] **Step 6: Commit**

```bash
git add CLAUDE.md crates/cityloom_editor/Cargo.toml Cargo.lock
git commit -m "Describe the model, vm and view folders; version <new>"
```
