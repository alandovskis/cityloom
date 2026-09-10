# User Stories — Streetmix at City Scale

Upstream inputs: `requirements.md` (requirements-analysis), `personas.md` and
`user-stories-questions.md` (this stage), `team-practices.md`
(practices-discovery), `scope-document.md`, `wireframes.md` and `user-flow.md`
(ideation). The three contributions under `contributions/` are this stage's
ensemble evidence and record what the design, developer and quality reviews
found against the Round 0 draft, including the positions that were maintained
rather than folded in.

`US{group}.{seq}` and `AC{group}.{seq}.{n}` identifiers are permanent
traceability keys. Downstream stages preserve them exactly rather than
renumbering.

## How these are organised

Thirteen groups, in the order a person moves through the product: arrive, find a
street, import it, understand what was imported, edit it, extend it, recover when
it goes wrong, keep it, then accounts, sharing, rights and output — with a final
group for commitments that cut across every screen. Each story is a vertical
slice.

**MoSCoW has three Must Have classes, not one.** The draft used a single
persona-anchored rule and then overrode it three times without saying so. Stated
properly:

| Class | Rule | Examples |
|---|---|---|
| **Persona** | P3 cannot produce a credible proposal without it | US3.1, US5.1, US6.1 |
| **Access** | Any user needs it to reach, judge, or keep using the product at all — including deciding whether it is worth their time before investing any | US1.1, US1.2, US8.1 |
| **Policy** | An approved NFR or a standing constraint makes it Must regardless of persona. The qualifying set is: any NFR in `requirements.md` stated as a commitment rather than a target, and the RC-1/RC-2 public-release gate | US2.3, US5.3, US10.1, US11.1, US11.3, US13.1, US13.2 |

Delivery Planning uses these to settle the next marginal story. A story that
fits no class is not Must Have.

US1.1 is Access rather than Persona because the reader has not yet decided to
use the product: a landing page that does not show real output is a page a
first-time visitor leaves. Reaching the tool and judging whether to reach it are
the same gate.

**Stage** refers to `scope-document.md`'s three-stage split, not to a sprint.

**Falsifiable from** appears on criteria that cannot go red in the Bolt that
implements their story, because the thing they assert about does not exist yet.
They are regression guards, not evidence, and must not be counted as coverage
before that stage.

## Three committed lists

Several criteria are bound to an enumerated set rather than to a universal
quantifier, because "every view" and "any editing action" cannot be closed and
cannot be tested. Each list is seeded here and grows with the stories that add
to it. **Adding a member without its test fails the build; adding one without
adding it here is the same defect.** The stage that introduces a new member owns
adding it.

**Editing actions** — bound by AC5.3.1, AC5.3.2, AC8.2.6, AC13.2.1. Seeded from
`team-practices.md`'s merge gate:

1. select a lane · 2. change a lane's type · 3. change a lane's width ·
4. add a lane · 5. remove a lane · 6. extend along the corridor · 7. undo

**Views that display a cross-section** — bound by AC4.1.2, AC4.1.6, AC4.2.1:

1. the editing surface · 2. the corridor comparison in US5.4 ·
3. the connected-street set in US6.3, where a target's cross-section is shown ·
4. the reloaded-design view in US8.1 · 5. the design list in US9.3, where a
preview is shown · 6. every produced output in US12

**Status messages** — bound by AC13.1.1, AC13.1.2:

1. search returned no result (AC2.1.4) · 2. import started (AC3.1.3) ·
3. import completed · 4. import failed (AC7.1.1) · 5. coarse pass empty or
failed (AC3.2.4) · 6. edit rejected as out of range (AC5.1.5) · 7. undo applied
(AC5.5.4) · 8. extension applied to one street (AC6.1.5) · 9. lanes the design
could not be matched onto (AC6.1.3) · 10. fit warning (AC6.2.1) · 11. fit could
not be checked (AC6.2.3, AC6.2.4) · 12. corridor apply completed (AC6.4.2) ·
13. corridor apply partially failed (AC6.4.3) · 14. correction re-applied after a
changed import (AC7.5.3) · 15. correction unresolved (AC7.5.4) · 16. design not
on this device (AC8.2.1) · 17. storage unavailable (AC8.2.4) · 18. save failed
mid-session (AC8.2.5) · 19. upload succeeded (AC8.3.1) · 20. upload failed
(AC8.3.3) · 21. migration failed (AC9.2.3) · 22. duplicate design name
(AC9.3.3) · 23. output failed (AC12.1.4)

---

## US1 — Arrive and understand what this is

### US1.1 — Understand the tool from a worked example

**As** Marcus (P3), **I want** to see what this tool produces before I commit
any effort to it, **so that** I can tell in under a minute whether it is worth
my time.

- **Priority**: Must Have (Access) · **Stage**: 1 · **Traces**: FR11.1, NFR4.4
- **INVEST**: Independent of every editor story — it is a page. Small.
- **Depends on**: nothing.

- **AC1.1.1** — *Given* I arrive at the site for the first time, *When* the
  landing page loads, *Then* I see a worked example of a real street redesign
  and a single route into the tool.
- **AC1.1.2** — *Given* I am on the landing page, *When* I read the worked
  example, *Then* it shows the same street before and after, with at least one
  dimension visible, so the example demonstrates the product's actual output
  rather than a stylised illustration.
- **AC1.1.3** — *Given* I am on the landing page at a viewport 360 CSS pixels
  wide, *When* the page renders, *Then* all content is reachable without
  horizontal scrolling of the page body.
- **AC1.1.4** — *Given* I navigate the landing page by keyboard alone, *When* I
  tab through it, *Then* the route into the tool is reachable and its focus
  state is visible.

### US1.2 — Start designing without being taught first

**As** Marcus (P3), **I want** to begin without a tutorial, a wizard or an
account, **so that** the cost of finding out whether this works for me is
approximately zero.

- **Priority**: Must Have (Access) · **Stage**: 1 · **Traces**: FR11.2, FR1.4,
  FR7.2
- **INVEST**: Valuable on its own — the difference between a trial and a bounce.
- **Depends on**: US2.1.

- **AC1.2.1** — *Given* I follow the route from the landing page, *When* the
  tool opens, *Then* I am in the working surface with no tutorial overlay, no
  first-run wizard and no sign-in prompt blocking it.
- **AC1.2.2** — *Given* I am in the tool and no street is selected — on first
  arrival, or after releasing a selection later in the session — *When* the
  working surface renders, *Then* I see a prompt telling me what to do next
  rather than a bare map.
- **AC1.2.3** — *Given* I am using the tool without an account, *When* I make an
  edit, *Then* nothing asks me to sign in to continue.
  **Falsifiable from Stage 2** — there is no sign-in in Stage 1 for this to
  catch.

---

## US2 — Find a street on a real map

### US2.1 — Navigate to a place

**As** Marcus (P3), **I want** to get to the street I care about by searching
for it or using where I am, **so that** I am not panning a map hunting for my
own neighbourhood.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR1.1
- **INVEST**: Small and independently demonstrable against a real basemap.
- **Depends on**: nothing.

- **AC2.1.1** — *Given* I am in the tool, *When* I search for a place by name,
  *Then* the map moves to that place.
- **AC2.1.2** — *Given* I am in the tool on a device that can report location,
  *When* I ask to use my location and grant permission, *Then* the map moves to
  where I am.
- **AC2.1.3** — *Given* I ask to use my location, *When* I refuse the browser's
  permission prompt, *Then* the tool continues working with search only and does
  not ask again in the same session.
- **AC2.1.4** — *Given* I search for a place that returns no result, *When* the
  search completes, *Then* I am told nothing matched, offered the suggestion to
  try a broader place name, and the map stays where it was.

### US2.2 — Select a street

**As** Marcus (P3), **I want** to pick a specific street from the map, **so
that** I can start working on the one I mean rather than an area.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR1.2
- **INVEST**: Small. Testable through the selected-street assertion.
- **Depends on**: US2.1.

- **AC2.2.1** — *Given* the map shows a street network, *When* I select a
  street, *Then* that street is shown as selected and is identified by name
  where one exists.
- **AC2.2.2** — *Given* a street has no name in the source data, *When* I select
  it, *Then* it is identified by something stable and human-readable rather than
  by a blank label.
- **AC2.2.3** — *Given* a street is selected, *When* I select a different one,
  *Then* the previous selection is released and only one street is selected.
- **AC2.2.4** — *Given* I select a street, *When* the selection resolves, *Then*
  it yields a stable OpenStreetMap identity — a way id together with the
  direction-normalised pair of bounding node ids — sufficient to key an overlay
  and to re-resolve the same segment on a later visit. Every stored edit in the
  product depends on this; the coarse pass (US3.2) is what must supply it.

### US2.3 — Select a street without a pointing device

**As** someone using a keyboard or a screen reader, **I want** to reach and
select a street without pointing at a map, **so that** the product is usable at
all rather than usable if I can use a mouse.

- **Priority**: Must Have (Policy — NFR4.1) · **Stage**: 1 · **Traces**: FR1.3,
  NFR4.2, NFR4.3
- **INVEST**: Genuinely independent — a second, equivalent path to the same
  state, which is why it is a story and not a criterion buried in US2.2.
- **Depends on**: US2.2.

