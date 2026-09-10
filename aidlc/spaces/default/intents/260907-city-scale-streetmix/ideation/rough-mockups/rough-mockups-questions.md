# Rough Mockups — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `intent-statement.md` (intent-capture); `scope-document.md` and
`intent-backlog.md` (scope-definition).

---

## Q1. What should the wireframes cover?

`scope-document.md` stages the work: stage 1 is map view, street import,
cross-section editor and corridor editing; stage 2 adds accounts and sharing;
stage 3 adds meeting-ready output. Wireframing beyond stage 1 is cheap now and
expensive to get wrong later, but it also designs things that may change once
stage 1 meets real use.

- A. Stage 1 only — the design surface, nothing else.
- B. Stage 1, plus the shape of stage 3's output — the differentiator affects what
  the editor must capture, so sketching it early avoids designing into a corner.
- C. All three stages — sketch the whole product now.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q2. How do the map and the cross-section share the screen?

This is the central layout problem. Streetmix shows one cross-section filling the
window; your problem statement requires that cross-section to live on a real map,
and a map wants space too. Every other screen decision follows from this one.

- A. Split screen — map on one side, cross-section on the other, both always
  visible.
- B. Map primary — the map fills the view; selecting a street opens the
  cross-section in a panel or drawer over it.
- C. Cross-section primary — the editor fills the view as in Streetmix, with the
  map as a small inset for context and navigation.
- D. Not yet defined — sketch more than one and compare.
- X. Other (please specify)

[Answer]: D

## Q3. How does a user go from editing one street to editing a corridor?

Corridor and network editing (C4) is the capability that distinguishes this from
Streetmix, and it has no precedent in the incumbent to borrow from. This is about
what the user does, not how it is built.

- A. Select a route — the user draws or picks a path through the network, then
  edits its streets in sequence.
- B. Edit and extend — the user edits one street, then pulls the design along to
  neighbouring streets from there.
- C. Select an area — the user selects a region of the map and edits every street
  in it.
- D. Not yet defined — this needs sketching before it can be chosen.
- X. Other (please specify)

[Answer]: D

## Q4. Which devices and form factors must be supported?

Your audience is advocates and the general public (market research Q6), who reach
things on phones. But cross-section editing is a spatial, drag-heavy task that
wants width.

- A. Desktop first, mobile viewing only — designing needs a large screen; phones
  can view and share.
- B. Desktop and tablet for editing, mobile for viewing.
- C. Fully responsive — editing must work on a phone too.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q5. What accessibility level applies, and how far does it reach into the editor?

WCAG 2.1 AA is the usual target and is legally required in many jurisdictions. A
drag-and-drop spatial editor is the hardest case for keyboard and screen-reader
access, so this needs deciding rather than assuming.

- A. WCAG 2.1 AA throughout, including a keyboard-operable path for every editing
  action.
- B. WCAG 2.1 AA for everything except the editing canvas, which gets a documented
  exception and an accessible alternative view of the same data.
- C. Best effort — follow the guidelines where practical without committing to a
  conformance level.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q6. What is the visual and interaction starting point?

`constraint-register.md` LC-3 is relevant here: Streetmix is AGPL and is not a
code dependency of this project. Its interaction design may inform yours; its
artwork must not be reused without separate permission. So the segment artwork in
particular has to come from somewhere else.

- A. An off-the-shelf design system — adopt an existing open component library and
  build the street-specific pieces on top.
- B. Minimal and plain — no design system, just clear layout, until there is a
  reason for more.
- C. Distinctive from the start — the visual identity is part of what makes a
  design credible, so it deserves attention early.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q7. What does a first-time visitor land on?

The first screen decides whether someone stays. Your audience is advocates and
the public, who arrive without training and often without a specific street in
mind.

- A. Straight into the map — find your street and start; no preamble.
- B. A prompt to choose a location — search or "use my location" first, then the
  map.
- C. An example design — land on a worked example of a redesigned street, with an
  obvious way to start your own.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: X. a hero page

## Consolidated Summary Confirmation

Your answers:

- Coverage: wireframe all three stages (Q1 C).
- Map and cross-section layout: not decided — sketch more than one and compare
  (Q2 D).
- Corridor interaction: not decided — needs sketching before it can be chosen
  (Q3 D).
- Devices: fully responsive; editing must work on a phone (Q4 C).
- Accessibility: WCAG 2.1 AA throughout, including a keyboard-operable path for
  every editing action (Q5 A).
- Visual starting point: distinctive from the start (Q6 C).
- First-time landing: a hero page (Q7 X).

How I am reading two of these, so you can correct me:

- "A hero page" means an unauthenticated marketing-style landing page with a hero
  section and a call to action into the tool, rather than the tool's own first
  screen.
- Q2 and Q3 answering "sketch first" are instructions to produce alternatives,
  not unanswered questions. The wireframes will present options with a
  recommendation rather than a single design.

One finding worth stating before I draw:

Fully responsive editing including phones (Q4) and a keyboard path for every
editing action (Q5) push toward the same solution rather than compounding into
two separate costs. Both rule out drag as the only way to change a lane. A
select-then-act interaction — pick a lane, then choose what it becomes and how
wide — works with touch on a small screen, with a keyboard, and with a screen
reader. Drag can remain as an accelerator on top for people using a mouse. The
wireframes are built on that.

What the artifacts will contain:

- Three layout options for the map and cross-section, drawn and compared against
  your constraints, with a recommendation you can overturn.
- Two corridor-interaction options, drawn and compared.
- Screens for all three stages, including the hero landing page.
- Empty, loading, error and partial states, not only the happy path.
- A one-line accessibility note per screen: heading level, landmark regions,
  keyboard entry point.
- The core user flow from landing to a shared, exported design.

Two gaps I will record rather than silently fill:

- A hero landing page is not in `intent-backlog.md`, which begins at the map
  view. It needs a proto-Unit if it is in the first release.
- "Distinctive from the start" implies visual identity work that is not in the
  backlog either, and competes for a solo builder's time with stage 1 capability
  work.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
