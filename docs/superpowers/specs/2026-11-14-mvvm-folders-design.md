# MVVM folders in the large slices

## Intent

`street/` (≈30 files at the top, 72 in all), `junction/` (≈30) and `map/` (≈20) each hold a flat list of files where the
model, the view-model and the views sit side by side. Finding a file means scanning the whole list. Splitting each of
these slices into `model/`, `vm/` and `view/` folders makes a file's place follow from what it is.

This is a move only: no behaviour, wording, rule or test outcome changes.

## Goals

- `street/`, `junction/` and `map/` each have `model/`, `vm/` and `view/`; every `.rs` file of the slice lives in one of
  them, except the slice's `mod.rs`.
- Every file's history follows it (`git mv`).
- `just test`, `just format-check` and `just e2e` pass as before.

## Non-goals

- The other slices (`account`, `archive`, `city`, `place`, `project`, `restriction`, `shared`, `shell`, `standards`,
  `sync`) stay as they are.
- No fixing of the existing references that cross layers the "wrong" way (listed under *Known exceptions*). They keep
  working under their new paths and are named here so they can be dealt with later.
- No renaming of files, types or functions, no splitting or merging of files.

## Anti-goals

- Re-export shims (`pub use view::page;` in `mod.rs` and the like) that keep the old flat paths alive. They would bring
  back the flat list this change removes.
- Any change in what the code does, hidden among the moves.

## Layout rules