- **AC2.3.1** — *Given* the map shows a street network, *When* I navigate by
  keyboard alone, *Then* I reach a list of the streets in view and can select
  one from it.
- **AC2.3.2** — *Given* I select a street from that list, *When* the selection
  completes, *Then* the resulting state is identical to selecting the same
  street on the map — same street identity, same selected state, same imported
  cross-section.
- **AC2.3.3** — *Given* I am using a screen reader, *When* focus moves to a
  street in the list, *Then* its name and its selected state are announced.
- **AC2.3.4** — *Given* the map view changes, *When* the list updates, *Then*
  focus is not stolen and my position in the list is not silently reset.

---

## US3 — Import a real street

### US3.1 — Turn a real street into an editable cross-section

**As** Marcus (P3), **I want** the street I selected to become an editable
cross-section built from real data, **so that** I am changing a real street
rather than drawing an imaginary one.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR2.1, NFR5.2
- **INVEST**: The walking skeleton's core and the highest-risk story in the set.
- **Depends on**: US2.2.

- **AC3.1.1** — *Given* I have selected a street, *When* the import completes,
  *Then* I see its lanes ordered left to right, each with a type, a direction
  and a width.
- **AC3.1.2** — *Given* the committed well-tagged fixture, *When* the adapter
  converts it on the CI runner, *Then* the conversion completes within a
  recorded time, and that time is **reported, not gated**, until
  `nfr-requirements` sets a threshold against real measurement.
  NFR1.1's 10-second user-facing budget is not asserted here: no device, no
  network and no reference street are established, and OQ1 leaves the OSM fetch
  — plausibly the dominant term — unplaced. Deferred to `nfr-requirements` with
  those conditions attached, following the same discipline `team-practices.md`
  applies to branch coverage and to the "week to under a day" claim.
- **AC3.1.3** — *Given* I select a street, *When* the selection is registered,
  *Then* the cross-section panel opens immediately showing the street's name and
  a skeleton cross-section, before any data arrives.
- **AC3.1.4** — *Given* the committed well-tagged fixture, *When* the adapter
  processes it, *Then* the resulting lane count and order match the fixture
  exactly. The fixture records the source extract, the complete `MapConfig`
  (including `country_code`, `driving_side`, `inferred_sidewalks` and
  `inferred_kerbs`) and the pinned osm2streets revision; a change to any of them
  is a fixture change, not a test failure.
- **AC3.1.5** — *Given* a committed raw OpenStreetMap extract, *When* the pinned
  osm2streets crate processes it in the release-mode build, *Then* its output
  matches the committed fixture for that extract. This runs in the slow CI tier
  where the release-mode WASM build already lives, and it is the only assertion
  in the set that goes red the day the dependency moves — which
  `team-practices.md` states is the fixture suite's entire purpose, and which
  B-0's exit is required to confirm.
- **AC3.1.6** — *Given* an import that has not completed within the budget
  `nfr-requirements` sets, *When* that budget elapses, *Then* the import is
  abandoned and treated as a failure, routing into US7.1 and US7.2. An
  indefinite spinner is not an acceptable outcome.
  **Falsifiable from `nfr-requirements`** — no budget value exists yet, so the
  timeout has no threshold to breach. The routing half is testable today with a
  provisional value; the criterion closes when the real figure lands.
- **AC3.1.7** — *Given* a full import, *When* it runs, *Then* no request is made
  to the application's own server. This is NFR5.2's client-side commitment made
  observable, and it is also what would catch OQ1's budget-breaking case — an
  OpenStreetMap fetch proxied through Railway at $0.05 per GB.

### US3.2 — See the surrounding network without importing all of it

**As** Marcus (P3), **I want** the map to show the street network around me
without processing every street, **so that** I can move around a city rather
than one block.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR2.2, NFR2.1
- **INVEST**: Independent of US3.1 — a different data path with a different
  budget, which is exactly why it is separated. It also supplies US2.2.4's
  street identity.
- **Depends on**: US2.1.

- **AC3.2.1** — *Given* a viewport covering at least 2,000 streets — the
  committed city-scale fixture area, which is what NFR2.1's "thousands of
  streets" means operationally — *When* the map renders, *Then* the street
  network is shown from the coarse pass without a full import of any street in
  view.
- **AC3.2.2** — *Given* I pan or zoom the map, *When* new area comes into view,
  *Then* the coarse pass covers it and only the street I select receives the
  full cross-section pass.
- **AC3.2.3** — *Given* N streets have been fully imported, *When* the viewport
  moves so that M of them are out of view, *Then* the count of retained
  full-import records is N − M. NFR2.2 sets no memory figure and states that it
  is a constraint on design rather than a testable threshold; this asserts the
  eviction behaviour instead, which is deterministic and needs no figure.
- **AC3.2.4** — *Given* an area with no mapped streets, or a coarse pass that
  fails, *When* it returns, *Then* I am told there is nothing to work with here
  and the tool remains usable.

---

## US4 — Tell a measurement from a guess

### US4.1 — See which values are measured and which are inferred

**As** Priya (P2), **I want** to see at a glance which dimensions came from the
map and which the tool worked out, **so that** I can tell how much weight the
proposal's numbers carry.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR3.1, FR3.2
- **INVEST**: Valuable to P2 above all; testable at the model level and in the
  accessibility tree.
- **Depends on**: US3.1.
- **Implementation note for the adapter's author**: in osm2streets, `LaneSpec`'s
  width is the parsed tag value where one exists and
  `typical_lane_widths(lt, highway_tag)` otherwise, and `LaneSpec.lane:
  Option<Lane>` is the only field carrying that distinction. That is where the
  `mapped` / `inferred` mapping is read from. Whether the parsed width is itself
  sometimes derived needs checking at implementation time rather than assuming.

- **AC4.1.1** — *Given* an imported street, *When* I look at any lane attribute,
  *Then* it carries exactly one provenance state of `mapped`, `inferred` or
  `user-set`.
- **AC4.1.2** — *Given* a cross-section is displayed in any view on the
  committed view list, *When* it contains both mapped and inferred values,
  *Then* the two are visually distinguished. The view list is committed
  alongside the stories; adding a view without a test fails the build.
- **AC4.1.3** — *Given* the committed thinly-tagged fixture, pinned to its
  `MapConfig` and osm2streets revision as in AC3.1.4, *When* the adapter
  processes it, *Then* at least one attribute comes back `inferred`.
- **AC4.1.4** — *Given* the committed thinly-tagged fixture, which yields an
  `inferred` width, *When* focus reaches that lane, *Then* the inferred state is
  present in its announced accessible name or state, and absent from the
  announced state of a lane whose width is `mapped`. This is executable under
  DOM and under any canvas that exposes an accessibility tree, and needs no
  pixel comparison. If Domain Design selects canvas without an accessibility
  tree, this fails — and so does NFR4.3 independently, which is the right
  outcome rather than a surprise at release.
- **AC4.1.5** — *Given* an imported cross-section, *When* I navigate it by
  keyboard alone, *Then* each lane is individually reachable by focus, in the
  same left-to-right order in which it is displayed. Announcement is asserted by
  AC4.1.4 and AC5.3.3; this asserts the focusability half of NFR4.3, which those
  criteria presuppose and which is the half a canvas editing surface could
  quietly fail.
- **AC4.1.6** — *Given* a cross-section containing an inferred value, *When* it
  is displayed in any view on the committed view list, *Then* the distinction is
  stated in words at least once per view — not conveyed by a visual marking alone, which P3 has no street-design
  vocabulary to decode.

### US4.2 — Never be shown a guess as a measurement

**As** Priya (P2), **I want** the tool never to present an inferred value as
though it were measured, **so that** I do not reject a proposal for
overstating what it knows.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR3.3
- **INVEST**: Stated as a prohibition, so it is testable as an invariant across
  every view rather than one screen.
- **Depends on**: US4.1.

- **AC4.2.1** — *Given* any view on the committed view list, *When* it shows an
  inferred value, *Then* the inference is visible in that view.
- **AC4.2.2** — *Given* any export or output the product produces, *When* it
  contains an inferred value, *Then* the inference survives into the output.
  **Falsifiable from Stage 3** — there is no export in Stage 1. The one Stage 1
  artifact that is an output is the persisted design, covered by AC8.1.2.
- **AC4.2.3** — *Given* a rendering path constructed with provenance marking
  unavailable, *When* a value with `inferred` provenance is passed to it,
  *Then* the value is omitted and a stated outcome is returned. Omitting is
  deliberate: an unmarked inferred value is the failure this whole group exists
  to prevent.
- **AC4.2.4** — *Given* a value omitted under AC4.2.3, *When* it is displayed,
  *Then* the omission reads as a deliberate omission rather than as a blank —
  a silent gap where a number was is its own credibility problem.

---

## US5 — Edit the cross-section

The model has **three layers**, established by Q5/Q8 and confirmed independently
by the developer and quality reviews: the imported baseline (immutable), the
correction layer (US7.3 — what the import got wrong), and the design layer
(US5 — what you are proposing). Criteria below say which layer they act on,
because two of them previously described the same mechanism with opposite
meanings.

### US5.1 — Change a lane's type and width

