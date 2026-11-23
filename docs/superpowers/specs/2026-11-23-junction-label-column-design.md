# Junction details: a label column, as the street's

CLM-51, step 3 of CLM-32 (`2026-11-22-junction-editor-layout-design.md`, "The steps", item 3).

## Why

The person asked for the junction editor to be user friendly, after the street editor's overhaul. The street's details
panel lays a setting out as a label column: the setting's name in a narrow column on the left, its control beside it,
each setting on its own ruled row, the actions near the top (#212 to #214). The junction's details panel stacks each
section's heading over its controls, so the two editors' panels read differently.

## Goals

- All six levels of the junction's details use the street piece pane's label column (the person's choice, A): the
  Junction, a street, a crossing (a street's level reached for its crossing), a corner, a lane, and the bus and bike
  symbols.
- Wide content (a list of lanes, of streets, of a lane's destinations; a fold's body) spans both columns under its
  label, as it would not fit 250 px.
- A level's actions (a street's Remove, a crossing's Mark or Remove, a lane's way back to its street) sit under the
  level's name, as the street piece pane's Move and Remove do.

## Non-goals

- The signal plan level, which keeps its layout (its phases and tables).
- Any change to what a level offers, its wording, or the model.
- The street's zone and street panes (`.zone-row`), a second label-column style, left as it is.

## Anti-goals

- A control cut off or overflowing at the panel's width (340 px) or on a 280 px phone.
- A longer keyboard path: moving the actions up puts them earlier in Tab order, never later.
- A read-only page on which anything becomes editable.

## Approaches

- **One class both pages carry (chosen).** The rules now scoped `.street-page #inspector` (the grid of `.insp-sec`, its
  `.note-h` in the first column, the radio tiles three across) move to a class, `.insp-columns` (builder's call on the
  name), set on the street's `#inspector` and on each junction level's container. A section that spans both columns
  carries `.insp-span`.
- **Widen the rules to `.tray-page #inspector`.** Rejected: the junction's signal plan level also draws into
  `#inspector`, and its phase cards and tables would be put in a 76 px column.
- **A copy for the junction.** Rejected: the duplication step 1 removed (SonarCloud failed on it).

## The design

- **CSS (`web/editor.css`).** `.street-page #inspector` rules become `.insp-columns` rules, unchanged in value; the
  street's `#inspector` gets the class (its look is unchanged, pinned by its specs). `.insp-columns .insp-span > *`
  spans both columns, its `.note-h` on its own row above.
- **Markup (`junction/view/inspector.rs`).** `panel_of`'s level views (not the signal plan's) are wrapped in
  `<div class="insp-columns">`. Each level's sections keep their ids and `h3`; a section whose body is a list or a
  fold gets `.insp-span`. A level's actions move into an `.insp-actions` row under `head(…)`, as the street's.
- **Radio groups** become the street's tiles where they are `.opts[role="radiogroup"]` (the crossing's Shorten choice,
  the junction's control if a radio group): builder's call, kept only where three across fits their labels in French.

## Constraints

- Test-driven: component tests that each of the six levels is inside `.insp-columns` and the signal plan level is not;
  that a level's actions come before its first section; e2e at 1,440 px and 280 px that nothing overflows the panel,
  in English and French; the street's existing specs unchanged.
- English and fr-CA: no new message expected.
- WCAG 2.2 AA: label and control stay associated (the `h3` names its section, `aria-labelledby` unchanged); targets
  at least 24 px.
- Version: minor; `Cargo.lock` follows; `junction/CLAUDE.md` and `street/CLAUDE.md` say the shared class.

## Left to the builder

- The class names, and which sections are `.insp-span`, by what fits at 340 px.
- Whether a level with a heading only (the bus and bike symbols) needs anything beyond the class.

## As built

- A crossing's level offers Remove only: with its crossing off, the model shows the junction's level, so Mark stays in the
  crossing's row of the street's level.
- A fold's number fields are beside their names; only the transit section spans both columns. The control's and the bus
  lane's choosers span them too: their French values do not fit beside a name.
- The junction's column is 88 px (« Signalisation » does not fit 76), and at 270 px or less the name is above its control.
- The bus lane's chooser is still cut where its route is longer than the panel (two streets' names, « Avenue Laurier Est
  (nord-est) vers Avenue Laurier Ouest (sud-ouest) »), as it was before; the bus lane's own level says the route whole.
- So are the longer transit measures in French (« G2 Voie de dépassement de file en bordure de trottoir »): measured on
  main's code, their lists had the same 278 px at 1,440 px, and less of the text on a 280 px phone than now.
