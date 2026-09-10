# Interaction Specification — Streetmix at City Scale

Component specifications follow
`.claude/knowledge/aidlc-design-agent/component-spec-template.md`.

**Standing constraint.** Whether the editing surface is built from real page
elements or drawn on a canvas is deferred to Domain Design (`stories.md`
OQ2/OQ-US7). Every specification below is written so it can be satisfied either
way, and where a canvas choice would make a requirement unverifiable, that is
said in place rather than discovered later. Nothing here presupposes an answer.

---

## The interaction principle, restated

Carried forward unchanged from `wireframes.md`: **select, then act**. Every
editing action completes through selection and a command. Drag interactions,
where offered, duplicate an action already reachable by selection (FR4.3,
FR4.4, AC5.3.1, AC5.3.2).

This is not only an accessibility measure. It is what makes the product usable
on a phone at 360 CSS pixels, where a precise drag on a lane a few pixels wide
is not an interaction anyone can perform.

## The committed editing-action list

`stories.md` binds AC5.3.1, AC5.3.2, AC8.2.6 and AC13.2.1 to this list. It is
reproduced here because the interaction spec is where each action's keyboard
path is defined. Adding an action without a keyboard path fails the build.

| # | Action | Keyboard path | Announced |
|---|---|---|---|
| 1 | Select a lane | Arrow keys within the lane strip; Home/End for first/last | Lane number, type, and provenance state |
| 2 | Change a lane's type | Tab to type select, arrow keys or type-ahead, Enter | New type, and that the change is user-set |
| 3 | Change a lane's width | Tab to width stepper; arrow keys step, or type a value and press Enter | New value with its unit and provenance |
| 4 | Add a lane | Tab to "Add lane", Enter; then choose position and type | Lane added at position N of M |
| 5 | Remove a lane | Tab to "Remove lane", Enter | Lane removed; M lanes remain |
| 6 | Extend along the corridor | Tab to "Extend along corridor", Enter; opens S4 | Corridor selection opened, N connected streets |
| 7 | Undo | Ctrl/Cmd+Z, or Tab to the undo control in the drawer header, Enter | What was undone, in the same words the confirmation used |

---

## Bottom sheet (the drawer)

| Field | Value |
|---|---|
| Component | BottomSheet |
| Description | The cross-section editing surface, presented as a drawer on wide viewports and a two-height bottom sheet on narrow ones |
| Category | layout |

### States

| State | Description | Trigger |
|---|---|---|
| closed | No street selected; map is the whole surface | initial load, or close |
| peek | Street name, header controls, cross-section, prompt | street selected on a narrow viewport |
| full | Peek content plus lane detail and actions | expand from peek, or street selected on a wide viewport |
| loading | Street name and a skeleton cross-section | selection registered, import in flight |
| error | Import failure message with retry and blank-start | import failed or timed out |

### Responsive behaviour

| Breakpoint | Behaviour |
|---|---|
| 360–767 px | Two-height bottom sheet. Peek is roughly 40% of viewport height; full covers the map. The map is never replaced by a second view |
| 768–1023 px | Fixed drawer occupying the lower half; map above |
| 1024 px and up | Fixed drawer occupying the lower third; map above |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `complementary`, labelled by the street name heading |
| Keyboard interaction | Escape collapses full to peek; Escape again closes. Never close from full in one press — a corridor apply is undoable from the header, and a single Escape that discards the whole context is a trap in the other direction |
| Label | The street name (h2) labels the region via `aria-labelledby` |
| Focus management | On expand, focus moves to the first lane. On collapse, focus returns to the control that collapsed it. On close, focus returns to the street on the map or in the street list, whichever initiated the selection |
| Screen reader | Height changes are announced as "expanded" / "collapsed" through the control's `aria-expanded`, not through the status region |

**Two heights means two keyboard entry points.** In peek the first lane may not
be reachable without expanding. The rule: Tab from the header reaches the lane
strip if it is visible, and otherwise reaches the expand control first.

---

## Lane strip

| Field | Value |
|---|---|
| Component | LaneStrip |
| Description | The cross-section itself — ordered lanes, left to right, each individually selectable |
| Category | display + input |

### States

| State | Description | Trigger |
|---|---|---|
| default | Lanes rendered to scale with type labels | cross-section loaded |
| selected | One lane marked as current | selection |
| estimated | A lane whose width is `inferred` carries the hatch | provenance is `inferred` |
| corrected | A lane carrying a `user-set` correction | provenance is `user-set` via US7.3 |
| skeleton | Placeholder blocks, no labels | import in flight |