**As** Marcus (P3), **I want** to change what a lane is and how wide it is,
**so that** I can show the street as I am proposing it.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR4.1
- **INVEST**: The smallest useful edit. Everything else in US5 builds on it.
- **Depends on**: US3.1.

- **AC5.1.1** — *Given* an imported cross-section, *When* I select a lane and
  change its type, *Then* the cross-section updates to show the new type.
- **AC5.1.2** — *Given* a selected lane, *When* I change its width, *Then* the
  editing state machine returns the updated model within the same synchronous
  call and no network request is issued. NFR1.2's 100-millisecond user-facing
  budget states no device and does not distinguish input-to-model from
  input-to-paint; it is deferred to `nfr-requirements` with those conditions.
  This criterion asserts the property that actually protects the budget, is free
  to test, and fails loudly the day an await appears in the edit path.
- **AC5.1.3** — *Given* a lane whose width was `mapped`, *When* I change that
  width in the design layer, *Then* its provenance becomes `user-set` for as
  long as the changed value stands.
- **AC5.1.4** — *Given* I change a lane's width, *When* the change is applied,
  *Then* the value it held before the edit remains recoverable.
- **AC5.1.5** — *Given* a width outside the accepted range — zero or negative,
  or greater than 20 metres for a single lane — *When* I enter it, *Then* the
  edit is rejected with a stated reason and the previous value is retained. The
  20-metre ceiling is a sanity bound, not a design standard: it is far above any
  real lane in any jurisdiction, so it catches typos and unit confusion without
  encoding a country's rules against the jurisdiction-neutral mandate. A
  jurisdiction-aware limit, if one is ever wanted, belongs in C9's pluggable
  standards, not here. `team-practices.md`
  requires editing operations to return outcome plus findings; this is the
  criterion that exercises that shape.

### US5.2 — Add and remove lanes

**As** Marcus (P3), **I want** to add a lane that is not there and remove one
that is, **so that** I can propose a different street rather than a re-sized
version of the current one.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR4.2
- **INVEST**: Independent of US5.1 — it changes the lane list rather than a
  lane's values, and it is where the overlay's identity problem lives.
- **Depends on**: US5.1.

- **AC5.2.1** — *Given* a cross-section, *When* I add a lane, *Then* it appears
  in the ordered list at the chosen position with `user-set` provenance on every
  attribute, and it lands in the **design layer** by default. Adding a lane as a
  *correction* to the import is US7.3's AC7.3.2 and is a separate, explicitly
  chosen action.
- **AC5.2.2** — *Given* a cross-section, *When* I remove a lane, *Then* it no
  longer appears and the remaining lanes keep their order.
- **AC5.2.3** — *Given* a lane I added, *When* it is stored, *Then* it carries
  an anchor relative to a keyed baseline lane — left-of or right-of a named
  neighbour, or an offset from a named edge — so that its position has a
  referent when the imported list around it changes. "The same position" has no
  meaning once positional indices are forbidden.
- **AC5.2.4** — *Given* I added a lane that exists in no import, *When* the
  design is saved and reloaded, *Then* that lane returns at the same anchor with
  the same attributes.

### US5.3 — Do every edit without dragging

**As** someone using a keyboard, a screen reader, or a phone, **I want** every
editing action to be reachable by selecting and then acting, **so that** I am
not excluded from the parts of the product that need a precise drag.

- **Priority**: Must Have (Policy — NFR4.1) · **Stage**: 1 · **Traces**: FR4.3,
  FR4.4, NFR4.2, NFR4.5
- **INVEST**: Independent — a second complete path to every edit.
- **Depends on**: US5.1, US5.2.
- **Note on scope**: the criteria below are bound to the **committed
  editing-action list**, which starts as the five `team-practices.md` names —
  select a lane, change its type, change its width, extend along the corridor,
  undo — and grows as editing stories are added. Without that binding, "any
  editing action" is unbounded and the story can never close. `team-practices.md`
  additionally treats keyboard operability as a standing merge gate, so each
  editing story carries its own concrete keyboard criterion too.

- **AC5.3.1** — *Given* the committed editing-action list, *When* each action is
  attempted using selection and commands only, *Then* each completes without a
  drag. Adding an action to the list without a keyboard path fails the build.
- **AC5.3.2** — *Given* the committed editing-action list, *When* an action
  offers a drag interaction, *Then* the same action is reachable by selection.
- **AC5.3.3** — *Given* I am using a keyboard alone, *When* I select a lane,
  change its type, change its width and undo, *Then* each step produces the
  expected model state and announces its accessible name and state.
- **AC5.3.4** — *Given* any zoom level at which a lane's drawn width is below 44
  CSS pixels, *When* I target that lane on a touch device, *Then* its activation
  target is at least 44 by 44 CSS pixels and the lane's drawn width is
  unchanged. Under DOM the activation target is the element's own box, testable
  with emulated touch and a bounding-box measurement. Under canvas it is the
  hit-test region, and **making that region queryable by the test harness is a
  condition of choosing canvas** — otherwise this criterion is untestable and
  the only automated route left is pixel comparison, which is forbidden.

### US5.4 — See what I changed

**As** Priya (P2), **I want** to see what the proposal changed relative to the
street as it exists, **so that** I am assessing a delta rather than
reconstructing one.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR4.5
- **INVEST**: Valuable independently of how edits are made.
- **Depends on**: US5.1.

- **AC5.4.1** — *Given* a design with edits, *When* I ask what changed, *Then* I
  see the three layers distinguished from one another: the imported baseline,
  any corrections made to it, and the proposed design.
- **AC5.4.2** — *Given* a design with no edits, *When* I ask what changed,
  *Then* I am told nothing has changed rather than shown an empty comparison.
- **AC5.4.3** — *Given* a changed attribute, *When* I revert that change,
  *Then* it returns to the value it held before this edit — the corrected
  baseline where a correction exists, otherwise the imported baseline — carrying
  **that value's own provenance**. A reverted mapped width shows `mapped` again,
  because that is what the value now is; provenance describes the value, not the
  history of the field. A US7.3 correction is not discarded by a revert, because
  a correction is not an edit to the design.

### US5.5 — Undo the last thing I did

**As** Marcus (P3), **I want** to undo what I just did, **so that** a
mis-click on a corridor of four streets is a mistake rather than an hour of
rework.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR4.1, FR4.2
- **INVEST**: Independent and long overdue — AC5.3.3 exercises undo,
  `team-practices.md` names it as one of five editing actions in the merge gate,
  and `wireframes.md`'s Success state promises "how to undo it" after a corridor
  apply. Nothing defined it.
- **Depends on**: US5.1, US5.2.
- **Distinct from US5.4**: revert-to-baseline is not undo-last-action.

- **AC5.5.1** — *Given* a sequence of editing actions, *When* I undo, *Then* the
  most recent action is reversed and the model returns to its immediately
  preceding state.
- **AC5.5.2** — *Given* a corridor apply that wrote a design onto several
  streets, *When* I undo, *Then* the entire apply is reversed as one unit, on
  every street it touched.
- **AC5.5.3** — *Given* no action has been taken, *When* I attempt to undo,
  *Then* nothing changes and I am told there is nothing to undo.
- **AC5.5.4** — *Given* I am using a keyboard alone, *When* I undo, *Then* the
  action completes and what was undone is announced.

---

## US6 — Extend a design along a corridor

### US6.1 — Extend a design to one adjacent street

**As** Marcus (P3), **I want** to push the design I just made onto the next
street along, **so that** I can start answering "that only works on that block"
without committing to a whole corridor.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR5.1
- **INVEST**: The smallest complete vertical slice of the differentiator, and
  the one that retires the actual risk. The draft treated the whole corridor
  feature as one unsplittable story; it splits cleanly here into one-street
  extension (US6.1), corridor selection (US6.3) and bulk apply (US6.4).
- **Depends on**: US5.1, US6.3 for the adjacency it needs.

- **AC6.1.1** — *Given* a design on one street and an adjacent street, *When* I
  extend the design onto it, *Then* the target street carries the design.
- **AC6.1.2** — *Given* an extension onto a target whose lane list differs from
  the source's, *When* it is applied, *Then* the design's changes are matched
  onto the target's corresponding lanes by the stated correspondence rule, and
  the target keeps what the design does not speak to. Wholesale replacement of
  the target's lane list is not what "apply" means here — the target's own data,
  including any corrections made to it, survives.
- **AC6.1.3** — *Given* the correspondence rule cannot match a lane, *When* the
  extension is applied, *Then* that lane is left as it was and I am told which
  lanes the design could not be matched onto.
- **AC6.1.4** — *Given* an extension, *When* it completes, *Then* the target
  street retains its own imported baseline and correction layer separately from
  the applied design.
- **AC6.1.5** — *Given* I am using a keyboard alone, *When* I extend to an
  adjacent street, *Then* the action completes and the result is announced.

### US6.2 — Be warned before applying a design that does not fit

**As** Marcus (P3), **I want** to be told before I apply, not after, where a
street cannot take the design, **so that** I do not publish a corridor proposal
containing a street that physically cannot hold it.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR5.2
- **INVEST**: Independent — a check with its own rule, fixed by Q4 to total
  width alone.
