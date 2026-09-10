# Refined Mockups — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `wireframes.md` and `user-flow.md` (rough-mockups),
`stories.md` and `personas.md` (user-stories), `requirements.md`
(requirements-analysis), `team-practices.md` (practices-discovery).

## What is already settled and is not re-asked

Carried forward rather than re-elicited: Option B as the layout (map primary,
cross-section in a drawer) and Option 1 for corridor interaction (edit then
extend); the select-then-act principle and that every drag duplicates a
selectable action; the eight screens S1–S8 and their accessibility annotations;
WCAG 2.1 Level AA including the editing surface (NFR4.1); the 360 CSS pixel
viewport floor (NFR4.4) and the 44 by 44 activation target (NFR4.5); the mobile
browser matrix (NFR4.6); and that whether the editing surface is DOM or canvas
is deferred to Domain Design, so nothing here may assume either.

## Three corrections this stage must make, not decide

`stories.md` records three approved screens that contradict a Must Have
criterion. They are not questions — the stories settled them — but they are work
this stage owns:

- **S4** says "Lanes will be scaled to fit." A scaled width is neither `mapped`,
  `inferred` nor `user-set`, so scaling produces a value with no valid
  provenance state (FR3.1). AC6.2.2 applies the design unchanged and marks the
  street as not fitting. S4's note must change.
- **S7** draws three mutually exclusive visibility radios. FR8.2 and
  AC10.2.1–AC10.2.3 require two independent mechanisms, where enabling a link
  leaves named grants untouched and vice versa. The radio group must be
  redrawn.
- **S8** draws the preview inside the export dialog, but US12.3 (preview) is
  Should Have. S8 needs a variant that works without it.

## What has no screen at all yet

Six stories added at user-stories have no wireframe: US5.5 (undo), US6.3
(choose a corridor), US6.4 (apply across it, with its confirmation and partial
failure), US7.5 (a correction that no longer resolves after re-import), US8.2
(work gone from this device), US8.3 (upload off the device), US13.1 (status
announcements). Most are states of existing screens; two are not, and those are
Q4 and Q7 below.

---

## Q1. How does the drawer work on a 360-pixel phone?

Option B was chosen over A and C specifically for "one layout for every form
factor", and `user-flow.md` records that the audience arrives "frequently on a
phone". No wireframe shows a phone. At 360 CSS pixels the map and a drawer
containing a cross-section, a lane list, a type select and a width stepper
cannot both be usefully visible.

- A. The drawer becomes a bottom sheet with two heights — a peek showing the
  street name and the cross-section, and a full height covering the map for
  editing. The map is never gone, only pushed down.
- B. The drawer becomes a full screen. Selecting a street navigates to the
  editor; a back control returns to the map. Two views, not one.
- C. The map collapses to a thin strip at the top showing only the selected
  street's context, and the drawer takes the rest permanently.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q2. How is an inferred value distinguished from a measured one?

FR3.2 requires the distinction wherever a cross-section is displayed. AC4.1.6
requires it stated in words at least once per view, because P3 has no
street-design vocabulary to decode a visual marking. AC4.1.4 requires it in the
accessibility tree. WCAG forbids colour alone. This is the product's central
credibility mechanism and it needs one answer used everywhere.

- A. A hatch or texture on the lane block, plus the value shown with a
  qualifier — "3.5 m (estimated)" — plus a one-line note per view explaining
  what estimated means.
- B. A distinct outline style on the lane block and an icon beside the value,
  with the explanatory note as above.
- C. No marking on the lane block itself; the distinction lives entirely in the
  lane detail panel and the dimensions table, where the numbers actually are.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q3. Where does the design system come from?

There is none, and `constraint-register.md` LC-3 forbids taking anything from
Streetmix. A Rust/WASM client narrows the options — most component libraries are
JavaScript — but the DOM/canvas question is deferred, so this must not
presuppose either.

- A. Define a minimal token set here — a spacing scale, a type scale, a colour
  palette with contrast ratios stated, and a named set of about ten components —
  and implement them directly. No library dependency.
- B. Adopt a CSS-only framework for layout and controls, with tokens layered on
  top. Less to build; adds a dependency and its licence to the manifest.
- C. Defer entirely to Domain Design, which picks the UI framework.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q4. Where does an unresolved correction live?

