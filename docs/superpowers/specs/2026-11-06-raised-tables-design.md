# Raised tables as street elements (issue #147, PR 1 of 2; part of #77)

Issue #147 (traffic calming: raised tables and chicanes as street elements). This
first PR is the raised table and the raised crossing; the chicane is a second PR
on the same issue and reuses what this one builds (the `calming` list, its
strip, tools, version and checks).

## Intent

A designer can put a raised table (speed table) or a raised crossing on a street,
see it in the 2.5D, and have the design checks say when it does not fit. It is an
element of the street that can be inspected and edited, not opaque geometry.

## Decided by the maintainer

- Raised table first, chicane second (a second PR).
- The edits-document version is the next free number at merge time: provisional
  `CALMING_VERSION = SIGNAL_VERSION + 1` (11); renumbered to 12 on rebase if the
  green-infrastructure PR #127 (which claims 11) merges first.
- A table may cross bus lanes and sit near transit stops; a design check fails
  an overlap with a stop's boarding area or too little clearance from a stop,
  and says why. No emergency-route data exists in the model, so none is checked.
- UI follows curbside uses and stops: a strip row and a tools section on the
  street page's left panel, not the Transform menu.
- Curb extensions are not part of this; modal filters and the school-zone
  template are #149 and #148.

## What it does

`Street.calming: Vec<Calming>` (saved as `calming`, skipped when empty):
`Calming { from_mm, length_mm, kind: Kind }` with `Kind::{Table, Crossing}`
(the chicane will add `Kind::Chicane`). A table lifts the whole roadway across
the street between its curbs, with a ramp at each end; a crossing is a table
with a painted pedestrian crossing on the plateau. `from_mm` is measured from the
anchored end (`Street::from_end`) and snapped to 100 mm; `length_mm` is the whole
table, ramp to ramp. The same conventions as curbside uses and stops apply:
`NoAnchor` (no real OSM end), `Elsewhere` (drawn on a road of another length or
from its other end: shown and checked, not rearranged), `Dragging`, one undoable
revision per edit, edits return a refusal that carries its reason.

Refusals (said as `calming-refused-*`): no anchor, elsewhere, dragging, outside
the street, overlaps another calming element, shorter than the minimum, a
freeway street (a table on a freeway is refused), a street with no roadway
piece in a zone it covers.

Checks (present only when the street has an element; judged in every zone the
element overlaps): `calming-length` (at least `MIN_TABLE_MM`), `calming-speed`
(the street's speed is at most `MAX_TABLE_SPEED_KMH`; a street with no speed
gives a note, never a fail), `calming-stop` (no overlap with a transit stop's
boarding area, and `CALMING_STOP_CLEAR_MM` of clearance; names the stop),
`calming-junction` (clear of the junction by `CALMING_JUNCTION_CLEAR_MM` past
the junction's reach, as `stop-junction` does with `EndInfo.reach_mm`).

Placeholders, all UNVERIFIED (no standard checked; marked so in the code, as the
other figures are): table height 100 mm (drawn at the 2.5D's `RISE` scale like
slabs), ramp length 1.5 m (about 1:15), `MIN_TABLE_MM` 6.0 m (plateau at least
3.0 m), `MAX_TABLE_SPEED_KMH` 50, `CALMING_STOP_CLEAR_MM` 5 m,
`CALMING_JUNCTION_CLEAR_MM` 5 m. Default length 7.0 m on add.

Rendering (street 2.5D, the only drawing of the street): the plateau is the
roadway pieces' tops raised by the table height over the plateau, ramps slope
between the roadway level and the plateau at the two ends, lane markings stop at
the ramp feet, a crossing paints zebra bars across the plateau; the element is
a labelled group with the element's name as its title; its pieces are not
pressable (the strip selects it). Elements outside the 40 m stretch are not
drawn; one cut by its edge is drawn whole or not at all in the reach (as trees
are), judged on the plateau.

UI: `CalmingStrip` (one listbox, `#calming-strip`, a cell per element and per
free stretch, one tab stop, `aria-activedescendant`), keys as the zone and curb
strips (arrows, Home/End, Alt+arrow moves 1 m, Shift 10 m), drag with one
undoable step and Escape to cancel; `CalmingTools` (`#calming-kind`,
`#calming-length`, `#calming-add`, `#calming-remove`). Refusals are announced.
`row.rs`'s shared `Row` is reused (a `CalmingRow`).

## Non-goals

The chicane; the map overlay; junction awareness (an element near a junction
draws and is judged on the street page only); a plan drawing; templates and the
Atlas; time-of-day variants (a roadway that changes with the clock refuses, as
curbside uses do: `TimeWindows`).

## Anti-goals

A table whose figures look like a standard's approval (every figure says
UNVERIFIED); an element that cannot be selected, edited or removed from the
keyboard; any change to how a street without calming is saved, shown or drawn
(byte-identical 2.5D markup and unchanged saved JSON when `calming` is empty).

## My calls

- Copy the structure of curbside uses and transit stops (and #127's freshest
  layout): `street/model/calming.rs`, `calming_shown.rs`, `calming_strip.rs`,
  `calming_tools.rs`, `iso/calming.rs`, messages `calming-*` and `check-*-calming*`.
- `Kind::Crossing` is one kind of the same element, not a second list.
- The element's name for a screen reader is the kind's label plus its place
  along the street, as for curbside uses.
- The glossary gets entries for the French terms (to confirm: "plateau
  surélevé", "traverse surélevée", "dos d'âne allongé"; the first two are the
  usual Québec terms).

## Acceptance

- Native tests for the model (add, move, change, remove, every refusal, snapping,
  elsewhere, undo and redo, the checks and their wording), the version and the
  round trip (a record with `calming` writes the new version; one without is
  written as before; a newer document is left alone), the view-model and
  component rendering (strip, tools), the 2.5D (a table raises the plateau,
  ramps, markings stop, crossing bars; a street without calming is
  byte-identical), English and fr-CA.
- Playwright: add a raised table on the street page by the tools and by the
  keyboard, see it in the drawing and the strip, a check fails for a table near a
  stop, undo removes it, it survives a reload.
- A minor version bump; the number is set at rebase.
