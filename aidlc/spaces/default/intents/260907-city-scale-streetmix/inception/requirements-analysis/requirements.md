# Requirements — Streetmix at City Scale

Upstream inputs: `intent-statement.md` (intent-capture), `scope-document.md`
(scope-definition), `team-practices.md` (practices-discovery).

`FR{n}` and `NFR{n}` identifiers are permanent traceability keys. Downstream
stages preserve them exactly rather than renumbering.

## Intent Analysis

The goal is not "a street editor with a map attached". It is to give someone
without professional tooling a way to produce a street redesign that a city
cannot dismiss.

Three things follow from that framing, and they shape every requirement below:

- **The audience is not the payer or the authority.** Advocates and the public
  are who the product serves; cities are who must accept its output. The product
  succeeds when those two are connected, which is why output fidelity matters
  more than feature count.
- **Credibility is the product.** A design that looks authoritative but rests on
  guessed data is worse than one that shows its workings, because the first
  failure destroys the trust the second was earning. This is why provenance
  appears in the functional requirements rather than as a technical detail.
- **Reach is the constraint on ambition.** One person, roughly $5 a month, and no
  deadline. Requirements that assume operational staff, paid infrastructure, or a
  second pair of eyes are not requirements for this project.

## Functional Requirements

### FR1 — Find and view streets on a real map

- **FR1.1** The system shall present a map of the street network, allowing the
  user to navigate to a location by search or by device location.
- **FR1.2** The system shall allow a street to be selected from the map.
- **FR1.3** The system shall allow a street to be selected without a pointing
  device, through a keyboard-reachable list equivalent to map selection.
- **FR1.4** The system shall show a prompt rather than a bare map when nothing is
  selected.

### FR2 — Import real street geometry

- **FR2.1** The system shall derive a street's cross-section from OpenStreetMap
  data through osm2streets, producing lanes ordered left to right with type,
  direction and width.
- **FR2.2** The system shall load street data in two layers: a coarse pass over
  the visible map area for what is displayed, and a full osm2streets pass on the
  street the user selects.
- **FR2.3** The system shall never write to OpenStreetMap. Imported data is a
  local working copy.

### FR3 — Distinguish what is known from what is guessed

This requirement group exists because OpenStreetMap defines width kerb-to-kerb
and largely does not carry sidewalk widths, so a derived cross-section is part
measurement and part inference.

- **FR3.1** Every lane attribute shall carry a provenance state of exactly one
  of: `mapped` (read from OpenStreetMap), `inferred` (derived by osm2streets
  from other tags or defaults), or `user-set` (corrected or entered by the user).
- **FR3.2** The system shall visually distinguish `inferred` values from `mapped`
  values wherever a cross-section is displayed.
- **FR3.3** The system shall not present an `inferred` value as a measurement in
  any view or export.
- **FR3.4** A user correction to an imported value shall set that attribute's
  provenance to `user-set`, never to `mapped`.

### FR4 — Edit a street's cross-section

- **FR4.1** The system shall allow a lane to be selected, and its type and width
  changed.
- **FR4.2** The system shall allow lanes to be added and removed.
- **FR4.3** Every editing action shall be performable through a select-then-act
  interaction, without requiring a drag.
- **FR4.4** Drag interactions, where offered, shall duplicate an action already
  reachable by selection.
- **FR4.5** The system shall preserve the imported baseline separately from the
  user's edits, so that what changed remains recoverable.

### FR5 — Work across a corridor

- **FR5.1** The system shall allow a design made on one street to be extended to
  connected streets chosen by the user.
- **FR5.2** The system shall warn, before applying, where a target street cannot
  accommodate the design being extended to it.
- **FR5.3** The system shall present connected streets as a selectable set rather
  than requiring each to be visited.

### FR6 — Handle streets that cannot be imported

- **FR6.1** On import failure the system shall explain what happened in terms of
  the street, not the underlying error.
- **FR6.2** On import failure the system shall offer a blank cross-section as a
  starting point, so no street is a dead end.