- **Depends on**: US6.1.
- **Definition, because osm2streets does not supply one**: the dependency
  returns no kerb-to-kerb width. `Road::total_width()` sums every lane including
  sidewalks, footways and verges, and kerbs appear only as a buffer type when
  `inferred_kerbs` is on. The project therefore owns this definition: **the
  carriageway width is the sum of lane widths between the two kerb buffers where
  both exist, and otherwise the sum excluding walkable lane types and verge
  buffers.** Stated here so two developers cannot implement two different checks.

- **AC6.2.1** — *Given* a design whose total lane width exceeds a target
  street's carriageway width as defined above, *When* I select that street for
  extension, *Then* I am warned before anything is applied.
- **AC6.2.2** — *Given* a warned street, *When* I proceed anyway, *Then* the
  design is applied unchanged and the street remains marked as not fitting.
  Lanes are **not** scaled to fit: a scaled width is neither mapped, inferred nor
  user-set, so scaling would produce a value with no valid provenance state and
  break the model the product's credibility rests on.
- **AC6.2.3** — *Given* a target street whose carriageway width is wholly
  inferred — every contributing lane width a `typical_lane_widths` default —
  *When* the fit is checked, *Then* I am told the fit **could not be checked**
  for that street. A default-versus-default comparison carries no information,
  and reporting it as a finding would present a guess as a measurement.
- **AC6.2.4** — *Given* a target street with no carriageway width at all — an
  import that failed, or data too thin to produce one — *When* the fit is
  checked, *Then* I am told the fit could not be checked, and that street is not
  counted among the streets the design fits.
- **AC6.2.5** — *Given* a design that fits every checked street, *When* I apply
  it, *Then* no warning is shown and any unchecked streets are named as
  unchecked rather than passed over silently.
- **AC6.2.6** — *Given* the fit check, *When* it evaluates any street, *Then* it
  uses total width only and applies no lane-type compatibility rule, so no
  jurisdiction's classifications enter the core model. The check does still
  consume country-parameterised default widths through the adapter's
  `MapConfig`; the fixture must pin `country_code` or the check is not
  reproducible.

### US6.3 — Choose a corridor of connected streets

**As** Marcus (P3), **I want** to see which streets connect to the one I am
working on and pick the ones my proposal covers, **so that** I am describing a
route rather than a list of addresses.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR5.3
- **INVEST**: Independent of applying anything — this is discovery and
  selection. It owns the connected-street graph, which no story previously did.
- **Depends on**: US3.2.
- **Note**: connectivity comes from the street network's own intersection
  topology, not from the coarse display pass. If the coarse pass is a basemap
  tile layer it yields no connectivity at all, so this story owns building or
  fetching the graph.

- **AC6.3.1** — *Given* a street I am working on, *When* I ask to extend along a
  corridor, *Then* I am shown the streets connected to it as a selectable set
  rather than required to visit each one.
- **AC6.3.2** — *Given* the committed intersection fixture, *When* the connected
  set is computed for a named street in it, *Then* the set matches the fixture's
  recorded expectation under the stated connectivity rule.
- **AC6.3.3** — *Given* a connected street with no name in the source data,
  *When* it appears in the set, *Then* it is identified by something stable and
  human-readable, as AC2.2.2 requires for map selection.
- **AC6.3.4** — *Given* I am using a keyboard alone, *When* I work through the
  connected set, *Then* every street in it is reachable and its selected state
  is announced.

### US6.4 — Apply a design across the corridor I chose

**As** Marcus (P3), **I want** to apply my design to every street I selected in
one action, **so that** a ten-block corridor is one decision rather than ten.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR5.1, FR5.3
- **INVEST**: Independent of US6.3's selection and US6.1's single-street
  mechanics; this is the bulk operation and its outcome reporting.
- **Depends on**: US6.1, US6.2, US6.3.

- **AC6.4.1** — *Given* a selected set of streets, *When* I apply the design,
  *Then* it is applied to each of them under AC6.1.2's correspondence rule.
- **AC6.4.2** — *Given* a corridor apply completes, *When* it finishes, *Then* I
  am shown a confirmation naming how many streets received the design, which
  were warned, which could not be checked, and how to undo it.
- **AC6.4.3** — *Given* a corridor apply where one target street cannot be
  prepared, *When* the apply completes, *Then* the operation is not atomic: the
  streets that succeeded keep the design, I am told which did not receive it and
  why, and the failure of one street does not roll back the others.
- **AC6.4.4** — *Given* a corridor apply, *When* I undo it, *Then* every street
  it touched is reverted as one unit, per AC5.5.2.

---

## US7 — Recover when the import goes wrong

### US7.1 — Understand why a street could not be imported

**As** Marcus (P3), **I want** an explanation in terms of the street, not the
software, **so that** I know whether to try again, try a different street, or
start from scratch.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR6.1
- **INVEST**: Small, independently testable — once the failing fixture exists.
- **Depends on**: US3.1.
- **Fixture gap**: `team-practices.md` commits five fixtures and none of them
  fails. A malformed or unimportable extract must be added to the committed set
  before this story's failure path is written, or US7.1, US7.2 and US7.4 have no
  arrangeable starting state at all.

- **AC7.1.1** — *Given* the committed failing fixture, *When* the failure is
  shown, *Then* the message names the street by the same name shown at selection
  and states which member of the adapter's closed set of typed failure reasons
  applies. `team-practices.md` already requires that closed set; using it makes
  this checkable and gives the copy a finite surface one person can write well.
- **AC7.1.2** — *Given* an import failure, *When* the message is shown, *Then*
  it contains no stack trace, no library error text and no internal identifier.

### US7.2 — Never hit a dead end

**As** Marcus (P3), **I want** to be able to try again, and to start from a
blank cross-section if trying again does not help, **so that** a street with
poor map data or a bad moment on the network does not stop me.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR6.2
- **INVEST**: Independent of US7.1 — one explains, one offers a way forward.
- **Depends on**: US7.1, US5.1.

- **AC7.2.1** — *Given* an import failure, *When* the failure is shown, *Then*
  retry is offered as the first action, alongside starting from a blank
  cross-section. The likely failures are transient — a timed-out fetch, a
  network blip, a rate limit — and forcing a blank cross-section after one
  discards the imported street the user actually came for.
- **AC7.2.2** — *Given* I retry after a transient failure, *When* the retry
  succeeds, *Then* I get the imported cross-section, not the blank one.
- **AC7.2.3** — *Given* I accept the blank cross-section, *When* it opens,
  *Then* every attribute I set on it carries `user-set` provenance.
- **AC7.2.4** — *Given* a design built on a blank cross-section, *When* it is
  displayed or exported, *Then* nothing in it is presented as measured.

### US7.3 — Correct what the import got wrong

**As** Priya (P2), **I want** to fix imported values that are wrong before I
design on them, **so that** my proposal is not built on a mistake I could see
and could not change.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR6.4, FR3.4,
  FR2.3
- **INVEST**: Independent of the editing stories — it changes the baseline's
  correctness rather than proposing a change to the street.
- **Depends on**: US4.1, US5.2.

- **AC7.3.1** — *Given* an imported cross-section with a wrong value, *When* I
  correct it, *Then* that attribute's provenance becomes `user-set` in the
  correction layer and never `mapped`. Unlike a design edit (AC5.1.3), a
  correction is permanent: the value is not the imported one and never was.
- **AC7.3.2** — *Given* the committed cycleway fixture, where the import missed
  a separately mapped lane, *When* I add it as a correction, *Then* it becomes
  part of the correction layer rather than the design layer.
- **AC7.3.3** — *Given* an import that invented a lane, *When* I remove it as a
  correction, *Then* the corrected baseline no longer contains it.
- **AC7.3.4** — *Given* any lane the import produced, *When* a correction is
  stored against it, *Then* it is keyed by its OpenStreetMap way id, the
  direction-normalised bounding node-id pair of its segment, and a stable
  per-lane key derived from lane type, direction and ordinal from the left edge
  — never by position in the lane list.
- **AC7.3.5** — *Given* any operation the product performs — a design edit, a
  correction, a corridor apply, a save — *When* it completes, *Then* no write is
  issued to any OpenStreetMap endpoint. Asserted as a global invariant over all
  paths, not for the correction save alone.

### US7.4 — See which streets are failing

**As** Dana (P1), **I want** import failures recorded where I can see them,
**so that** I can find the patterns instead of hearing about one street at a
time.

- **Priority**: Should Have · **Stage**: 1 · **Traces**: FR6.3
- **INVEST**: Serves the maintainer, not a Must Have class.
- **Depends on**: US7.1.

- **AC7.4.1** — *Given* an import failure, *When* it occurs, *Then* the
  underlying error is logged once with the source street identifier, where the
  maintainer can observe it without asking the user.
- **AC7.4.2** — *Given* recorded failures, *When* I look at them, *Then* each
  carries the street identifier and the typed failure reason.
- **AC7.4.3** — *Given* failure recording, *When* it operates, *Then* it stores
  nothing that identifies the person who hit the failure.

### US7.5 — My corrections survive a later import

**As** Priya (P2), **I want** a correction I made to still mean something the
next time the street is imported, **so that** I am not re-fixing the same
mistake every time I open the design.

