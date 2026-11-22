# Junction editor layout: the street editor's overhaul, on the junction page

CLM-32. The street editor was reworked after the editors' UX review of 2026-11-14
(`2026-11-14-street-editor-layout-design.md`, whose non-goals say "the junction editor and the map (later, the same
pattern)"). The junction page took the first part of it in #215 (one level of details at a time, under a path). This
design brings the rest, in steps, each its own pull request. Step 1 is designed in full here; the later steps are
sketched and get their own design when they start.

## Why

The person asked for the junction editor to be user friendly, after the street editor's overhaul: the two editors
should work the same way, and someone opening the junction editor for the first time should find their way.

Today the junction page still splits attention three ways, as the street page did before #205: the details on the
left, the effect of an edit (the status line, Checks, Changes) in a Notes sidebar on the right, the drawing between.
The title and the tools (Undo, Redo, Start over) sit in the drawing's column and the scenarios in the details panel.
Someone who learned the street page finds every part of the junction page on the other side.

## Goals (the whole of CLM-32)

- The junction page uses the street page's layout and panel patterns: one panel for what is selected; the effect of
  an edit in view next to the drawing, without a second sidebar; the title and the commands always reachable.
- The signal plan level puts the phases first, the policies behind folds that say what is set.
- The common tasks (change the control, add or shorten a crossing, ban a turn, add a bike box or a protected corner,
  set up a signal, read the checks and fix what fails) are easy for a first-time user.

## Non-goals

- Changing the model, what is saved, the checks, or what any edit does.
- Editing on the drawing itself (option 4 of the review).
- The map page.

## Anti-goals (a failure even if it works)