### Provenance rendering — the three channels

Q2 settles this, and it is the product's central credibility mechanism, so it is
specified once here and referenced everywhere else.

| Channel | Treatment | Serves |
|---|---|---|
| Visual | Diagonal hatch fill on the lane block. Never colour alone (WCAG 1.4.1) | Someone scanning the drawing |
| Textual | The value reads "1.8 m (estimated)" wherever it appears, plus one sentence per view explaining what estimated means (AC4.1.6) | Someone reading the numbers, and P3, who has no street-design vocabulary |
| Accessible name | The lane's accessible name includes its provenance: "Lane 2 of 5, bike lane, 1.8 metres, estimated" (AC4.1.4) | A screen reader, which perceives neither hatch nor parentheses unless told |

A `user-set` value uses the same three channels with the word "your entry"
rather than "estimated". A `mapped` value carries no marking — the unmarked
state means measured, and that asymmetry is deliberate: the burden is on the
uncertain value to declare itself.

**If the hatch fails contrast** at small sizes (WCAG 1.4.11 requires 3:1 for a
graphical object), the textual and accessible-name channels carry the
distinction alone and the hatch is decoration. That fallback is recorded in
`accessibility-checklist.md` rather than assumed away.

### Activation targets

AC5.3.4: every lane's activation target is at least 44 by 44 CSS pixels at any
zoom where its drawn width falls below that. **The target is enlarged beyond the
drawn extent; the drawing is never widened.** A lane's rendered width is data,
and widening it to be tappable would misrepresent the street — the same class of
error as presenting a guess as a measurement.

Under DOM the target is the element's own box, measurable with a bounding-box
assertion under emulated touch. Under canvas it is the hit-test region, and
**making that region queryable by the test harness is a condition of choosing
canvas** — otherwise the only automated route is pixel comparison, which
`team-practices.md` forbids.

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `listbox` with each lane as an `option`, or a native equivalent |
| Keyboard interaction | Arrow keys move between lanes; Home/End jump to first/last; Enter or Space selects |
| Label | "Cross-section of [street name], N lanes" |
| Focus | Each lane individually focusable (AC4.1.5) — the half of NFR4.3 a canvas surface could quietly fail |
| Screen reader | On focus: lane number, position, type, width, provenance |

---

## Width stepper

| Field | Value |
|---|---|
| Component | WidthStepper |
| Description | Decrement, value, increment — with direct typed entry |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default | Current value with unit and provenance qualifier | lane selected |
| editing | Typed entry active | focus in the value field |
| rejected | Out-of-range entry refused, previous value retained | value ≤ 0 or > 20 m |

### Validation

AC5.1.5: zero, negative, or greater than 20 metres for a single lane is
rejected with a stated reason, and the previous value is retained. The 20-metre
ceiling is a sanity bound catching typos and unit confusion — not a design
standard, and deliberately far above any real lane in any jurisdiction, so it
does not encode a country's rules against the jurisdiction-neutral mandate.

The rejection is inline, adjacent to the field, and is announced. It is
`aria-describedby` on the input rather than unassociated text nearby.

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `spinbutton`, or a native number input with explicit min/max |
| Keyboard interaction | Up/Down arrows step; typed entry commits on Enter or blur; Escape reverts the in-progress entry |
| Contrast | Stepper controls meet 3:1 as UI components |
| Screen reader | Announces value, unit and provenance on change |

---

## Undo control

| Field | Value |
|---|---|
| Component | UndoControl |
| Description | Reverses the last editing action, including a corridor apply as one unit |
| Category | action |

Q5: this is a **persistent control in the drawer header**, present in every
drawer state including the phone peek. The confirmation toast names what
happened and points at this control; it does not carry the action.

The header is in the peek deliberately. Without it, undo disappears exactly when
a corridor apply has just changed four streets.

### States

| State | Description | Trigger |
|---|---|---|
| available | An action is reversible | any action on the committed list has occurred |
| unavailable | Nothing to undo | fresh session, or history exhausted |

AC5.5.3: attempting to undo with nothing to undo changes nothing and says so. It
is announced, not silent — a control that does nothing without explanation is
worse than a disabled one.

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `button` |
| Keyboard interaction | Ctrl/Cmd+Z anywhere in the editing surface; Enter when focused |
| Label | "Undo [what]" where the accessible name names the reversible action |
| Screen reader | On activation, announces what was undone using the same words the original confirmation used (AC5.5.4) |

---

## Keep-off-this-device control