- **Priority**: Must Have (Persona) · **Stage**: 1 · **Traces**: FR6.4
- **INVEST**: Split out of US7.3 because it carries three things the other
  criteria do not need: a fingerprint of what was imported, the stable per-lane
  key from AC7.3.4, and a user-visible state for a correction that no longer
  resolves.
- **Depends on**: US7.3, US8.1.
- **Why this is not "the correction is re-applied"**: the cross-section is a
  function of the way's tags, the `MapConfig` and the pinned osm2streets
  revision. A re-import differs only when one of those changed — and in every
  such case the lane a correction attached to may no longer be the lane it
  corrected. Re-applying unconditionally would push a stale user width onto a
  lane OpenStreetMap has since re-tagged, which is the same class of harm as
  presenting an inference as a measurement. **A correction can always be
  preserved; it cannot always be re-applied.**

- **AC7.5.1** — *Given* a stored correction, *When* it is saved, *Then* it
  records a fingerprint of what was imported: the way tags, the complete
  `MapConfig` and the pinned osm2streets revision.
- **AC7.5.2** — *Given* a re-import whose fingerprint matches the one recorded
  with the correction, *When* it completes, *Then* the correction is re-applied
  without comment.
- **AC7.5.3** — *Given* a re-import where the fingerprint changed but the
  corrected lane still resolves by its AC7.3.4 key, *When* it completes, *Then*
  the correction is re-applied and I am told the underlying import changed.
- **AC7.5.4** — *Given* a re-import where the corrected lane no longer resolves,
  *When* it completes, *Then* the correction is retained and surfaced as
  unresolved — neither applied nor discarded — and I can see what it was.

---

## US8 — Keep a design and come back to it

### US8.1 — Return to my design without an account

**As** Marcus (P3), **I want** the design I made to still be there when I come
back, **so that** a campaign that runs over weeks does not need me to start
again each time.

- **Priority**: Must Have (Access) · **Stage**: 1 · **Traces**: FR7.2, and the
  walking skeleton's round-trip criterion in `team-practices.md`
- **INVEST**: This is B-0's pass criterion in story form.
- **Depends on**: US5.1.

- **AC8.1.1** — *Given* a design with edits, *When* I close the browser and
  return on the same device, *Then* the design is there with every edit intact.
- **AC8.1.2** — *Given* a reloaded design, *When* I compare it to what I saved,
  *Then* lane order, types, widths and every provenance state are identical.
- **AC8.1.3** — *Given* a design held on my device, *When* it is stored, *Then*
  nothing about it is sent to a server.
- **AC8.1.4** — *Given* I have a stored design, *When* I start work on a second
  street, *Then* the first is not replaced: device storage holds multiple
  designs, and I can reach a specific one again without an account. Stage 1 needs
  the minimum affordance for that; naming and organising designs is US9.3.

### US8.2 — Understand when my work is gone

**As** Marcus (P3), **I want** to be told plainly when work I expected to find
is no longer on this device, **so that** I do not think the product lost it or
that I imagined saving it.

- **Priority**: Must Have (Access) · **Stage**: 1 · **Traces**: FR7.2; derived
  from the Q6/Q9 browser-first decision
- **INVEST**: Independent of US8.1 and easy to omit entirely. Browser storage
  can vanish through a private window, cleared site data, or eviction under
  pressure — none of which the product can intercept.
- **Depends on**: US8.1, US8.3.

- **AC8.2.1** — *Given* I follow a route that names a specific design, *When*
  device storage does not hold it, *Then* I am told that design is not on this
  device, in those terms.
- **AC8.2.2** — *Given* a cold start with empty storage and no design named in
  the route, *When* the tool opens, *Then* I see the normal first-run prompt
  from AC1.2.2 and **no loss message**. Empty storage is also what a genuine
  first-time visitor has; telling a stranger the product lost their work is the
  worst first impression available.
- **AC8.2.3** — *Given* the message from AC8.2.1, *When* I read it, *Then* it
  explains that designs are kept on this device unless uploaded, points at the
  upload action from US8.3, and offers the route to start again.
- **AC8.2.4** — *Given* I open the editor in a mode where storage is
  unavailable, *When* the working surface loads, *Then* I am told before I
  invest any work that nothing will persist here. Storage availability is
  detectable on load, so the warning does not wait for an edit.
- **AC8.2.5** — *Given* storage becomes unavailable mid-session, *When* a save
  fails, *Then* I am told at the moment it happens, and the session's work stays
  available in the open tab until I close it.
- **AC8.2.6** — *Given* storage is unavailable, *When* the tool runs, *Then*
  every action on the committed editing-action list still completes for the
  session.

### US8.3 — Keep a design that is not tied to one device

**As** Marcus (P3), **I want** to explicitly hand a design to the service,
**so that** it survives my laptop and I can pick it up elsewhere.

- **Priority**: Must Have (Access) · **Stage**: 1 · **Traces**: FR7.2; derived
  from the Q9 opt-in-upload decision
- **INVEST**: Raised from Should Have at the mob triage. A weeks-long campaign
  on storage a private window can wipe, with no way off the device, is not a
  credible proposal without professional tooling — it is a proposal with a coin
  flip attached. It is also the mitigation AC8.2.3's mandatory message names.
- **Depends on**: US8.1.

- **AC8.3.1** — *Given* a design on my device, *When* I explicitly ask to keep
  it, *Then* it is uploaded and I am told it now exists off this device.
- **AC8.3.2** — *Given* I have not asked, *When* I use the tool, *Then* nothing
  about my design leaves the device.
- **AC8.3.3** — *Given* an upload that fails, *When* it fails, *Then* the design
  remains on the device, I am told it was not uploaded, and I can try again.
- **AC8.3.4** — *Given* an uploaded design, *When* I ask to remove it, *Then* it
  is deleted from the service without an account being required.
- **AC8.3.5** — *Given* an uploaded design, *When* it is stored, *Then* it is
  stored against an identifier that carries no name, no contact details and no
  cross-site linkage.

---

## US9 — Accounts

### US9.1 — Create an account and sign in

**As** Marcus (P3), **I want** an account, **so that** my designs are mine
across devices and over time.

- **Priority**: Must Have (Persona) · **Stage**: 2 · **Traces**: FR7.1
- **INVEST**: Standard and small; independent of everything in Stage 1.
- **Depends on**: nothing in Stage 1.

- **AC9.1.1** — *Given* I have no account, *When* I complete sign-up, *Then* I
  am signed in.
- **AC9.1.2** — *Given* an account, *When* I sign in on another device, *Then* I
  reach the same saved designs.
- **AC9.1.3** — *Given* the accounts surface exists in a deployed build, *When*
  the access gate is off, *Then* the surface is not reachable by the public, and
  the gate is enforced server-side rather than by hiding the interface.
- **AC9.1.4** — *Given* I ask to do something that requires an account —
  saving under a name, sharing, or reaching a design from another device —
  *When* the sign-in invitation appears and I decline, *Then* I continue with my
  work in progress intact and am not asked again in the same session. The
  invitation is triggered only by such a request: it never fires on an edit and
  never blocks one, which is what keeps AC1.2.3 true.

### US9.2 — Keep what I made before signing in

**As** Marcus (P3), **I want** the design I already made to come with me when I
create an account, **so that** signing up does not cost me the work that
convinced me to sign up.

- **Priority**: Must Have (Persona) · **Stage**: 2 · **Traces**: FR7.2
- **INVEST**: Independent of US9.1's mechanics; it is the migration path.
- **Depends on**: US9.1, US8.1.

- **AC9.2.1** — *Given* a design on my device and no account, *When* I sign up,
  *Then* that design appears among my saved designs.
- **AC9.2.2** — *Given* the migration, *When* it completes, *Then* lane order,
  types, widths and every provenance state are identical to what was on the
  device.
- **AC9.2.3** — *Given* the migration fails, *When* it does, *Then* the design
  remains on the device and I am told it was not moved.

### US9.3 — Save, name and reopen designs

**As** Marcus (P3), **I want** to name my designs and find them again, **so
that** a campaign with several proposals does not become a pile of untitled
work.

- **Priority**: Must Have (Persona) · **Stage**: 2 · **Traces**: FR7.3
- **INVEST**: Small and testable.
- **Depends on**: US9.1.

- **AC9.3.1** — *Given* I am signed in, *When* I save a design with a name,
  *Then* it appears in my list of designs under that name.
- **AC9.3.2** — *Given* a saved design, *When* I reopen it, *Then* lane order,
  types, widths and every provenance state are identical to what was saved.
- **AC9.3.3** — *Given* a name that duplicates one I already used, *When* I
  save, *Then* I am told rather than silently overwriting the other design.
- **AC9.3.4** — *Given* I have no saved designs, *When* I open the list, *Then*
  I see an empty state that explains what will appear here and offers the route
  to find a street.

---

## US10 — Share

### US10.1 — Designs are private until I say otherwise

**As** Marcus (P3), **I want** my designs private by default, **so that** an
unfinished proposal is not discoverable while I am still working on it.

- **Priority**: Must Have (Policy — NFR6.2) · **Stage**: 2 · **Traces**: FR8.1,
  NFR6.2
- **INVEST**: A default, stated as an invariant; testable directly.
- **Depends on**: US9.3.

- **AC10.1.1** — *Given* I save a new design, *When* it is created, *Then* it is
  private with no sharing enabled.