- **FR6.3** The system shall record import failures where the maintainer can
  observe them, so failure patterns become visible.
- **FR6.4** The system shall allow the user to correct imported data before
  designing on it, subject to FR3.4.

### FR7 — Accounts (Stage 2)

- **FR7.1** The system shall allow a user to create an account and sign in.
- **FR7.2** Sign-in shall not be required to design. A design created before
  sign-in shall survive signing in.
- **FR7.3** The system shall allow a signed-in user to save, name and reopen
  designs.

### FR8 — Sharing (Stage 2)

- **FR8.1** Saved designs shall be private by default.
- **FR8.2** The system shall allow the owner to grant access to named people, or
  to enable a link, as separate explicit choices.
- **FR8.3** The system shall show each design's current sharing state wherever
  designs are listed.

### FR9 — Data subject rights (Stage 2, gating public release)

`scope-document.md` makes these a gate on public availability rather than a
scheduled feature. Stage 2 may be built and used by the owner and invited
testers before they exist; it may not be opened to the public until they do.

- **FR9.1** The system shall allow an account holder to delete their account,
  removing their personal data and their designs.
- **FR9.2** The system shall allow an account holder to export their designs in a
  usable, machine-readable format.
- **FR9.3** Anonymous designs shall expire 30 days after creation.
- **FR9.4** Account data shall be retained until the account is deleted.

### FR10 — Meeting-ready output (Stage 3)

- **FR10.1** The system shall produce output suitable for a public meeting, in
  forms chosen by purpose rather than by file format.
- **FR10.2** Output shall preserve the provenance distinction required by FR3.3.
- **FR10.3** The system shall preview output before it is produced.

### FR11 — Entry and identity

- **FR11.1** The system shall present a landing page explaining what it is, with
  a worked example and a route into the tool.
- **FR11.2** The system shall be usable without a tutorial or first-run wizard.

## Non-Functional Requirements

### NFR1 — Performance

- **NFR1.1** A full osm2streets import of a single selected street shall complete
  within 10 seconds.
- **NFR1.2** An editing action shall produce visible feedback within 100
  milliseconds.
- **NFR1.3** The coarse viewport pass (FR2.2) is measured separately from
  NFR1.1 and has no stated budget yet.

### NFR2 — Scale

- **NFR2.1** The system shall support working across a city-scale street network
  of thousands of streets, with only the visible portion loaded at any time.
- **NFR2.2** Memory use shall remain within a browser tab's practical working set
  at that scale. No figure is set; this is a constraint on design, not a testable
  threshold yet.

### NFR3 — Availability

- **NFR3.1** The service shall target 99.5% monthly availability or better.
- **NFR3.2** Deployments shall not count against NFR3.1: a new version shall
  become active only after passing a health check, with the previous version
  serving until then.
- **NFR3.3** The application service shall hold no attached storage volume.
  Persistent state lives in a separate database service. This is what makes
  NFR3.2 achievable — a service with an attached volume takes downtime on every
  redeploy.

### NFR4 — Accessibility

- **NFR4.1** The system shall conform to WCAG 2.1 Level AA, including the editing
  surface.
- **NFR4.2** Every editing action shall be operable by keyboard alone.
- **NFR4.3** Lane elements shall be individually focusable and announceable to a
  screen reader.
- **NFR4.4** The system shall support viewing and editing at viewport widths from
  360 CSS pixels upward. Below 360 CSS pixels is unsupported and need not be
  tested.
- **NFR4.5** Every interactive editing control, including an individual lane
  element, shall present an activation target of at least 44 by 44 CSS pixels.
  Where a lane's true scaled width renders narrower than that, the activation
  target shall be enlarged beyond the lane's drawn extent; the drawing shall not
  be widened to meet the target, because a lane's rendered width is data and
  widening it would misrepresent the street. This holds whichever way OQ2 is
  settled: on real page elements the target is the element's own box, and on a
  canvas it is the hit-test region, but the 44-pixel floor and the
  do-not-widen-the-drawing rule apply to both.