| Field | Value |
|---|---|
| Component | KeepOffDeviceControl |
| Description | Uploads the current design to the service so it survives this browser, and removes it again |
| Category | action |

**Design-level, not street-level.** It sits in the map header beside the
design's name and street count, not in the drawer header. The drawer is scoped
to one street; a design spans several, and uploading one street of a corridor is
not a thing anyone wants. On a card in S6 it repeats per design.

### States

| State | Description | Trigger | Criterion |
|---|---|---|---|
| available | The design is on this device only | default | AC8.3.2 |
| in progress | Upload running, progress shown | control activated | — |
| kept | The design exists on the service as well as in this browser; the control becomes "Remove from service" | upload succeeded | AC8.3.1 |
| failed | The design is still on the device and nothing was lost; a retry is offered | upload failed | AC8.3.3 |
| removing | Deletion from the service running | "Remove from service" activated | AC8.3.4 |

### Wording rules

- **"Kept off this device", never "Uploaded".** The user's concern is losing
  work when a browser clears its storage, not the mechanics of a transfer.
- **The failure state says nothing was lost.** A failed upload is not a failed
  save; the design remains on the device. That sentence is the difference
  between a retry and a panic.
- **Removal requires no account** (AC8.3.4). The design was uploaded without
  one, so it must be removable without one.

### Responsive behaviour

| Breakpoint | Behaviour |
|---|---|
| 360–767 px | The map header is three lines: search and menu; the design name and street count; the control alone on the third. **The label is never abbreviated** — see the Label in Name note below. While the sheet is at full height the map header is not visible, and the control is reached through the `Design` disclosure button in the sheet's own header |
| 768–1023 px | Two lines: search and menu; the design name, street count and control sharing the second |
| 1024 px and up | One line beside the design name and street count, as drawn in the wide S3 |

**Label in Name (WCAG 2.5.3).** The visible label must be contained in the
accessible name. An abbreviated visible label with a longer accessible name
fails that unless the visible text is a substring of it. The label therefore
reads "Keep off this device" at every width, and the accessible name is the same
string; anything further goes in `aria-describedby`, which is not part of the
name.

**The full-height case is reached by activation, not by focus.** Keeping a
design off the device is design-level; the full sheet is a street-level surface.
An earlier draft made the map header reachable by tabbing past the sheet's last
element, which collapsed the sheet and relocated focus — **a change of context
triggered by a component receiving focus, which WCAG 3.2.1 On Focus prohibits.**
The replacement is an explicit `Design` disclosure button in the sheet header,
specified in full below. A context change on activation is permitted, and a
visible button is more discoverable than a hidden tab stop.

---

## Design disclosure

| Field | Value |
|---|---|
| Component | DesignDisclosure |
| Description | Reveals design-level information and actions when the bottom sheet is at full height and the map header is not visible |
| Category | navigation |

**It is a disclosure, not a menu.** An earlier draft of this spec called it "a
menu" with arrow-key navigation. That wording invokes the ARIA menu-button
pattern, whose conformance rules are strict and easily broken — every direct
child of `role="menu"` must be a `menuitem`, `menuitemcheckbox`,
`menuitemradio` or `group`, and arrow-key navigation implies typeahead, Home and
End as well. The popup's first line is the design's name and street count, which
is informational text, not an item. Under `role="menu"` that line is a
non-conformant child and assistive technology is not obliged to expose it — a
Name/Role/Value risk (WCAG 4.1.2) on precisely the content the popup exists to
surface at 360 pixels.

Nothing here needs menu semantics. A disclosure button revealing an ordinary
labelled group is simpler, has no pattern-conformance obligations beyond naming
its parts, and does everything required.

### States

| State | Description | Trigger |
|---|---|---|
| collapsed | Button visible, popup hidden | default at full sheet height |
| expanded | Popup visible below the button | button activated |

### Structure

| Element | Role | Notes |
|---|---|---|
| Trigger | `button` with `aria-expanded` and `aria-controls` pointing at the popup | Accessible name "Design"; no `aria-haspopup`, which would signal menu semantics |
| Popup | `group`, labelled by its heading via `aria-labelledby` | **Not** `role="menu"` |
| First line | A heading (`h3`) reading the design name and street count | Informational; not focusable, not an item, and legitimate content inside a `group` |
| Actions | Ordinary `button` elements — "Keep off this device", "See all streets" | Reached by Tab in DOM order, as any two buttons would be |

### Responsive behaviour

