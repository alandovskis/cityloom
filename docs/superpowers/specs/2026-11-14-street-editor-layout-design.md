# Street editor layout: one selection panel and a results tray

Layout option 1 of the editors' UX review (2026-11-14), done for the street editor first. The person asked for the
option and skipped the questions; every choice below not in the option itself is the builder's call and is marked so.

## Why

The street page splits attention three ways: the left panel edits, the right Notes panel shows the effect, the drawing
sits between. In the left panel, zone and curbside commands come before the selected piece, so "Driving lane, 3.0 m"
sits halfway down; the title and toolbar (Add, Undo) scroll away with the drawing column; on a narrow screen the
drawing, the details and the notes stack, so a change and its effect are never in view together.

## Goals

- One panel for what is selected, whatever was chosen last, with the path to it in words.
- The effect of an edit (status, Impact, Checks) in view next to the drawing, without a second sidebar.
- The title and the commands always reachable while working on the strips below the drawing.
- On a phone, the drawing and the panel in view together.

## Non-goals

- The junction editor and the map (later, the same pattern).
- Editing directly on the drawing (option 4 of the review).
- Any change to the model, what is saved or the checks. The panel's name changes from "Piece details" to "Details",
  since it now holds the street's settings too.

## Anti-goals

- A keyboard or screen-reader path that gets longer: every control reachable today stays reachable in no more steps
  than one press on the path.
- A layout that hides the drawing behind the results.

## Three pull requests

### 1. The selection panel (this branch first)

- The details panel holds one level at a time: **Street** (speed limit, standards), **Zone** (the zone commands),
  **Piece** (the inspector), **Curbside** (curbside commands), **Stop**, **Raised element** (calming), **Green
  element**. The level shown is the one the person reached last: pressing or choosing a piece shows Piece, a zone
  cell Zone, a curbside cell Curbside, and so on; adding a piece, stop, element or tree shows it.
- Above it, a breadcrumb, the path to the level shown: `Street` for the street; `Street › Zone 1` for a zone;
  `Street › Zone 1 › Driving lane` for a piece, which belongs to its zone's section; `Street › Parking` for a curbside
  use, a stop, a raised element or a green element, which run along the street whatever its zones. Each step before
  the last is a button that shows that level; the last is the current one (`aria-current="location"`).
- A level changes only by the person's choice: a press or key in a strip, a step of the path, or a command that adds
  something (which is then chosen). An edit never moves the panel to a strip's stretch on its own: a strip keeps its
  choice while its row comes and goes with the pieces (from the review). A piece just added is shown. A sync reload keeps every choice.
- With nothing chosen the panel shows the Street level (builder's call, as the junction editor's panel does with
  nothing selected), with a line saying how to choose something and a button to the selected zone's commands
  ("Zone commands: Zone 1"), since Split was on the panel by default before (from the review).
- A level whose thing is no longer chosen (a piece removed, a selection let go with Escape) shows its parent
  instead: a piece its zone, the others the street (builder's call).
- The Piece / Street tabs and their remembered choice (`cityloom-street-panel`) go. The level is not remembered
  across a reload (builder's call: it follows the selection, which is not remembered either).

### 2. The layout

- The details panel moves to the right edge; the Notes leave the right edge and become a tray at the bottom of the
  drawing column (sticky), its handle the status line with the four tabs; `]` opens and closes the tray as it
  opened and closed the Notes, `[` the panel.
- The title card and the toolbar sit at the top of the column and never scroll away: the column is the head, a
  scroller (the drawing and the strips under it) and the tray (builder's call: three parts rather than sticky
  elements, so the tray takes room from the scroller instead of covering it).
- The tray starts closed to its handle (builder's call: open, it takes room from the drawing at 1440 by 900); a
  remembered choice wins, and choosing a tab opens it. The drawing is laid out to fit the room the column leaves it,
  narrower and centred when a long title, the welcome or the open tray leave less.
- Scoped to the street page (the junction page shares the shell and `editor.css` and keeps its layout).

### 3. The phone

- Under 900 px: the drawing at the top, the panel as a sheet from the bottom at half the height, which can be
  opened to full height and closed; the tray below the strips.

## Constraints

- MVVM: the level shown is view-model state (`StreetVm`), tested natively; views only bind it.
- English and fr-CA for every new string, whole sentences, the OQLF rules (`docs/glossary-fr-ca.md`).
- WCAG 2.2 AA: the breadcrumb is a `nav` with a list; focus never lands on a hidden control; a level change by the
  breadcrumb moves focus to the path's current step, from which Tab enters the level.
- Element ids the specs and the shell use stay (`#inspector`, `#zone-tools-slot`, `#speed-slot`, `#notes`, …).
- Every PR raises the version (minor).