- **NFR4.6** The mobile pass/fail matrix is the current and the previous major
  version of Safari on iOS and of Chrome on Android. A mobile browser outside
  that matrix need not be tested.
- **NFR4.7** Desktop support is assumed via current evergreen browsers. NFR4.6
  scopes the mobile matrix only and does not narrow desktop support. No desktop
  matrix has been established; OQ6 carries that gap.

Note on the 44-pixel figure: it is a project commitment, not a consequence of
NFR4.1. Target size is Level AAA in WCAG 2.1 (SC 2.5.5 Target Size) and reaches
Level AA only in WCAG 2.2 (SC 2.5.8 Target Size (Minimum), at 24 CSS pixels), so
conformance to WCAG 2.1 Level AA would not by itself require it. It is set here
because the editing surface is the product, and a missed tap on a phone is a
wrong edit rather than a retry.

### NFR5 — Cost

- **NFR5.1** Hosting and tooling shall remain within approximately $5 per month.
- **NFR5.2** Per-user computation shall run in the user's browser rather than on
  the server, so that cost does not scale with usage.

### NFR6 — Security and privacy

- **NFR6.1** No credential, connection string or secret shall be present in the
  repository.
- **NFR6.2** Shared designs shall be inaccessible to anyone not explicitly
  granted access by the owner.
- **NFR6.3** No payment data and no special-category personal data shall be
  collected or stored.

### NFR7 — Maintainability and verification

- **NFR7.1** Line coverage shall not fall below 80% over the street and lane
  domain model, the provenance model, the osm2streets adapter, the persistence
  layer, and the editing state machine.
- **NFR7.2** The osm2streets dependency shall be pinned to a specific commit,
  including its transitive `abstutil` dependency.
- **NFR7.3** Every defect shall have a failing regression test reproducing it
  before the fix lands.

## Product Goals — Not Requirements

Recorded deliberately outside the requirement sets, because they are not testable
as stated and `team-practices.md` carries an explicit warning against promoting
this one into an acceptance criterion.

- **G1** A corridor redesign that takes a planner about a week of work today
  should take under a day in this tool. No baseline has been measured and no user
  has been timed. Revisit once a real user can be observed; do not treat as a
  pass/fail criterion until then.

## Constraints

Carried from `constraint-register.md` and `team-practices.md`, not re-derived.

| Constraint | Consequence |
|-----------|-------------|
| The lane schema comes from osm2streets | The street model is shaped by a dependency, not chosen freely |
| The client is Rust compiled to WebAssembly | Toolchain, testing and layer boundaries are Rust's |
| One person builds this, with AI assistance | No parallel work, no specialist cover, no second reviewer |
| Approximately $5 per month | Anything above it is a change to the constraint, not an overrun |
| Open source and free | No revenue to fund infrastructure growth |
| Streetmix is not a code dependency | Its code and assets must not be copied in |
| Jurisdiction-neutral core | No country's standards baked into the model |
| The repository is public from the first commit | Secrets leak publicly, not internally |

## Out of Scope

- Traffic simulation and modelling.
- Construction- or engineering-grade output.
- Editing OpenStreetMap data.
- Pluggable jurisdiction standards and city GIS import/export — both Should Have,
  after the three stages, and neither excluded.

## Assumptions & Open Questions

### Assumptions

- **A1** osm2streets' schema is sufficient for the editing this product needs.
  Established from its documentation, not from running it. B-0 tests this.
- **A2** 99.5% availability is achievable on a single environment with no
  failover. Zero-downtime deploys make it plausible; a platform incident has no
  fallback.
- **A3** A city-scale network can be held in a browser tab at acceptable memory
  cost. Untested.
- **A4** Erasure and export are a proportionate set of data subject rights for a
  product holding street designs rather than sensitive personal data. This is a
  judgment, not legal advice.
- **A5** 30 days is an appropriate expiry for anonymous designs. Chosen, not
  derived from observed behaviour.