- A keyboard or screen-reader path that gets longer: every control reachable today stays reachable in no more steps
  (the street spec's anti-goal).
- A layout that hides the drawing behind the results.
- The two editors drifting apart again: what the junction page shares with the street page is the same markup, CSS
  and components, not a copy.
- A read-only page (#222) on which any edit becomes possible.

## The steps

1. **The layout** (this design, in full below): the details on the right, the notes as the shell's tray under the
   drawing, the title, scenarios and tools fixed at the top.
2. **The signal plan level.** Each signal policy below the phases (pedestrian calls, vehicle actuation, bicycle
   detection, transit priority, pre-emption, time-of-day plans) becomes a fold that says what is set, as #215 did for
   a street's transit priority and position.
3. **Label-column panes** for the junction, a street and a crossing, as the street's #212–#214.
4. **Previews where a choice changes much at once**, using the Transform dialog's pattern (#224) where it fits.
   Candidates to check first: the junction's control (stop, signal, roundabout, turbo) and a phase's pattern.
5. **The phone layout**, shared with the street page (the street spec's step 3, not built yet).

Turns allowed, getting across and where paths meet stay in the notes as the Streets tab through these steps (the
person's choice, option A). Giving a street's level its own turns and crossing is a possible later step, not part
of these five.

## Step 1: the layout

### Approaches

- **The street page's layout, shared (chosen).** The junction page takes the street page's markup structure, its
  CSS (one class both pages carry) and the shell's `Tray` component. The junction keeps its own panel contents.
- **A copy of the street page's CSS under a junction class.** Rejected: some 60 lines that must change together,
  which is the drift this work removes (and SonarCloud flags duplication).
- **The tray written by hand in `intersection.html`.** Rejected: the street page moved its tray into
  `shell::tray::Tray` (#207, #208) so that a tab and its panel always name each other; a hand-written copy would lose
  that.

### What the person sees

At a desktop width (900 px and wider):

```
┌ top bar ───────────────────────────────────────────────────────────────┐
├────────────────────────────────────────────────────────┬─┬─────────────┤
│ Title card · Scenarios · Undo Redo Start over          │D│ Details     │
│ ┌ drawing ───────────────────────────────────────────┐ │e│ Junction ›  │
│ │                     2.5D junction                  │ │t│  street     │
│ │                                                    │ │a│             │
│ │ plan key                                           │ │i│ (inspector) │
│ └────────────────────────────────────────────────────┘ │l│             │
│ ┌ tray handle: status · Streets Checks Changes · Notes^┐│s│            │
│ │ (open: the chosen tab's panel, at most ¼ the height) ││ │            │
│ └──────────────────────────────────────────────────────┘│ │            │
└────────────────────────────────────────────────────────┴─┴─────────────┘
```

- **The details panel is on the right edge**, its "Details" edge tab at the panel's left edge; `[` still opens and
  closes it. Closed, the tab goes to the window's edge and the drawing takes the room. Its contents are today's: the
  path and the level shown (`#inspector`).
- **The column is three parts**, as on the street page: the head (title card, scenarios, tools), which never scrolls
  away; the drawing with its plan key, which takes the room left; and the notes' tray at the bottom.
- **The scenarios move from the details panel to the head**, beside the title, where the street page has them
  (builder's call on the exact order inside the head, matching the street page's).
- **The notes are the shell's tray** (`shell::tray::Tray`). Its handle holds the status line (`#fit`, today in the
  statusbar under the drawing), the three tabs and the toggle; `]` opens and closes it. Its tabs and panels are
  today's, unchanged in content:
  - **Streets** (`t-space` / `p-space`): getting across, turns allowed (with its hint), where paths meet and the
    conflict note.
  - **Checks** (`t-checks` / `p-checks`): does it work, the passing checks' fold, guided goals, the Atlas source.
  - **Changes** (`t-changes` / `p-changes`): your changes.
  - After the panels, in the body: the fine print ("Sample numbers and limits…") and the title block (`#title-block`,
    "About this junction"), as the street's tray holds its title block.
- **The tray starts closed to its handle** (`NotesAtStart::Closed`, as the street page; today the junction's notes
  open on a screen 1,360 px or wider). A remembered choice wins; choosing a tab opens it on that tab. Open, it takes at
  most a quarter of the column's height and its body scrolls. (Builder's call carried over from the street spec: open
  at the start, it takes room from the drawing.)
- **Under 900 px** the junction page does what the street page does today under that width, from the same CSS: no
  new phone design (step 5).

### How it is built

- **Markup (`web/intersection.html`).** The body carries the class the street page's layout is scoped to. The
  structure follows `web/street.html`: `.stage-main` holds `.stage-head` (title card, `#scenario-slot`, `.tools` with
  `#history`), a scrolling part with the drawing section (`#scroll`, `#plan-key`), and `#notes-slot`; after `main`,
  the `#inspector-toggle` edge tab (`right`) and the details `aside` (`.panel.details`, `#details-panel`, holding
  `#inspector`). The Notes `aside`, its tabs and its panels leave the markup; `web/tray.css` is linked. The edge tabs'
  and panel's ids stay (`#details-panel`, `#inspector`, `#inspector-toggle`, `#notes`, `#notes-toggle`, the tab and
  panel ids, `#fit`, `#title-block` and every slot `mount_page` fills).
- **CSS (`web/editor.css`).** The rules scoped to `.street-page` are scoped to one class both pages carry (builder's
  call on its name; `.tray-page` is the suggestion), and their comment says both pages use them. The junction's
  `.panel.left` rules in `editor.css` (the inspector's top border; the narrow layout's order) follow the panel to
  `.panel.details` or go where they no longer apply. `app.css`'s `.panel.left` / `.panel.right` stay: the map uses
  them.
  Only the layout rules move to the shared class. The street's label-column look for its panel (the
  `.street-page #inspector …` rules) stays the street page's: the junction's inspector shares its class names, and
  giving it that look is step 3 (builder's call: until then the CSS has the shared layout class and the street's
  own class side by side; `street/CLAUDE.md`'s "the junction's inspector keeps its own look" still holds).
- **The drawing's height (`junction/view/drawing.rs`, `view/frame.rs`).** Today it is the window's height less a
  fixed 555 px (at least 340), the room the old layout left. In the new column the drawing takes the height the
  column leaves it, measured as the street page measures it (`street/view/mod.rs`: a `ResizeObserver` on the
  scroller, which follows the tray opening and closing and the head's height). `Frame::fit` takes that room
  instead of the window's height on the junction page; the compare page, which has no such column, keeps today's
  formula, so it does not change (builder's call: measure, not retune the constant, since the tray's height
  changes as it opens and closes). The "whole drawing in view" and handle-in-view tests hold at 1,440 × 900 and
  1,280 × 800. `junction/CLAUDE.md`'s note on why the scenarios sat in the details panel (the 555 px) is replaced.
- **The tray (`junction/view/notes_tray.rs`, new).** `JunctionNotes`, as `street/view/notes_tray.rs`: a function per
  tab giving its `TrayTab` and its panel, the panels holding the empty slots `mount_page` fills next, the status
  slot `fit-slot`, the fine print and the title block as the tray's children. The headings and the hint that are
  `data-i18n` attributes in the markup today become `tr` calls with the same message keys; no message changes.
  `mount_page` mounts it into `#notes-slot` first, before the slots inside it are filled, and the page script still
  calls `mount_shell` after `mount_page`.
- **The shell.** `Editor::mount_shell` passes `NotesAtStart::Closed`. The shell binds the tray's toggle and tabs by
  their ids, as on the street page; nothing in `shell/` changes beyond that.
- **The stuck page (`mount_bare_shell`).** A junction whose streets can no longer be drawn shows only its name and
  the message in the details panel. With the notes no longer in the markup, it mounts no tray; what it removes
  (`.plan-drawing`, `.statusbar` today) follows the new structure, and the shell copes with a page with no notes
  toggle (builder's call how; the stuck page's specs keep passing).
- **The compare page** draws its junction half through `mount_compare_junction`, which uses none of this markup; it
  does not change.

### Keyboard order

The details `aside` comes after `main` in the markup, as on the street page (the person's choice: follow the street
page), so the order Tab follows matches what is seen: the head (title, scenarios, tools), the drawing, the tray's
handle, then the details. Today the junction's details come before `main`, so from the top of the page the inspector
is now further by Tab. The anti-goal is measured as on the street page: from the drawing, where the skip link lands,
every control is reachable in no more steps than before, and `[` still opens and closes the details.

### Read-only

Nothing about read-only changes: the details' edit sections stay locked, the turns table's buttons disabled, the
History not drawn (#222). The scenarios keep what they do read-only today in their new place.

### Tests

Test-driven, the failing test first.

- **Browser (`e2e/tests/editors.spec.ts`).** The street's "panel and tray" tests run on the junction page too
  (parametrised over both pages, not copied): the details right of the drawing, overlapping it nowhere, no
  `.panel.left`; the tray under the drawing with its handle (`#fit`, the first tab, the toggle) in view; the title
  (the junction's name) and `#undo` in view; closing the tray leaves its handle and a tab opens it; the open tray at
  most a quarter of the column; the edge tab at the panel's left edge and at the window's edge when closed. The
  junction's "start closed when narrow" test becomes the tray's (closed to its handle at 1,280 px, open when asked,
  open after a reload). Specs that open the junction's notes by the old sidebar or look for the statusbar are updated
  to the tray; no spec loses what it checks.
- **Component (`junction/view/tests.rs`).** The tray has the three tabs, each naming its panel, in both languages;
  every slot `mount_page` fills is inside it. The tests that read `intersection.html` (every `data-i18n` key exists,
  English text is the English message, no text without its message) keep passing on the new markup.
- **Shell (`shell/tests.rs`).** The junction page's notes start closed.
- **Guards.** `junction/view/notes_tray.rs` goes in `i18n_guard.rs`'s `MIGRATED`; `layout_guard.rs` is satisfied
  (it is under `view/`).

### Constraints

- MVVM: the tray only draws; no view-model change is expected. If one is needed it is tested natively.
- English and fr-CA: no new message is expected; any that is added goes in both `.ftl` files, OQLF rules.
- WCAG 2.2 AA: focus is never hidden under the sticky bar or the tray's handle; `[` and `]` keep their
  `aria-keyshortcuts`; the tray's tabs are a `tablist` as on the street page.
- The version rises (minor) and `Cargo.lock` follows (`cargo update -p cityloom_editor`).
- The docs that describe the junction page's layout are updated with it: `src/junction/CLAUDE.md`, `e2e/CLAUDE.md`
  where it names the junction's notes, the comment over the layout in `editor.css`, and the user documentation in
  `website/` where it says where the junction's details and notes are.

### Left to the builder

- The shared CSS class's name.
- The order inside the head, matching the street page's.
- How the stuck page avoids a dangling notes toggle.
- Splitting step 1 into more than one commit (one per behaviour, each with its test).