AC7.5.4 is the case nothing has ever been drawn for: you corrected a lane's
width, the street was imported again, OpenStreetMap changed underneath, and the
lane your correction attached to no longer resolves. The correction is neither
applied nor discarded — it is retained and surfaced. Somebody has to see it.

- A. An inline banner in the drawer for that street, listing unresolved
  corrections with what each said, and actions to re-apply to a chosen lane or
  discard.
- B. A marker on the street in the map plus a badge in the drawer header,
  opening a panel with the same list. Discoverable without opening the street.
- C. A per-design review view listing every unresolved correction across every
  street, reached from the design rather than the street.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q5. How is undo offered?

US5.5 requires undoing the last editing action, and AC5.5.2 requires a corridor
apply — potentially four streets at once — to undo as a single unit.
`wireframes.md`'s Success state already promises "a brief confirmation naming
what happened and how to undo it".

- A. A toast after every reversible action, carrying the undo control, plus a
  keyboard shortcut. Nothing persistent in the interface.
- B. A persistent undo control in the drawer header, always available, plus the
  keyboard shortcut. The toast confirms but does not carry the action.
- C. Both — toast for immediacy, persistent control for later.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q6. What does the corridor flow look like now it is four stories?

`wireframes.md` S4 draws one screen: a checklist of connected streets with a
width note and an apply button. The stories split that into extending to one
adjacent street (US6.1), choosing a corridor (US6.3), applying across it
(US6.4), and the fit warning (US6.2) — and AC6.4.2 requires a confirmation
naming how many streets received the design, which were warned, and which could
not be checked.

- A. Keep one screen. The checklist gains per-street status — fits, does not
  fit, could not be checked — and the confirmation replaces the checklist in
  place after applying.
- B. Two screens. Choosing the corridor is S4; the outcome is a separate result
  screen listing what happened per street, which is also where undo lives.
- C. Keep one screen for choosing, and put the outcome in the toast from Q5,
  with a link to details only when something was warned or unchecked.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q7. Is there a screen for a design that spans several streets?

**Narrowed by Q8.** The answer recorded here — no overview screen — stands on
the screen question. Q8 settled the storage question separately: device storage
does hold multiple designs, and the map carries the overview role that a
dedicated screen would otherwise have had.

S3 shows one street's cross-section. After a corridor apply the user holds a
design over several streets, and two stories need to see it as a whole: US5.4
(see what I changed, across the three layers) and US8.1.4 (device storage holds
more than one design, and I can reach a specific one without an account). S6 —
"My designs" — is Stage 2 and needs an account.

- A. Yes — a design overview screen in Stage 1, listing the streets in the
  current design with their status, from which any street opens in the drawer.
  It also becomes the Stage 1 home for returning to stored designs.
- B. No new screen. The map itself is the overview: streets in the design are
  highlighted, and a count in the header opens a list. Cheaper, and the map is
  already the primary surface.
- C. No overview in Stage 1 at all. One design at a time on the device; the
  multi-design case waits for accounts in Stage 2.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q8. Follow-up — Q7 contradicts an approved acceptance criterion

Q7 chose no overview in Stage 1: one design at a time on the device, with the
multi-design case waiting for accounts.

**AC8.1.4 says the opposite**, and it is an approved Must Have:

> *Given* I have a stored design, *When* I start work on a second street,
> *Then* the first is not replaced: device storage holds multiple designs, and I
> can reach a specific one again without an account.

It was written because P3's persona is a campaign running over weeks across
several streets, and because US6.4 produces a design spanning several streets in
one action. The inception guardrail forbids carrying an unresolved contradiction
forward, so this needs settling rather than recording.

Note that Q7 and AC8.1.4 are not quite the same question. AC8.1.4 requires
*storage of more than one design* and *a way back to a specific one*. It does
not require an overview screen. So there is a middle answer.

- A. Keep AC8.1.4 and give it the thinnest affordance that satisfies it — the
  map's empty-state prompt (S2) gains a "continue a recent design" entry
  listing what is stored on this device. No new screen, no overview, and the
  criterion holds.
- B. Keep AC8.1.4 and use the map as the overview after all — streets in the
  current design highlighted, a count in the header opening a list. Q7's option
  B, chosen now for the criterion rather than for the overview.