- **A6** The audience of advocates and the public is reachable without
  institutional distribution. Carried unvalidated from Ideation.
- **A7** The 360 CSS pixel floor (NFR4.4) and the mobile browser matrix (NFR4.6)
  are chosen, not measured against observed usage — there is no analytics and no
  user base yet. 360 is the narrowest width in common current Android devices, so
  it covers the iPhone baseline of 375 as well; Safari on iOS and Chrome on
  Android are chosen because between them they carry almost all mobile browsing.
  Revisit once real usage exists.

### Open Questions

- **OQ1** Where OpenStreetMap data is fetched from is unsettled, and it has a
  direct cost consequence: routed through the Railway service, egress bills at
  $0.05 per GB against a $5 monthly budget; fetched by the browser directly from
  an OpenStreetMap API, it costs nothing. At the city scale set by NFR2.1 this is
  the difference between a negligible and a budget-breaking bill. Domain Design or
  Infrastructure Design must settle it.
- **OQ2** Whether the editing surface is built from real page elements or drawn on
  a canvas is deferred to Domain Design. The NFR4 group is verifiable
  automatically in the first case and only by hand in the second.
- **OQ3** No budget is set for the coarse viewport pass (NFR1.3), and none can be
  until the layered loading model is built.
- **OQ4** No memory figure supports NFR2.2.
- **OQ5** What form meeting-ready output must take for a city to accept it
  (FR10.1) has not been established with any city.
- **OQ6** No desktop browser matrix is established (NFR4.7). Desktop is supported
  and assumed to be evergreen, but nothing states which browsers the
  keyboard/accessibility CI tier in `team-practices.md` actually runs against.
  Settle it before that test matrix is written.

## Revision History

- **Revision 1** — R-01: NFR4.4 given a measurable threshold rather than being
  demoted to an open question; NFR4.5 added for activation targets. R-02: the
  trailing pointer section removed, leaving one `## Assumptions & Open Questions`
  heading holding both subsections.
- **Revision 2** — R-03: NFR4.4's exclusion clause had unsupported every browser
  outside the phone matrix, desktop included. NFR4.4 now bounds only the viewport
  floor; the mobile matrix moved to NFR4.6, and NFR4.7 states the desktop
  position explicitly. R-04: assumption A7 records 360 CSS pixels and the mobile
  matrix as chosen rather than measured. R-05: OQ2 now names the NFR4 group
  instead of an enumeration that goes stale. OQ6 added for the missing desktop
  browser matrix.