- **AC10.1.2** — *Given* a private design, *When* someone not granted access
  requests it, *Then* they receive a not-found or forbidden response and never
  the design.
- **AC10.1.3** — *Given* an unauthenticated request for any private design,
  *When* it is made directly to the service, *Then* it is refused.

### US10.2 — Choose who can see a design

**As** Marcus (P3), **I want** to grant access to named people or turn on a
link, as separate deliberate choices, **so that** I never publish something
while intending to show it to one person.

- **Priority**: Must Have (Persona) · **Stage**: 2 · **Traces**: FR8.2
- **INVEST**: Independent of US10.1. The two mechanisms are separated in the
  requirement precisely so one cannot be enabled by reaching for the other.
- **Depends on**: US10.1.
- **Open question carried to Domain Design**: whether a person I share with
  needs an account to view. P2's stated barrier is being unwilling to install or
  sign up for anything to read one proposal, and requiring an account puts the
  product's own adoption barrier in front of the persona whose acceptance is the
  differentiator. See OQ-US5.

- **AC10.2.1** — *Given* a private design, *When* I grant access to a named
  person, *Then* only they gain access and no link is enabled.
- **AC10.2.2** — *Given* a private design, *When* I enable a link, *Then* that
  is a separate explicit action from granting named access.
- **AC10.2.3** — *Given* a design with named access granted, *When* I enable a
  link, *Then* the named grants are unchanged; and *when* I disable the link,
  *then* the named grants remain. The two mechanisms are independent in both
  directions.
- **AC10.2.4** — *Given* a design with a link enabled, *When* I disable it,
  *Then* the link stops working immediately.

### US10.3 — See what is shared at a glance

**As** Marcus (P3), **I want** each design's sharing state visible wherever my
designs are listed, **so that** I never have to open a design to find out
whether it is public.

- **Priority**: Must Have (Persona) · **Stage**: 2 · **Traces**: FR8.3
- **INVEST**: Small; independently testable.
- **Depends on**: US10.2.

- **AC10.3.1** — *Given* a list of my designs, *When* it renders, *Then* each
  design shows its current sharing state.
- **AC10.3.2** — *Given* I change a design's sharing state, *When* I return to
  the list, *Then* the displayed state matches.
- **AC10.3.3** — *Given* I am using a screen reader, *When* focus reaches a
  design in the list, *Then* its sharing state is announced, not conveyed by an
  icon alone.

---

## US11 — Data subject rights