| Breakpoint | Behaviour |
|---|---|
| 360–767 px | Present in the sheet header whenever the sheet is at full height, because the map header is then off-screen |
| 768 px and up | Not rendered — the map header is always visible and carries these controls directly |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | Trigger `button`; popup `group`. Stated explicitly because every other component in this file states its role, and the one that did not is the one that produced a finding |
| Keyboard interaction | Enter or Space toggles. **Tab moves through the popup's buttons in DOM order — no arrow-key handling**, because there is no composite widget here. Escape closes the popup and returns focus to the trigger |
| Label | Trigger's accessible name is its visible label, "Design" (WCAG 2.5.3). The popup's name comes from its heading |
| Focus management | Opening does not move focus; the user Tabs into the popup, which is the ordinary disclosure behaviour and avoids a second context change. Closing by Escape returns focus to the trigger. Tabbing past the popup's last button continues into the sheet body — the popup is not a focus trap, because it is not a modal |
| Screen reader | The trigger announces its expanded state through `aria-expanded`; the popup announces its heading on entry |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `button`. Its accessible name is exactly its visible label — "Keep off this device", or "Remove from service" once kept — so the two states are distinguishable without reading surrounding text and WCAG 2.5.3 holds at every width. Any further explanation is `aria-describedby`, which is not part of the name |
| Keyboard interaction | Reachable in the map header's tab order, before the drawer. While the sheet is at full height, it is reached instead through the `Design` disclosure button in the sheet header — see the DesignDisclosure component below for its full specification. **No tab stop ever collapses the sheet or moves focus as a side effect** (WCAG 3.2.1) |
| Screen reader | Progress, success and failure each announced through the polite live region — except failure, which is assertive, because work the user believes is safe may not be |
| Focus management | Focus stays on the control across all state changes; it is never moved to a toast |

**Nothing leaves the device until this control is used** (AC8.3.2). It is the
only affordance in the product that crosses that line, which is what makes Q9's
browser-first decision meaningful rather than nominal.

**AC8.3.5 is not specified here.** The criterion requires an uploaded anonymous
design to be stored against an identifier carrying no name, no contact details
and no cross-site linkage. That is a persistence and privacy mechanism rather
than an interaction, and this stage does not decide it — it is recorded as an
open question in `mockups.md` and handed to Domain Design. The control's states
above map to AC8.3.1 through AC8.3.4 and no further.

---

## Corridor checklist

| Field | Value |
|---|---|
| Component | CorridorChecklist |
| Description | Connected streets as a selectable set, each with a fit status |
| Category | input + display |

Q6 keeps this as one screen; the confirmation replaces the checklist in place
after applying.

### Fit states — four, not two

| State | Meaning | Rendered as |
|---|---|---|
| Fits | Design total width ≤ target carriageway width | The word "Fits" |
| Does not fit | Design exceeds it, shortfall stated | "Does not fit — narrower than the design by N m" |
| Fit could not be checked | Target carriageway width is wholly default-derived, or absent | "Fit could not be checked — no measured width" |
| Not yet checked | Pre-selection | "Not yet checked" |

**Each state is a word, never an icon or a colour alone.** The third state
exists because a comparison between two default-derived numbers carries no
information, and reporting it as a finding would present a guess as a finding —
the precise failure the provenance model exists to prevent (AC6.2.3, AC6.2.4).

### Responsive behaviour

| Breakpoint | Behaviour |
|---|---|
| 360–767 px | Each row wraps to two lines: name and length on the first, fit status on the second. The status is never truncated — it is the reason the row exists |
| 768 px and up | One row per street |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | Native checkbox group inside a `fieldset` with a `legend` |
| Keyboard interaction | Tab into the group, arrow keys or Tab between items, Space toggles |
| Label | Each checkbox's accessible name includes the fit status, so the status is announced with the street rather than as separate decoration |
| Screen reader | The apply outcome is announced through the status region |

---

## Unresolved-correction banner

| Field | Value |
|---|---|
| Component | UnresolvedCorrectionBanner |
| Description | A correction that survived a re-import but could not be re-applied |
| Category | feedback |

### Behaviour

AC7.5.4: the correction is **retained and surfaced — neither applied nor
discarded**. The banner quotes what the correction said, explains that the map
data changed and the lane is gone, and offers two actions.

Q4 puts the entry point in two places: a marker on the street in the map, so a
corridor of ten streets does not require opening each one to discover the
problem, and a badge in the drawer header.

**Neither action is destructive by default.** "Apply to a lane…" enters a
selection mode. "Discard" is the only way to lose the correction and is never
the initial focus.

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `region` with an accessible name — **not** `alert`. It persists rather than interrupting, and an alert that never goes away is a role misuse |
| Keyboard interaction | Precedes the lane strip in tab order, because it is a blocking question about the data below it |
| Screen reader | Announced on drawer open through the status region, once, not on every focus change |
| Focus management | Initial focus is the non-destructive action |