**On the 44-pixel figure and WCAG.** The iteration-1 review's required action
cited "WCAG 2.5.5/2.5.8" as though NFR4.1's WCAG 2.1 Level AA commitment already
fixed a target size. It does not: SC 2.5.5 Target Size is Level **AAA** in
WCAG 2.1, and the Level AA criterion — SC 2.5.8 Target Size (Minimum), at 24 CSS
pixels — is introduced in WCAG 2.2, which this project has not committed to. The
44-pixel figure in NFR4.5 is therefore a project commitment beyond NFR4.1, and is
recorded as one. The reviewer verified and confirmed this at revision 1. Do not
"correct" it back to a WCAG 2.1 AA consequence.

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-09T02:50:57Z
**Iteration:** 1
**Request Challenge:** review:e2d6db5ea219ff4022411059bf7cc848

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `aidlc/spaces/default/intents/260907-city-scale-streetmix/inception/requirements-analysis/requirements.md` > NFR4.4, NFR4.5 | Closed at revision 1 and still holding at revision 2. NFR4.4 carries a measurable viewport floor (360 CSS pixels) and NFR4.5 carries a testable activation-target rule (44 by 44 CSS pixels, with the narrow-lane case resolved in favour of the data rather than the drawing). NFR4.5 kept its ID through the revision-2 edits — the new requirements were appended as NFR4.6 and NFR4.7 rather than renumbering it — so nothing that pointed at it went stale. | None — closed. | Resolved |
| R-02 | Minor | `aidlc/spaces/default/intents/260907-city-scale-streetmix/inception/requirements-analysis/requirements.md` > `## Assumptions & Open Questions` | Closed at revision 1 and still holding. One `## Assumptions & Open Questions` heading with `### Assumptions` and `### Open Questions` beneath it; the revision-2 additions (A7, OQ6) went into those subsections rather than creating a third heading. | None — closed. | Resolved |
| R-03 | Major | `aidlc/spaces/default/intents/260907-city-scale-streetmix/inception/requirements-analysis/requirements.md` > NFR4.4, NFR4.6, NFR4.7 | The required action is met, and met the way it was specified rather than by softening the sentence. NFR4.4 now bounds one thing only — the viewport floor — and its exclusion clause reads "Below 360 CSS pixels is unsupported and need not be tested", which cannot be read as a browser statement. The mobile matrix moved intact to NFR4.6, whose own exclusion is explicitly scoped ("A mobile browser outside that matrix need not be tested"). NFR4.7 states the desktop position in the affirmative and pre-empts the misreading directly ("NFR4.6 scopes the mobile matrix only and does not narrow desktop support"). I checked the three for a new contradiction and found none: each exclusion now names the axis it governs, and no sentence in the group excludes anything the other two claim. Desktop is no longer ambiguous about whether it is supported — it is stated as supported; what remains open is which desktop browsers form the test matrix, and NFR4.7 hands that to OQ6 by name rather than leaving it implicit. | None — closed. | Resolved |
| R-04 | Minor | `aidlc/spaces/default/intents/260907-city-scale-streetmix/inception/requirements-analysis/requirements.md` > `### Assumptions` > A7 | The required action is met with real content, not a restatement. A7 records both figures as chosen rather than measured, names why there is no measurement ("there is no analytics and no user base yet"), and gives the one-line reason for each choice — 360 as the narrowest width in common current Android devices, which therefore also covers the iPhone baseline of 375, and Safari on iOS plus Chrome on Android as the pair that between them carry almost all mobile browsing. That is more than A5 gives the 30-day figure and enough for a later stage to know what evidence would justify changing either number. It also correctly cites NFR4.6 for the matrix rather than NFR4.4, so it did not inherit the pre-revision pointer. | None — closed. | Resolved |
| R-05 | Minor | `aidlc/spaces/default/intents/260907-city-scale-streetmix/inception/requirements-analysis/requirements.md` > `### Open Questions` > OQ2 | The required action is met, and by the more durable of the two options offered: OQ2 now reads "The NFR4 group is verifiable automatically in the first case and only by hand in the second", so it cannot go stale again when an accessibility requirement is added. I searched the whole document for remaining `NFR4` references and found no other stale one — the 44-pixel note cites NFR4.1 and NFR4.5 correctly, A7 cites NFR4.4 and NFR4.6 correctly, OQ6 cites NFR4.7 correctly, and neither the Constraints table nor the Intent Analysis references an NFR by ID at all. | None — closed. | Resolved |

### Summary

All five findings are closed on inspection. The revision-2 repair of R-03 is the substantive one and it holds: splitting the single over-broad sentence into a viewport floor (NFR4.4), a mobile matrix (NFR4.6) and an explicit desktop position (NFR4.7) leaves each exclusion scoped to the axis it governs, with no new contradiction between them and no silent unsupporting of desktop. A7 and OQ6 are genuine additions carrying reasons and a named next step rather than restatements of the requirements they annotate. I checked the ID scheme across the whole document: NFR4.1-NFR4.7, A1-A7 and OQ1-OQ6 each run without a duplicate or a gap, NFR4.5 kept its ID rather than being renumbered when 4.6 and 4.7 were appended, and every FR ID is untouched. One thing engineering should know rather than be blocked by: NFR4.7 states desktop support as an assumption ("assumed via current evergreen browsers") rather than as a pass/fail line, which is honest given OQ6 — but OQ6 must be settled before the keyboard/accessibility CI tier in `team-practices.md` gets its browser matrix written, or that tier will be built against an unstated target.