These gate Stage 2's public availability rather than being scheduled within it
(`scope-document.md`, and `project.md`'s first affirmed mandate).

### US11.1 — Delete my account and everything in it

**As** Marcus (P3), **I want** to delete my account and have my designs go with
it, **so that** leaving is as straightforward as arriving.

- **Priority**: Must Have (Policy — RC-1/RC-2 gate) · **Stage**: 2 (gating) ·
  **Traces**: FR9.1
- **INVEST**: Independently testable; blocks public release rather than a
  feature.
- **Depends on**: US9.1.

- **AC11.1.1** — *Given* an account with saved designs, *When* I delete the
  account, *Then* my personal data and my designs are removed.
- **AC11.1.2** — *Given* deletion, *When* it completes, *Then* a subsequent
  request for any of those designs returns not-found.
- **AC11.1.3** — *Given* I begin deletion, *When* I confirm, *Then* I am told
  what will be removed before it happens.
- **AC11.1.4** — *Given* a deletion that fails partway, *When* it fails, *Then*
  it is retried to completion or the account remains intact and I am told —
  never a half-deleted account with orphaned designs. This is the failure that
  matters most in the group, because the gate it serves is a legal one.

### US11.2 — Take my designs with me

**As** Marcus (P3), **I want** my designs in a usable, machine-readable form,
**so that** my work is not held hostage by the tool that made it.

- **Priority**: Must Have (Policy — RC-1/RC-2 gate) · **Stage**: 2 (gating) ·
  **Traces**: FR9.2, FR3.3
- **INVEST**: Independent of deletion; different code path, different failure.
  This is also the story that serves P4's stated requirement that a tool must
  let data out again.
- **Depends on**: US9.3.

- **AC11.2.1** — *Given* an account with designs, *When* I request an export,
  *Then* I receive every design in a machine-readable format.
- **AC11.2.2** — *Given* an export, *When* I inspect it, *Then* it contains lane
  order, types, widths and provenance states for each design.
- **AC11.2.3** — *Given* an export, *When* it is produced, *Then* an inferred
  value is identifiable as inferred within it.

### US11.3 — Uploaded anonymous designs do not persist forever

**As** Dana (P1), **I want** designs uploaded without an account to expire,
**so that** the service does not accumulate content nobody can claim or delete.

- **Priority**: Must Have (Policy — RC-1/RC-2 gate) · **Stage**: 2 (gating) ·
  **Traces**: FR9.3, FR9.4
- **INVEST**: Independent; a scheduled behaviour with a clear assertion.
- **Depends on**: US8.3.
- **Carries an open question**: FR9.3 was written as "anonymous designs shall
  expire 30 days after creation", before the browser-first decision. Under that
  decision the product can only expire what it was given, so this story binds
  uploaded designs alone. See OQ-US1.

- **AC11.3.1** — *Given* a design uploaded without an account, *When* 30 days
  pass from its creation, *Then* it is removed.
- **AC11.3.2** — *Given* a design belonging to an account, *When* time passes,
  *Then* it is retained until the account is deleted.
- **AC11.3.3** — *Given* a design held only on a device, *When* time passes,
  *Then* the service takes no action, because it never held it.
  **Falsifiable from the stage that introduces server-side retention** — until
  then it asserts that the service does nothing about data it never received.

---

## US12 — Meeting-ready output

### US12.1 — Produce something for a public meeting

**As** Marcus (P3), **I want** output chosen by what I need it for rather than
by file format, **so that** I am not guessing which format a printed board
needs.

- **Priority**: Must Have (Persona) · **Stage**: 3 · **Traces**: FR10.1
- **INVEST**: The Stage 3 differentiator. Independent of Stage 2.
- **Depends on**: US5.1.
- **Carries a known gap**: `requirements.md` OQ5 records that no city has been
  asked what form output must take. The criteria below test that output is
  produced, purpose-labelled and legible to someone who was not there when it
  was made — not that any city accepts it.

- **AC12.1.1** — *Given* a design, *When* I ask for output, *Then* I choose by
  purpose — something to print, something to project, something to attach —
  rather than by file format.
- **AC12.1.2** — *Given* the "something to print" purpose, *When* the output is
  produced, *Then* it is a single page at a named size and orientation with a
  stated scale.
  **Falsifiable from the stage that names them** — the size, orientation and
  scale are not fixed by any artifact yet, and `requirements.md` OQ5 records
  that no city has been asked. One named purpose with one named artifact is the
  minimum that makes US12.1 implementable; naming the artifact itself is
  outstanding work, not a decision this stage can take honestly.
- **AC12.1.3** — *Given* any produced output, *When* someone who was not present
  when it was made reads it, *Then* it identifies the street and its location
  and states the before-and-after relationship without the producer present.
  This is the criterion that serves the elected officials and passive readers
  `personas.md` records as unmodelled, and it is already drawn — `wireframes.md`
  S8's "Include" checkbox group exists for exactly this reader.
- **AC12.1.4** — *Given* output production fails, *When* it does, *Then* I am
  told which purpose failed and the design is unaffected.

### US12.2 — Output keeps the provenance distinction

**As** Priya (P2), **I want** the output to show which numbers were measured and
which inferred, **so that** what reaches a formal process carries the same
honesty as what was on screen.

- **Priority**: Must Have (Persona) · **Stage**: 3 · **Traces**: FR10.2, FR3.3
- **INVEST**: Independent of US12.1's format work; a property of every output.
- **Depends on**: US12.1, US4.2.
- **Format dependency**: the structural assertion below requires a vector or
  document output that can be inspected. In a raster output there is nothing to
  inspect and pixel comparison is forbidden, so US12.1's chosen formats
  determine whether this is CI-verifiable. Stated here rather than discovered at
  Stage 3.

- **AC12.2.1** — *Given* any produced output, *When* it contains an inferred
  value, *Then* the inference is visible in the output itself, not only in a
  companion note.
- **AC12.2.2** — *Given* an output generation path constructed with provenance
  marking unavailable, *When* a value with `inferred` provenance is passed to
  it, *Then* the value is omitted and a stated outcome is returned.
- **AC12.2.3** — *Given* a produced output containing an inferred value, *When*
  it is generated, *Then* the inferred marking is carried by at least one
  non-colour channel — a hatch pattern, a symbol, or a text marker — present in
  the output's structure. The legibility judgement itself joins the defined
  manual walkthrough before each stage's release, under the same honest heading
  `team-practices.md` already uses: WCAG 2.1 AA is not fully CI-verifiable.

### US12.3 — See the output before producing it

**As** Marcus (P3), **I want** to preview output before producing it, **so
that** I do not print six boards and discover the labels are unreadable.

- **Priority**: Should Have · **Stage**: 3 · **Traces**: FR10.3
- **INVEST**: Genuinely optional to the value; a time-saver rather than a
  blocker.
- **Depends on**: US12.1.
- **Wireframe consequence**: `wireframes.md` S8 draws the preview inside the
  export dialog. If US12.3 does not ship, S8 needs a preview-less variant.

- **AC12.3.1** — *Given* a chosen purpose, *When* I preview, *Then* I see what
  will be produced before committing to it.
- **AC12.3.2** — *Given* a preview, *When* I change the design, *Then* the
  preview reflects the change.

---

## US13 — Commitments that cut across every screen

These are not a workflow step. They are invariants that no single story could
carry, promoted to stories for the same reason US2.3 and US5.3 were: a
cross-cutting commitment buried in one screen's criteria is a commitment nobody
tests.

### US13.1 — The product says out loud what it just did

**As** someone using a screen reader, **I want** to hear what changed when it
changes, **so that** I am not left guessing whether a ten-second import
finished.

- **Priority**: Must Have (Policy — NFR4.1, WCAG 2.1 AA SC 4.1.3) · **Stage**: 1
  · **Traces**: NFR4.1, NFR4.3
- **INVEST**: One invariant, testable against the committed status-message list.
  Holds whichever way OQ2 is settled — a live region lives in the surrounding
  page even if the editing surface is canvas.
- **Depends on**: the stories that produce each status.
- **Why this exists**: every status in the draft was written visual-only —
  search found nothing, import in progress, import complete, fit warning,
  corridor applied, upload succeeded, output failed. Each is a state change away
  from focus. A screen-reader user selected a street and heard nothing for ten
  seconds, then nothing when it arrived.

- **AC13.1.1** — *Given* the committed status-message list, *When* the product
  reports any of them visually, *Then* the same information is announced without
  moving focus.
- **AC13.1.2** — *Given* a status the product reports, *When* it is added to the
  product without an entry in the committed list, *Then* the build fails.
- **AC13.1.3** — *Given* an import that takes several seconds, *When* it
  completes, *Then* completion is announced — not only the start.

### US13.2 — The editing surface works on a phone

**As** Marcus (P3), **I want** to design on the device I actually have with me,
**so that** I can work at the bus stop I am proposing to move.

- **Priority**: Must Have (Policy — NFR4.4) · **Stage**: 1 · **Traces**: NFR4.4,
  NFR4.5
- **INVEST**: Independent, and load-bearing: `user-flow.md` says the audience
  arrives "frequently on a phone", and the whole Option B layout was chosen over
  A and C for "one layout for every form factor". That form factor had no
  acceptance criterion — NFR4.4 was marked covered by the landing-page criterion
  alone, which is the "works on phones" defect returning in traceability form.
- **Depends on**: US5.1, US5.2, US5.3.

- **AC13.2.1** — *Given* the editing surface at a viewport 360 CSS pixels wide,
  *When* I perform each action on the committed editing-action list, *Then* each
  completes and the page body does not scroll horizontally.
- **AC13.2.2** — *Given* a viewport 360 CSS pixels wide, *When* the
  cross-section, the lane list and every editing control render, *Then* each is
  reachable and operable.
- **AC13.2.3** — *Given* the mobile browser matrix in NFR4.6, *When* AC13.2.1
  runs, *Then* it runs against that matrix in the slow CI tier.

---

## Won't Have this time

| Item | Source | Why not now |
|---|---|---|
| City GIS import and export | `scope-document.md` C8 | Should Have after the three stages; not selected as must-have |
| Pluggable jurisdiction standards | `scope-document.md` C9 | Should Have after the three stages; deferring keeps the core jurisdiction-neutral |
| Traffic simulation and modelling | `requirements.md` Out of Scope | Different product |
| Construction- or engineering-grade output | `requirements.md` Out of Scope | Would change the product's claims and its liability |
| Editing OpenStreetMap data | `requirements.md` Out of Scope; standing prohibition | Designs are proposals over the map, never changes to it |

## Verification notes — engineering requirements, not user stories

The draft carried several criteria whose actor was the build rather than a
person. They are correct as engineering requirements and wrong as acceptance
criteria, so they are recorded here and owned elsewhere. They are not
Given/When/Then criteria and no story traces to them.

| Requirement | Owner |
|---|---|
| Session cookies carry `HttpOnly`, `Secure` and a `SameSite` policy | `team-practices.md` Deployment; asserted in the Stage 2 test suite |
| The imported baseline is held behind shared, never mutable, references | `team-practices.md` Code Style; enforced by crate boundaries |
| Every dimension crossing the adapter boundary carries provenance at the type level | `team-practices.md` Code Style; enforced by the type system |
| The overlay key never uses osm2streets positional indices | `team.md`, as amended at this stage's learnings ritual; asserted by AC5.2.3 and AC7.3.4 through observable behaviour rather than by inspecting storage |

## Corrections required to approved wireframes

Three approved screens contradict a Must Have criterion. Recorded here for
Refined Mockups (2.5) to correct rather than left standing.

| Screen | What it says | Why it must change |
|---|---|---|
| `wireframes.md` S4 | "Oak Street is narrower than Elm. **Lanes will be scaled to fit.**" | Contradicts AC6.2.2. A scaled width is neither mapped, inferred nor user-set, so scaling produces a value with no valid provenance state under FR3.1 |
| `wireframes.md` S7 | Three mutually exclusive sharing radios (Only me / People I invite / Anyone with the link) | Contradicts AC10.2.1–AC10.2.3. FR8.2 requires two independent mechanisms; the radio model silently destroys named grants when a link is enabled, and kills a live link when narrowing back |
| `wireframes.md` S8 | Preview drawn inside the export dialog | US12.3 is Should Have; S8 needs a preview-less variant, or US12.3 must be raised |

## Dependencies and critical path

```
US1.1        (independent — a page, gates nothing)
US2.1 ─┬─► US2.2 ─┬─► US2.3
       │          └─► US3.1 ─┬─► US4.1 ──► US4.2
       │                     │      └────► US7.3 ──► US7.5
       │                     ├─► US5.1 ─┬─► US5.2 ─┬─► US5.3 ──► US13.2
       │                     │          ├─► US5.4  └─► US5.5
       │                     │          ├─► US7.2
       │                     │          └─► US8.1 ─┬─► US8.3 ──► US8.2
       │                     │                     │       └───► US11.3
       │                     │                     └─► US7.5
       │                     └─► US7.1 ─┬─► US7.2
       │                                └─► US7.4
       ├─► US1.2
       └─► US3.2 ──► US6.3 ──► US6.1 ──► US6.2 ──► US6.4

US13.1 ──► (every story that reports a status)

US9.1 ─┬─► US9.2 (also needs US8.1)
       ├─► US9.3 ─┬─► US10.1 ──► US10.2 ──► US10.3
       │          └─► US11.2
       └─► US11.1

US5.1 ──► US12.1 ─┬─► US12.2 (also needs US4.2)
                  └─► US12.3
```

The critical path to a usable Stage 1 runs
US2.1 → US2.2 → US3.1 → US5.1 → US8.1, with US4.1 for the provenance half.

**That path is the story-shaped part of B-0, not the whole of it.**
`team-practices.md` also requires B-0's exit to confirm that the pinned
osm2streets build produces a working artifact end to end — the project's single
Critical dependency (`raid-log.md` R-1, D-1) — which AC3.1.5 carries but which is
a build assertion rather than a user story. OQ1 must also be settled before any
of it runs. The earlier draft claimed the path *was* B-0; it is not.

## Amendments this stage requires of approved artifacts

These are not open questions. They are work with an owner, listed separately
because Domain Design and every later stage read `requirements.md` and
`scope-document.md` — not this file's open-question list. A decision recorded
only here can be lost.

| Artifact | What must change | Why | Before |
|---|---|---|---|
| `requirements.md` | Add a requirement for undoing an editing action, covering a corridor apply as one undoable unit | US5.5 is a Stage 1 Must Have tracing to no requirement. `team-practices.md` already names undo in its merge gate and `wireframes.md` promises it after a corridor apply, so the obligation exists everywhere except the requirements | Domain Design |
| `requirements.md` | Add a requirement for the case where device storage no longer holds a design | US8.2 is a Stage 1 Must Have tracing to no requirement. FR7.2 covers a design surviving sign-in; nothing covers storage losing it, which the browser-first decision made the common case | Domain Design |
| `requirements.md` | Add a requirement for explicitly uploading a design without an account | US8.3 is a Stage 1 Must Have tracing to no requirement, raised from Should Have at Q12 | Domain Design |
| `requirements.md` | Amend FR9.3 — "Anonymous designs shall expire 30 days after creation" — to bind uploaded designs only | The product never holds designs that were not uploaded, so the requirement as approved asserts an obligation over data it does not have. US11.3 is written to the narrower reading | Domain Design |
| `scope-document.md` | Record that Stage 1 now includes server-side storage of anonymous uploaded designs | Q9 chose browser-first with opt-in upload and Q12 made the upload Must Have. `scope-document.md` places all server-side storage in Stage 2, and NFR5.2's client-side posture did not anticipate it. This also reopens whether the Stage 2 public-release gate reaches back into Stage 1 — the question Q9 shrank rather than closed | Domain Design, and before any Stage 1 release |

Until these land, an artifact a later stage trusts contradicts one it does not
read.

## Open questions raised by this stage

- **OQ-US1** — `requirements.md` FR9.3 states "Anonymous designs shall expire 30
  days after creation" without qualification. The browser-first persistence
  decision (Q6/Q9) means the product never holds designs that were not uploaded,
  so FR9.3 can only bind uploaded ones. US11.3 is written to that narrower
  reading, and the requirement itself should be amended rather than reinterpreted
  in place. US8.2 and US8.3 likewise have no FR behind them. Raised against
  `requirements.md`.
- **OQ-US2** — Whether an uploaded anonymous design is reachable by link before
  accounts exist. US8.3 covers the upload; it does not say whether uploading
  produces a shareable URL. If it does, Stage 1 has a sharing surface ahead of
  `scope-document.md`'s Stage 2 placement of C6, and US10's privacy defaults
  would need to apply a stage earlier. For Domain Design.
- **OQ-US3** — What "suitable for a public meeting" means concretely (FR10.1,
  US12.1). `requirements.md` OQ5 already records that no city has been asked.
  AC12.1.2 names one artifact so the story is implementable; the rest waits.
- **OQ-US4** — Elected officials and people who only need to understand a
  proposal are not modelled as a persona (`personas.md`, coverage gap). AC12.1.3
  and AC1.1.2 serve them directly. Whether that is adequate cannot be known until
  someone in that group is spoken to.
- **OQ-US5** — Whether a person a design is shared with needs an account to view
  it (US10.2). This lands on P2's stated barrier — unwilling to sign up to read
  one proposal — and P2 is the persona whose acceptance is the differentiator.
  For Domain Design.
- **OQ-US6** — The correspondence rule AC6.1.2 depends on is named but not
  defined: how a design's lanes are matched onto a target street's lanes when the
  lists differ. Q13 settled that it is per-lane mapping rather than wholesale
  replacement; the rule itself is a model question. For Domain Design.
- **OQ-US8** — FR6.3 ("The system shall record import failures where the
  maintainer can observe them") and FR10.3 ("The system shall preview output
  before it is produced") are both written as obligations in the approved
  requirements and are delivered here as Should Have (US7.4, US12.3). The
  priority call is defensible — neither is needed for P3 to produce a credible
  proposal — but it leaves `requirements.md` asserting two obligations the story
  set does not commit to. US12.3's optionality is load-bearing elsewhere: the S8
  wireframe correction depends on it. Either the requirements soften to
  "should", or both stories rise to Must Have. Raised against `requirements.md`.
- **OQ-US7** — Whether the walking skeleton's third pass criterion is
  automatable. `team-practices.md` requires mapped values "visibly distinguished
  from inferred ones in what is rendered". AC4.1.4 discharges this through the
  accessibility tree, which exists under DOM and under a canvas that exposes one.
  If Domain Design selects canvas without an accessibility tree, B-0's third
  criterion falls to the manual walkthrough on the very first Bolt, indefinitely.
  For Domain Design, and it should be weighed there as a cost of the canvas
  choice rather than discovered after B-0.


## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-10T01:35:20Z
**Iteration:** 1
**Request Challenge:** review:2ec4bd98371954025da141a042be45a5

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `stories.md` > `## Three committed lists` | The editing-action list (7 members), the view list (6 members) and the status-message list (23 members) are now seeded in the same explicit form US5.3 used for editing actions, each with a stated binding to the criteria that depend on it. I checked every AC id cited in the status-message list against the story text it is claimed to summarise (AC2.1.4, AC3.1.3, AC3.2.4, AC5.1.5, AC5.5.4, AC6.1.3, AC6.1.5, AC6.2.1, AC6.2.3, AC6.2.4, AC6.4.2, AC6.4.3, AC7.1.1, AC7.5.3, AC7.5.4, AC8.2.1, AC8.2.4, AC8.2.5, AC8.3.1, AC8.3.3, AC9.2.3, AC9.3.3, AC12.1.4) and every one exists and says what the list claims for it. AC4.1.2/AC4.1.6/AC4.2.1 and AC5.3.1/AC5.3.2/AC8.2.6/AC13.2.1 now have a definite, enumerated subject to test membership against. | None — closed. | Resolved |
| R-02 | Major | `stories.md` > `## Amendments this stage requires of approved artifacts` | The five amendments (undo, storage-loss, opt-in-upload requirements; the FR9.3 narrowing; the Stage-1 server-side-storage note in `scope-document.md`) are now stated as owned work with a reason and a "before Domain Design" deadline in a section the lead draft's own rationale says exists precisely because downstream stages read `requirements.md`/`scope-document.md` and not the open-question list. I confirmed `requirements.md` still carries the unamended FR9.3 and no undo/storage-loss/upload requirement — consistent with the amendments being outstanding work due before Domain Design rather than a claim that they are already done. This closes the finding as specified: a later stage now has a named, dated obligation to reconcile, not a disclosure it can miss. | None — closed. | Resolved |
| R-03 | Minor | `stories.md` > `## How these are organised` > MoSCoW table | The Policy row now states its qualifying rule ("any NFR in `requirements.md` stated as a commitment rather than a target, and the RC-1/RC-2 public-release gate") rather than naming two examples as if exhaustive. I checked it against every story classed Policy — US2.3/US5.3/US13.1 (NFR4.1), US10.1 (NFR6.2), US13.2 (NFR4.4), US11.1/US11.3 (RC-1/RC-2) — and each cited NFR is written with "shall" in `requirements.md`, so all seven fit the stated rule with no exception needed. The following paragraph now gives US1.1 an explicit, coherent argument for Access classification (judging whether the tool is worth the reader's time is treated as part of "reaching" it). | None — closed. | Resolved |
| R-04 | Minor | `stories.md` > `## Open questions raised by this stage` > OQ-US8 | OQ-US8 now raises the FR6.3/FR10.3 "shall" vs. Should-Have mismatch in the same form as OQ-US1, naming both requirement IDs, both story targets (US7.4, US12.3), and that US12.3's optionality is load-bearing for the S8 wireframe correction. | None — closed. | Resolved |
| R-05 | Minor | `stories.md` > AC5.1.5 | AC5.1.5 now states a 20-metre ceiling and gives the reasoning for why it is a sanity bound rather than a design standard, with an explicit pointer away from the jurisdiction-neutral mandate ("far above any real lane in any jurisdiction... without encoding a country's rules"; a jurisdiction-aware limit is placed in C9's pluggable standards instead). I checked this against `project.md`'s ALWAYS rule that the core model must not bake in any jurisdiction's classifications or widths and found no tension: 20 m is well above any real-world lane width in any jurisdiction, so the bound catches typos/unit errors without encoding a country's rule. | None — closed. | Resolved |
| R-06 | Minor | `stories.md` > AC3.2.1 | AC3.2.1 now names a viewport covering at least 2,000 streets as "the committed city-scale fixture area, which is what NFR2.1's 'thousands of streets' means operationally" — a stated, checkable quantity in place of the undefined "city-scale area". | None — closed. | Resolved |
| R-07 | Minor | `stories.md` > AC9.1.4 | AC9.1.4 now states its trigger explicitly (asking to save under a name, share, or reach a design from another device) and states the invariant that resolves the tension with AC1.2.3 in the same sentence: "it never fires on an edit and never blocks one, which is what keeps AC1.2.3 true." I re-read both criteria together and found no remaining contradiction — AC1.2.3 is about edits with no account, AC9.1.4 is scoped to a narrower, distinct set of account-requiring actions. | None — closed. | Resolved |
| R-08 | Minor | `stories.md` > new AC4.1.5 | A new AC4.1.5 now asserts that each lane in an imported cross-section is individually reachable by keyboard focus in display order, closing the presupposed-but-unasserted half of NFR4.3. The renumber (old AC4.1.5 became AC4.1.6) was checked for stale references: the `## Three committed lists` view-list binding on line 63 now cites AC4.1.2, AC4.1.6, AC4.2.1 (updated), and I found no remaining reference to an AC4.1.5 meaning the old wording anywhere in `stories.md`, `personas.md` (which carries no AC references at all), or `traceability.json` (which carries no AC-level references either). | None — closed. | Resolved |
| R-09 | Minor | `stories.md` > AC3.1.6, AC12.1.2 | Both criteria now carry an explicit "Falsifiable from" label naming the artifact that will supply the missing value (`nfr-requirements` for the import budget, "the stage that names them" for AC12.1.2's page size/orientation/scale), matching the convention the set already uses elsewhere (AC1.2.3, AC4.2.2, AC11.3.3). | None — closed. | Resolved |

### Summary

All nine findings are resolved on inspection, not merely relabelled: the three committed lists are seeded with concrete, checkable membership and every cross-referenced AC id in the status-message list actually says what the list claims; the amendments table converts the requirements/scope gap from disclosure into dated, owned work while `requirements.md` correctly still shows the pre-amendment text; the MoSCoW Policy rule now covers every story it classifies; and the five smaller precision gaps (OQ-US8, the AC5.1.5 ceiling, the AC3.2.1 scale, the AC9.1.4 trigger, the AC3.1.6/AC12.1.2 labels) are each closed with reasoning that holds up against the standing jurisdiction-neutral and no-account-friction constraints. The AC4.1.5→AC4.1.6 renumber left no stale reference in any sibling artifact. I found no new contradiction introduced by any fix.