---

## Map overlays and the legend

The map carries four simultaneous overlays: the street network; the current
design's streets highlighted (Q8); markers for streets with unresolved
corrections (Q4); and per-street fit status after a corridor apply (Q6).

**Four overlays without a legend is a map nobody can read.**

### Legend specification

| Requirement | Implementation |
|---|---|
| Form | A collapsible region with a text list, not a floating colour key |
| Content | One row per active overlay: its name, its treatment, and its current count where it has one ("3 streets in this design", "1 street with an unresolved correction") |
| Colour | Every overlay is distinguished by shape or pattern as well as colour (WCAG 1.4.1) |
| Keyboard | Reachable in tab order after the map's street-list alternative |
| Screen reader | The legend is the text equivalent — the map's own accessible description references it |
| Narrow viewport | Collapsed by default at 360 px; the toggle states how many overlays are active |

**Open**: whether four overlays plus a legend survive at 360 CSS pixels is a
question for the first build. The treatment is specified; that it works is not
asserted.

---

## Status messages and the live region

US13.1 requires every status the product reports visually to be announced
without moving focus (WCAG 2.1 AA SC 4.1.3). This holds whichever way the
canvas question is settled — a live region lives in the surrounding page even if
the editing surface is drawn.

### Implementation

| Requirement | Implementation |
|---|---|
| Region | One `aria-live="polite"` region for ordinary status; one `aria-live="assertive"` reserved for failures that stop work (import failed, save failed mid-session) |
| Content | The same words shown visually. Never a separate, terser string — divergence between what is seen and what is heard is its own defect |
| Timing | Announced when the state changes, not when a toast animates |
| Binding | The 23-member committed status-message list in `stories.md`. AC13.1.2: adding a status without a list entry fails the build |

### The three that are easiest to omit

- **Import completed** (AC13.1.3). Import start is announced and completion is
  routinely forgotten, which leaves a screen-reader user waiting through the
  whole budget and then hearing nothing.
- **Fit could not be checked** — the state that exists precisely so silence is
  not mistaken for a pass.
- **Save failed mid-session** (AC8.2.5) — assertive, because work is at risk.

---

## Toast

| Field | Value |
|---|---|
| Component | Toast |
| Description | Transient confirmation of what just happened |
| Category | feedback |

Q5: the toast **names what happened and points at the undo control**. It does
not carry the undo action.

| Requirement | Implementation |
|---|---|
| Duration | Dismisses after 5 seconds; never auto-dismisses a failure |
| ARIA | Content routed through the polite live region rather than the toast being a live region itself, so a dismissed toast does not remove the announcement |
| Motion | Respects `prefers-reduced-motion`; the toast appears without animation when requested |
| Focus | Never takes focus |

---

## Focus and keyboard rules that span components

1. **Visible focus everywhere.** Minimum 2px outline at 3:1 contrast. `outline:
   none` without a replacement is a defect, not a style choice.
2. **Escape is layered, and this list is exhaustive.** Innermost first, one
   layer per press:

   | State | Escape does | Next press |
   |---|---|---|
   | A modal is open (S5, S7, S8) | Closes the modal, focus returns to its trigger | Falls through to whatever is beneath |
   | The `Design` disclosure is open | Closes the popup, focus returns to the trigger, **and consumes the press** — the sheet does not also collapse | Collapses the full sheet to peek |
   | The sheet is at full height | Collapses to peek | Closes the sheet |
   | The sheet is at peek | Closes it, focus returns to the street on the map or in the street list | — |

   **Never two layers in one press.** The disclosure row is stated here rather
   than only in the component, because a rule that claims to enumerate every
   layer and then omits one is worse than no rule: an implementer treats the
   list as authoritative and the component spec as decoration, and the two
   handlers agree by luck instead of by construction.
3. **Focus returns.** Closing any overlay returns focus to what opened it.
4. **No focus theft.** AC2.3.4: when the map view changes and the street list
   updates, focus is not stolen and list position is not reset.
5. **Skip link.** First focusable element on every page, jumping to `main`.
6. **Tab order follows visual order** at every breakpoint, including when the
   bottom sheet reorders content on a phone.

## Motion

All animation under 300 ms, eased. Every animation is suppressed under
`prefers-reduced-motion`, including the bottom sheet's height transition — which
then becomes an instant state change, not a slower one.
