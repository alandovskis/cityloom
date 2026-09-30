# Lane inspector: surface and curb material, width, in a left sidebar

Date: 2026-09-30

## Goal

Let a resident select a piece of the street and change its **width**, its
**surface material** and, for sidewalks and bike lanes, its **curb material**,
from a sidebar to the left of the drawing. Material changes are part of the
model, so they undo, redo and appear under "Your changes".

## Decisions taken with the user

- Material means a surface finish per piece, chosen from a short list that
  depends on the kind of piece. It is modelled in Rust, not a UI-only label.
- The sidebar is an inspector for the selected piece, collapsible like the
  notes panel.
- Sidewalks and bike lanes also carry a curb material. Curb list, for both:
  granite, concrete, asphalt, or none (flush). Default is concrete.
- Material is descriptive for now. It does not change the people-moved numbers
  or any check. The existing "sample numbers, not real measurements" note
  still applies, and the material and curb lists are synthetic placeholders.

## Model (Rust)

`catalogue.rs`
- `MATERIALS`: `asphalt`, `concrete`, `permeable` (permeable paving), `brick`
  (brick pavers), `grass`, `planted` (planted bed), `gravel`. Each has an id
  and a display name.
- `CURBS`: `granite`, `concrete`, `asphalt`.
- `Kind` gains `materials: &[usize]` (indices into `MATERIALS`, the first is the
  default) and `has_curb: bool`.

  | Kind | Surface materials | Curb |
  |---|---|---|
  | sidewalk | concrete, brick, asphalt, permeable | yes |
  | bike | asphalt, concrete, permeable | yes |
  | travel, bus, loading | asphalt, concrete, permeable | no |
  | parking | asphalt, permeable, concrete | no |
  | planting, median | grass, planted, gravel | no |

`model.rs`
- `Segment` gains `material: usize` and `curb: Option<usize>`. A new segment
  gets its kind's first material and, if `has_curb`, `Some(concrete)`.
  Samples load with defaults.
- `set_material(uid, material) -> bool`: false if the material is not allowed
  for the kind or is unchanged. Goes through `edit()`, so it is one revision
  and follows the existing undo, redo and gesture rules. Label: "Parking
  surface: permeable paving".
- `set_curb(uid, curb: Option<usize>) -> bool`: false if the kind has no curb,
  the value is invalid, or unchanged. Label: "Sidewalk curb: granite" or
  "Sidewalk curb: none".
- `SegView` gains `material` (id string), `curb` (id string or null).
  Allowed materials come from the catalogue JSON, so the page holds no rules.
- Width uses the existing `set_width` and `nudge_width`. No new width code.

`lib.rs`
- Export `set_material(uid, material)` and `set_curb(uid, curb)`, where `curb`
  is an index into the curb table, or -1 for none.
- `catalogue()` includes each kind's `materials` and `has_curb`. A new
  `materials()` export returns the surface and curb tables.

Tests (native, in `model.rs`): allowed and disallowed material; curb on a kind
without one is rejected; none and each curb value; no-op returns false;
undo and redo restore material and curb; a new segment gets the defaults;
revision labels; every kind's first material is in its own list.

## Page

Sidebar (`index.html`, `style.css`, `app.js`)
- New `<aside class="inspector">` at the left of `.sheet-body`. The grid
  becomes `240px 1fr 340px`. It collapses through a header button beside the
  notes button and the `[` key. State is remembered under its own
  `localStorage` key and set as `data-inspector="closed"` on `<html>`.
- Nothing selected: "Select a piece to change its width and surface."
- A piece selected shows:
  - name and swatch;
  - **Width**: a number field in the current units, − and + steppers that
    call `nudge_width` (100 mm), and the allowed range as a hint;
  - **Surface**: a radio group of the kind's allowed materials, each with a
    small hatch swatch and a name;
  - **Curb** (sidewalk and bike lane only): a radio group of granite,
    concrete, asphalt, none.
- Below 1100px it stacks under the drawing instead of sitting beside it.
  Hidden in print.
- Keyboard: radios move with arrow keys; changing selection in the drawing or
  the schedule updates the inspector; focus is not stolen from the drawing.

Schedule
- The Type cell gets a second, softer line: the surface, and for curb kinds
  "· granite curb" or "· no curb".

Drawing
- Engineering view: the hatch identifies the **material** and the name label
  identifies the kind, as in engineering sections. Seven new surface hatch
  patterns, plus three for curbs.
- Standard view: unchanged hatch (it identifies the kind).
- Curb: a small block above the ground line on each side of a sidewalk or bike
  lane that faces a road piece (travel, bus, parking, loading). About 150 mm
  wide at drawing scale (at least 5px) and about 12px tall. Filled with the curb
  hatch in Engineering, plain ink outline in Standard. "None" draws no block.
  Where a bike-lane curb is drawn, the solid lane-marking block at that same
  boundary is omitted, since the curb replaces the line.

Docs
- `DESIGN.md` notes the left inspector and that material and curb lists are
  synthetic. `README.md` mentions nothing new.

## Out of scope

Effects of material on outcomes, costs, checks or capacity. Layered
pavement build-ups. Curb height or width as editable values. Saving, sharing
or exporting. A materials legend outside the inspector.

## Verification

`cargo test`, then rebuild the wasm and check in the browser: select each kind,
change width, surface and curb, undo and redo, keyboard operation, collapsed
and stacked layouts, light and dark, both drawing views, and print.