- C. Amend AC8.1.4 — Stage 1 genuinely holds one design at a time, and starting
  a second street replaces the first after an explicit warning. This is a change
  to an approved acceptance criterion and joins the amendments table in
  `stories.md`; it also weakens US6.4, since a corridor design is by definition
  several streets in one design.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Consolidated Summary Confirmation

**Your answers**

- **Phone layout**: the drawer becomes a bottom sheet with two heights — a peek
  showing the street name and cross-section, and a full height covering the map
  for editing. The map is never gone (Q1).
- **Provenance display**: hatch or texture on the lane block, the value written
  with a qualifier — "3.5 m (estimated)" — and a one-line note per view
  explaining what estimated means (Q2).
- **Design system**: a CSS-only framework for layout and controls, with project
  tokens layered on top (Q3).
- **Unresolved corrections**: a marker on the street in the map plus a badge in
  the drawer header, both opening the same list (Q4).
- **Undo**: a persistent control in the drawer header. The toast confirms what
  happened but does not carry the action (Q5).
- **Corridor flow**: one screen. The checklist gains per-street status and the
  confirmation replaces it in place after applying (Q6).
- **Multi-street designs**: no dedicated overview screen (Q7), but device
  storage does hold multiple designs and the map carries the overview role —
  streets in the current design highlighted, a count in the header opening a
  list (Q8).

**Seven things I will draw that follow from your answers rather than being
stated by them**

- **A fourth wireframe correction.** The approved Screen States table says the
  Success state shows "a brief confirmation naming what happened and how to undo
  it". Q5 moved the undo action to a persistent control, so the toast now names
  what happened and points at that control rather than carrying the action. The
  three corrections already recorded in `stories.md` become four.
- **The map becomes a layered surface and needs a legend.** Your answers give it
  four jobs at once: the street network, the current design's streets
  highlighted (Q8), markers for streets with unresolved corrections (Q4), and
  per-street fit status after a corridor apply (Q6). Four overlays without a
  legend is a map nobody can read. The legend is itself an accessibility
  surface — it cannot be colour-only, and it needs a text equivalent.
- **The corridor checklist row gets crowded at 360 pixels.** Q6 puts name,
  length, checkbox and one of four fit states — fits, does not fit, could not be
  checked, not yet checked — on one row. At the phone width that is a two-line
  row, and the fit state needs a word, not just an icon, for the same reason
  provenance does.
- **The peek height must show the undo control.** Q1 gives the sheet a peek
  state and Q5 puts undo in the drawer header. If the header is not in the peek,
  undo disappears exactly when a corridor apply has just changed four streets.
  The header, including undo, is part of the peek.
- **The hatch needs a non-visual twin in three places, not one.** Q2's hatch
  covers the lane block. The same distinction has to reach the accessibility
  tree (AC4.1.4), the words in the detail panel (AC4.1.6), and any produced
  output (AC12.2.3, which requires a non-colour channel in the output's
  structure). One decision, three renderings.
- **Two heights means two keyboard entry points.** The approved S3 annotation
  names the first lane as the keyboard entry point. In peek, the first lane may
  not be reachable without expanding, so the sheet needs a stated focus rule for
  expanding and collapsing, and Escape has to mean collapse-then-close rather
  than close.
- **"Continue a recent design" needs a home.** Q8 puts the design count in the
  map header. Returning to a stored design on a fresh visit happens before any
  design is current, so the S2 empty-state prompt carries it too — otherwise
  AC8.1.4's "reach a specific one again" has no entry point on arrival.

**Two things I will record as open rather than answer**

- **Which CSS framework.** Q3 chose the approach, not the dependency. Naming one
  depends on a decision this stage does not own: whether the editing surface is
  DOM or canvas, and which Rust framework renders it. A classless framework
  suits the "native HTML elements first" accessibility rule; a class-based one
  suits a component library. It also adds a licence row to the manifest
  `scripts/verify.sh` checks. For Domain Design.
- **Whether the map overlays survive at 360 pixels.** Four overlays plus a
  legend on a phone-width map is a real risk, and the honest answer is that it
  needs testing rather than asserting. The mockups will show the intended
  phone treatment; whether it holds is a question for the first build.

Does this all look correct before I generate the mockups, interaction spec,
design system mapping and accessibility checklist?

- Looks correct
- Request changes

[Answer]: Looks correct