- **model/**: editing rules and pure computation over the domain, with no Leptos and no wording: what other slices may
  use under the dependency rule.
- **vm/**: view-models and what they own: presentation state, the key state machines, the `Said`/wording helpers
  (`text.rs`), the `Copy` handles the views take (`watch.rs`), and data the view-model builds for a port.
- **view/**: Leptos components, and every pure renderer that produces markup (SVG), with the component tests.
- **Slice root**: `mod.rs` (wasm-bindgen entry points), `i18n/`, `CLAUDE.md`.
- Where a file could go in two places, it goes where its callers keep the direction model ← vm ← view, so a move does
  not add an upward reference.

`model.rs`, `vm.rs` and `view.rs` (where they exist) become `model/mod.rs`, `vm/mod.rs` and `view/mod.rs`, so
`crate::<slice>::model::…`, `…::vm::…` and `…::view::…` keep their meaning. Other files gain a segment, e.g.
`crate::junction::read_model` becomes `crate::junction::model::read_model`.

## File-by-file mapping

### street/

| Now | After |
|---|---|
| `model.rs`, `model/*` | `model/mod.rs`, `model/*` (unchanged) |
| `measures.rs` | `model/measures.rs` |
| `pieces.rs` | `model/pieces.rs` (the importer in `city/` uses it) |
| `vm.rs` | `vm/mod.rs` |
| `impact.rs` | `vm/impact.rs` (it uses `text::check_label`; only the vm and the watch use it) |
| `text.rs` | `vm/text.rs` |
| `keys.rs` | `vm/keys.rs` |
| `watch.rs` | `vm/watch.rs` |
| `row.rs` | `vm/row.rs` (the `Row` trait and the cell ids the vm, the watch and the strip share; no Leptos) |
| `view.rs` | `view/mod.rs` |
| `page.rs`, `inspector.rs`, `notes.rs`, `panel_tabs.rs`, `piece_list.rs`, `speed_view.rs`, `strip.rs`, `calming_strip.rs`, `calming_tools.rs`, `curb_strip.rs`, `curb_tools.rs`, `green_strip.rs`, `green_tools.rs`, `stop_tools.rs`, `zone_tools.rs` | `view/…` |
| `drawing.rs` | `view/drawing.rs` |
| `iso/` | `view/iso/` |
| `tests.rs` | `view/tests.rs` |

### junction/

| Now | After |
|---|---|
| `model.rs`, `model/fixtures.rs` | `model/mod.rs`, `model/fixtures.rs` |
| `read_model.rs`, `geometry.rs`, `beyond.rs`, `curbside.rs`, `stops.rs`, `signal.rs`, `turbo.rs`, `turbo_ring.rs`, `turbo_spiral.rs` | `model/…` |
| `vm.rs` | `vm/mod.rs` |
| `text.rs`, `keys.rs`, `watch.rs` | `vm/…` |
| `drawing.rs`, `figure.rs`, `phase_svg.rs`, `iso_marks.rs`, `frame.rs`, `turns.rs`, `inspector.rs`, `notes.rs`, `page.rs` | `view/…` |
| `iso/` | `view/iso/` |
| `tests.rs` | `view/tests.rs` |

`junction` has no `view.rs` today, so `view/mod.rs` is new and only declares the modules.

### map/

| Now | After |
|---|---|
| `network.rs`, `source_quality.rs`, `corridor.rs`, `editable.rs` | `model/…` (new `model/mod.rs` declares them) |
| `vm.rs` | `vm/mod.rs` |
| `areas.rs`, `selection.rs` | `vm/…` (signal-backed sub-view-models) |
| `camera.rs` | `vm/camera.rs` (the vm's camera; `svg.rs` and `view.rs` read it) |
| `report.rs`, `report_review.rs` | `vm/…` (the report worded in the interface language; `report_review` uses `vm::AreaStreet`) |
| `overlay.rs`, `style.rs` | `vm/…` (the GeoJSON and layer JSON the vm hands to the `Mapper` port; the vm calls them) |
| `view.rs` | `view/mod.rs` |
| `areas_view.rs`, `network_view.rs`, `selection_view.rs`, `source_view.rs`, `home.rs`, `svg.rs` | `view/…` |
| `tests.rs` | `view/tests.rs` |

## Mechanics

- One commit per slice (street, junction, map), each made of `git mv` plus the edits it needs, each passing
  `just test` and `just format-check` on its own. A last commit updates the documentation if it was not done per slice,
  and one bumps the version.
- **Paths**: every `use`/path to a moved module is rewritten to the new path, in the whole crate (other slices import
  `street::model`, `street::pieces`, `street::measures`, `street::drawing`, `street::vm`, `junction::read_model`,
  `junction::signal`, `junction::turbo`, `junction::figure`, …).
- **`super::`**: a moved file's `super::x` now points at its new parent. Rewrite such paths as `crate::<slice>::…`.
- **Visibility**: `pub(super)` on an item of a file that moves one level deeper narrows it (to `model`, say, instead of
  the slice). Where that breaks a caller, use `pub(in crate::<slice>)`, which keeps today's reach. Do not widen to
  `pub(crate)` or `pub`.
- **`include_str!`**: relative paths in moved files gain one `../` (`tests.rs`, `iso/*`, `street/text.rs`'s
  `include_str!("model.rs")` which becomes the path to `model/mod.rs`; `junction/tests.rs` reads `i18n/*.ftl`).
- **`#[cfg(test)] mod tests;`** moves from the slice's `mod.rs` to `view/mod.rs`; test helpers called from other files
  (`crate::junction::tests::…`) follow it to `crate::junction::view::tests::…`. In `junction`, where `vm/text.rs` and
  `model/read_model.rs` reach the fixtures, it is declared `pub(in crate::junction) mod tests;` (today's reach, the
  slice). The same holds for any module private at the slice root today that the slice's `mod.rs` still uses after it
  moves (map's `report` and `report_review`): `pub(in crate::<slice>)`, no wider. (Builder's call, settled.)
- **i18n guard** (`src/i18n_guard.rs`): every moved path in `MIGRATED` and `LEFT_OUT` is updated (both the path string and
  the `include_str!`). Its test that every file of these slices is listed must still pass; the new
  `map/model/mod.rs` and `junction/view/mod.rs` get an entry like any new file.
- **CI selection** (`scripts/ci/spec-map.json`): the rules match `src/street/**`, `src/junction/**`, `src/map/**`, so no
  change.
- **Docs**: the root `CLAUDE.md` (Architecture: the list of each slice's files, the SVG renderers' paths), the slices'
  `CLAUDE.md` files (`street/`, `junction/`, `map/`, and `shared/` and `standards/` where they name these files), and
  and every comment or reason string that names a moved file: `web/app.css`, `web/editor.css`, `shared/flat.rs`,
  `shared/iso/ground.rs`, `street/iso/projection.rs`, `street/notes.rs`, `street/text.rs`, the header comments of the
  `.ftl` files (street, junction, map, restriction, both languages), `docs/glossary-fr-ca.md`, and the reasons in
  `i18n_guard.rs`'s `NOT_MESSAGE_IDS` / `ALLOWED_LITERALS` (e.g. `junction/iso/scene.rs`). A final
  `grep` for the old paths finds none outside `docs/superpowers/`. The dependency rule in the root `CLAUDE.md` gains one line: inside
  `street/`, `junction/` and `map/`, `model/`, `vm/` and `view/` hold the layers. Older plans and specs under
  `docs/superpowers/` are history and stay as written.
- **Version**: patch bump of `crates/cityloom_editor/Cargo.toml` (`0.71.0` → `0.71.1` at the time of writing; whatever
  main has plus a patch) and `cargo update -p cityloom_editor`.
- Work happens on the `mvvm-folders` branch in its own worktree (`../mvvm-folders`), from `origin/main`.

## Known exceptions (kept, not fixed)

References that point from a lower layer to a higher one, or from one slice into another's view, exist today and stay:

- `junction/model/read_model.rs` uses `view::turns::compass_name_key` (a message key for a bearing); its tests also call
  `view::iso` and `view::phase_svg`.
- `street/vm/mod.rs` tests call `view::green_tools::{fine_typed, fine_shown}`.
- Tests in `street/model/mod.rs` and `street/model/{curb,zones,calming}.rs` call `crate::street::vm::text`.
- `junction/vm/text.rs` tests use the fixtures in `view::tests`.
- `map/vm/report*.rs` use `street::view::drawing` and `junction::view::figure` (the report's figures).
- `shell/tests.rs` builds a `street::vm::StreetVm` (test only).

## Verification

- `just format-check`, `just test` (with `DATABASE_URL`) after each slice's commit.
- `just e2e` once at the end (the build must still produce the same pages).
- `git log --follow` on a few moved files shows their history.
- `git diff -M --stat origin/main` shows the moves as renames with small edits, not deletes and adds.

## Left to the builder

- The exact order of moves inside a slice, and whether the documentation is updated per slice or in one commit.
- Any file not named above (one added on main meanwhile) goes by the layout rules.
