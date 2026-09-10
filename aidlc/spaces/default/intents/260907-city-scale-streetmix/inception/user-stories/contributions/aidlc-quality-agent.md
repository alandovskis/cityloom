**Collaborator:** aidlc-quality-agent

## Contribution

I read every acceptance criterion in `stories.md` and asked one question of each:
can I write a test that fails for the right reason before the implementation
exists? That is the bar the affirmed TDD ordering sets — the failing test, or
the executable form of the criterion, comes first — so a criterion that cannot
be made to fail is a criterion that produces no test at all.

Most of this draft passes that bar, and several criteria are better than the
requirements they trace to. The failures are concentrated in four places: the
two performance numbers, three criteria whose preconditions cannot be
constructed, one criterion that asserts a test rather than a behaviour, and a
set of `OK` traceability rows that claim more than the named stories deliver.

---

### 1. AC3.1.2 and AC5.1.2 are the same defect as "works on phones"

**AC3.1.2** — *"the import runs, Then it completes within 10 seconds."*
**AC5.1.2** — *"the change is visible within 100 milliseconds."*

Neither is testable as written, and the reason is identical to the objection
that was upheld at Practices Discovery: a threshold without stated measurement
conditions looks checkable and is not. Specifically, both are missing:

- **Hardware.** Which device? `NFR4.6` names a mobile matrix (current and
  previous Safari on iOS, Chrome on Android) but the traceability marks NFR4.6
  `Deferred`, and `OQ6` records that no desktop matrix exists at all. So the
  machine these numbers are measured on is an open question, not a given.
- **Network.** AC3.1.2 in particular. `OQ1` explicitly leaves unsettled whether
  OSM data is fetched by the browser directly or routed through the Railway
  service. That fetch is plausibly the dominant term in the 10 seconds. You
  cannot write a failing test for a budget whose largest component is unplaced.
- **Street.** "a street" — a residential cul-de-sac and a six-lane arterial with
  a separately mapped cycleway are not the same import. The committed fixture
  set gives us bounded candidates; the criterion should name one.
- **Boundary.** AC3.1.2: from selection to what — first render, or interactive?
  AC5.1.2: input-to-model-update, or input-to-painted-frame? Those are different
  numbers and only the first is deterministically measurable without a device.
- **Statistic.** One run, the median of five, the slowest of five? Without this
  the test is a coin flip and will be quarantined within a month.

I am not proposing a device lab or paid tooling — the budget forbids it and so
does the brief. I am proposing the split this project has already used twice:

**(a) A deterministic part that runs in CI today.** For AC3.1.2: *"Given the
committed well-tagged fixture, when the adapter converts it, then the conversion
completes within N milliseconds on the CI runner"* — with N **recorded, not
gated**, until a real measurement exists. For AC5.1.2: *"Given a selected lane,
when I change its width, then the editing state machine returns the updated
model within the same synchronous call and no network request is issued."* That
second one is the property that actually protects a 100ms budget, it is free to
assert, and it fails loudly the day someone puts an await in the edit path.

**(b) The user-facing figure goes back to `nfr-requirements` with the conditions
attached.** This is not me inventing a threshold — the numbers are inherited
from NFR1.1 and NFR1.2. It is refusing to invent the *conditions*, which is the
same discipline `team-practices.md` applies to the "week to under a day" claim
and to branch coverage ("reported, not gated, until a real number exists to set
a floor against at `nfr-requirements`"). The precedent is already in the
document set; these two criteria are the exception to it.

There is also a missing sad path implied by AC3.1.2 and written nowhere: what
happens at 10.001 seconds? Nothing in US3.1 says whether a slow import is
abandoned, and if it is not, the criterion has no failure branch to test.

### 2. AC4.1.4 asserts that a test can exist, not that the system does anything

*"Then an automated test can assert that value is rendered distinguishably from
a mapped one."*

Two problems. First, the subject of the criterion is a test, so the test that
discharges it is circular — a test asserting that a test can be written passes
by being written. Second, and this is the substantive one: whether the statement
is **true** depends on `OQ2`, which Domain Design has not settled. If the
editing surface is canvas, there is no DOM node carrying the distinction, and
the only automated route left is pixel comparison of rendered output — which the
standing constraint forbids outright. So under a canvas choice this criterion is
either unsatisfiable or it forces a banned technique, and it says nothing about
that.

**AC4.1.5, immediately below it, is already the correct form** — *"the inferred
state is announced, not conveyed by colour alone"* — because the accessibility
tree is an observable that survives both branches of OQ2 (`NFR4.3` requires lane
elements to be individually focusable and announceable regardless of what draws
them). The fix is to delete AC4.1.4 and strengthen AC4.1.5 with the fixture
binding AC4.1.4 was reaching for:

> *Given the committed thinly-tagged fixture, which yields an `inferred` width,
> When focus reaches that lane, Then the inferred state is present in its
> announced accessible name or state, and absent from the announced state of a
> lane whose width is `mapped`.*

That is executable on DOM and on canvas-with-an-accessibility-tree, needs no
pixel comparison, and fails for the right reason. If Domain Design picks canvas
*and* provides no accessibility-tree representation, this criterion fails — but
so does NFR4.3 independently, which is the correct outcome rather than a
surprise at release.

**AC4.1.2** has a related but smaller problem: *"wherever the cross-section
appears"* is an unbounded quantifier. You cannot test "every view" without an
enumeration of views. Bind it to a committed list, or state that it is
discharged by a single shared rendering component plus one test per listed view.

### 3. AC12.2.3 and AC5.3.4 — the two the brief asks about directly

**AC12.2.3** (*"Given a printed output, When it is read without colour, Then the
distinction is still discernible"*) has **no CI-executable assertion at any
budget as written**. "Printed", "read" and "discernible" are all human acts or
human judgements. It will sit in the story set looking like a gate and never be
tested. It is also the odd one out in its own family: AC4.1.5 and AC10.3.3 both
state the same WCAG "not by colour alone" idea in a testable form, and this one
does not. Restate it structurally and route the judgement where this project
already routes judgements:

> *Given a produced output containing an inferred value, When it is generated,
> Then the inferred marking is carried by at least one non-colour channel — a
> hatch pattern, a symbol, or a text marker — present in the output's structure.*

That is assertable against a vector or PDF output. The legibility judgement then
joins the **defined manual walkthrough before each stage's release** that
`team-practices.md` already establishes for the screen-reader pass, under the
same honest heading that WCAG 2.1 AA is not fully CI-verifiable. Note that even
the structural form depends on which formats US12.1 produces: in a raster output
there is nothing to inspect, and pixel comparison is forbidden. That dependency
should be stated in US12.2 rather than discovered at Stage 3.

**AC5.3.4** (44 by 44 CSS pixel activation target) is the better news: it **is**
testable in CI at this budget, under one branch of OQ2, and it needs two
additions to become executable.

- *Under DOM*: Playwright emulated touch (`hasTouch`) at a 360 CSS pixel
  viewport is free and runs in the slow tier that already exists;
  `getBoundingClientRect()` on the lane's activation element asserts both
  dimensions are at least 44, and a separate assertion confirms the drawn width
  still equals the true scaled width. No device lab, no paid tooling.
- *Under canvas*: there is no element to measure. The hit-test region is
  internal, so the criterion becomes untestable unless that region is made
  queryable by the test harness. That is a **testability requirement Domain
  Design must accept as a condition of choosing canvas**, and it belongs in the
  criterion. `NFR4.5` already says the right thing ("on real page elements the
  target is the element's own box, and on a canvas it is the hit-test region");
  the AC dropped that sentence and should carry it.
- **"at true scale" has no stated starting state.** At what zoom? A 3.5 m lane
  is only narrower than 44 px below some zoom level. Better as a property:
  *"Given any zoom level at which a lane's drawn width is below 44 CSS
  pixels…"*, which removes the need to name a zoom and makes it a real invariant.
- "on a touch device" leans on `NFR4.6`, which the traceability marks
  `Deferred`. Worth naming that dependency.

### 4. Criteria whose starting state cannot be constructed

These three cannot be arranged, so no test can be written against them:

- **AC3.2.3** — *"Given a city-scale network, When I work continuously for a
  session, Then only the visible portion is held loaded."* Which network, how
  many streets? How long is a session, doing what? "Held loaded" measured how?
  `NFR2.2` states plainly that no figure is set and this is "a constraint on
  design, not a testable threshold yet" — and the traceability correctly marks
  NFR2.2 `Deferred`. AC3.2.3 then writes an acceptance criterion for exactly
  that untestable thing. Replace it with the eviction behaviour, which is
  deterministic and needs no memory figure:
  > *Given N streets have been fully imported, When the viewport moves so that M
  > of them are out of view, Then the count of retained full-import records is
  > N − M.*
- **AC4.2.3** — *"Given a view where the provenance marking cannot be
  rendered…"*. There is no such view to arrange; you would have to invent one.
  Make it a property of the renderer instead: *"Given a rendering path
  constructed with provenance marking unavailable, When a value with `inferred`
  provenance is passed to it, Then the value is omitted and a stated outcome is
  returned."* That is a unit test on a contract and it can fail.
- **AC12.2.2** — identical shape, identical fix.

**AC5.3.2** — *"When I inspect that action"* — is a review step, not an
execution step. Together with **AC5.3.1**'s unbounded *"any editing action the
product offers"*, the fix is the same: bind both to a **committed enumeration of
editing actions**. `team-practices.md` already names five (select a lane, change
type, change width, extend along the corridor, undo). Parameterise the test over
that list, and adding a sixth action without a keyboard path then fails the
build. Without the binding, "any" degenerates to whatever the test author
happened to remember. The same fix applies to **AC8.2.4**'s "fully usable".

### 5. Criteria that assert nothing capable of failing

Under a tests-first posture these produce green-from-birth tests that get
counted as coverage:

- **AC1.2.3** (*"nothing asks me to sign in to continue"*) — there is no sign-in
  in Stage 1. It cannot be made to fail until US9.1 exists.
- **AC4.2.2** (*"any export or output"*) — there is no export in Stage 1; US12
  is Stage 3. Same problem. The one Stage 1 artifact that *is* an output is the
  persisted design, and AC8.1.2 already covers its provenance.
- **AC11.3.3** (*"the service takes no action, because it never held it"*) —
  asserts that the service does nothing about data it never received. Cannot
  fail by construction.
- **AC4.1.1** (*"exactly one provenance state"*) — guaranteed by the affirmed
  `Provenance` enum and the unconstructable-un-annotated-form rule. Cheap and
  worth keeping as documentation of the invariant, but it is not evidence.

Mark the first three as **binding from the stage that makes them falsifiable**,
so nobody writes them in Stage 1 and counts them. That is not a request to
delete them — they are good regression guards — it is a request that the story
set say which of its criteria can actually go red in the Bolt that implements it.

### 6. A contradiction between AC5.4.3 and AC7.3.4

**AC5.4.3** — *"When I revert a single changed attribute, Then it returns to the
imported baseline value with its original provenance."*
**AC7.3.4** — *"Given I have corrected a street, When that same street is
imported again later, Then my correction is still applied rather than silently
discarded."*

Q5/Q8 established a three-layer model: imported baseline, user corrections
(`user-set`, per FR3.4), and the proposed design. If a user corrects a width
(making it `user-set`), then designs on it, then reverts the design change,
AC5.4.3 as written returns the value to the **imported** figure with its
**original** (`mapped`) provenance — silently discarding the correction that
AC7.3.4 promises survives even a full re-import. A test author has to pick one,
and the inception guardrail forbids carrying an unresolved contradiction
forward. Fix AC5.4.3:

> *…Then it returns to the value it held before this edit — the corrected
> baseline where a correction exists, otherwise the imported baseline — with
> that value's provenance.*

### 7. The committed fixture set is under-used, and one fixture does not exist

`team-practices.md` commits five fixtures: a well-tagged street, a thinly-tagged
street, a one-way, a street with a separately mapped cycleway, and one
intersection. The story set uses two of them (AC3.1.4 well-tagged, AC4.1.3 and
AC4.1.4 thinly-tagged). Three are referenced by no criterion, and each has an
obvious home:

- **the one-way fixture** → AC3.1.1 asserts lanes come back with "a type, a
  direction and a width", but no criterion checks a direction against a fixture
  with a known one. Bind it.
- **the cycleway fixture** → AC7.3.2's "an import that missed a lane" case.
- **the intersection fixture** → AC6.1.1's connected-street set (see below).

**A failing-import fixture does not exist.** US7.1's INVEST line says it is
"independently testable against a fixture that fails" — but no such fixture is in
the committed set. Under the affirmed ordering, that fixture must be committed
before the failure path is written. Without it, US7.1, US7.2 and US7.4 have no
arrangeable starting state at all. Add a malformed or unimportable extract to the
committed set and name it in those Givens.

### 8. The fixture suite cannot detect the thing it exists to detect

This is the finding I would most want acted on. `team-practices.md` states the
fixture suite's purpose: *"to tell the builder the day the dependency's
behaviour moves underneath them."*

AC3.1.4 tests **the adapter against a recorded fixture**. The fixture *is*
osm2streets' output, captured once. So the suite can be green while the pinned
osm2streets crate is broken, unbuildable, or producing different output — the
fixture is a frozen memory of what osm2streets said, and nothing re-asks it.
No criterion in the draft asserts that the pinned crate is invoked at all.

`team-practices.md` is explicit that B-0's exit "also confirms the pinned build
actually produces a working binding end to end — it is not just a UI/product
exit criterion." Nothing in the story set carries that. Add to US3.1:

> *Given a committed raw OSM extract, When the pinned osm2streets crate
> processes it in the release-mode build, Then its output matches the committed
> fixture for that extract.*

That belongs in the slow tier, where the release-mode WASM build and the
from-source osm2streets compile already live, so it costs nothing new. It is the
only test in the set that would go red the day the dependency moves — which is
the single Critical dependency risk (`raid-log.md` D-1) this whole practice was
built around.

### 9. Missing sad paths that actually happen

Most stories carry one; these five do not, and the missing case is the likely one:

- **US6.1** — no criterion for a **partial corridor apply**. Applying across N
  streets is exactly where one target fails to prepare. Nothing says whether the
  operation is atomic. Add: *"Given a corridor apply where one target street
  cannot be prepared, When the apply completes, Then I am told which streets did
  not receive the design, and the streets that did are unaffected"* — and state
  whether it is all-or-nothing, because a test cannot assert both.
- **US5.1** — no criterion for an **invalid width** (negative, zero, absurd).
  This is a value a user types. `team-practices.md` requires editing operations
  to "return outcome plus findings"; nothing in the story set exercises that
  shape. Add: *"Given a width outside the accepted range, When I enter it, Then
  the edit is rejected with a stated reason and the previous value is retained."*
- **US8.3** — no criterion for a **failed upload**. US9.2 gets this right
  (AC9.2.3 covers a failed migration); US8.3 should mirror it exactly.
- **US3.2** — no criterion for a **coarse pass that returns nothing or fails**
  (ocean, unmapped area, request failure). Common, and currently undefined.
- **US11.1** — no criterion for a **partial deletion**. This gates public
  release; a deletion that half-completes and leaves orphaned designs is the
  failure that matters most in the group.

### 10. `traceability.json` — `OK` rows that claim more than the story delivers

- **`NFR1.1` → US3.1 `OK`** and **`NFR1.2` → US5.1 `OK`**. These claim coverage
  that AC3.1.2 and AC5.1.2 cannot deliver (§1). They should read `Deferred`,
  target `nfr-requirements`, exactly as `NFR1.3` and `NFR2.2` do. As they stand,
  a downstream stage will believe the performance budgets are discharged by
  acceptance criteria when no test can be written for either.
- **`NFR2.1` → US3.2 `OK`** rests on AC3.2.3 (§4). Fix the criterion or defer
  the row.
- **`NFR4.4` → US1.1 `OK`**. NFR4.4 requires "viewing **and editing** at
  viewport widths from 360 CSS pixels upward". AC1.1.3 tests the **landing
  page** at 360px. No criterion anywhere asserts the *editing surface* works at
  360px. This is the "works on phones" defect returning in traceability form —
  the requirement got its threshold, and the story set then tested it on the
  easiest page in the product. Add a criterion under US5.3: *"Given the editing
  surface at a viewport 360 CSS pixels wide, When I perform each action in the
  committed editing-action list, Then each completes and the page body does not
  scroll horizontally"*, and point NFR4.4 at both stories.
- **`NFR5.2` → "US3.1, US8.1" `OK`**. AC8.1.3 ("nothing about it is sent to a
  server") supports the US8.1 half. US3.1 has no criterion asserting the import
  runs in the browser — AC3.1.4 is a fixture test of the adapter and says
  nothing about where it executes. Add the mirror of AC8.1.3: *"When the full
  import runs, Then no request is made to the application's own server."* That
  criterion would also catch the budget-breaking half of `OQ1` — an OSM fetch
  proxied through Railway at $0.05/GB — which currently nothing would.
- **`FR2.3` → US7.3 `OK`**. FR2.3 is a global prohibition on writing to
  OpenStreetMap. AC7.3.5 asserts it only for the correction save path; the
  design edits (US5) and corridor application (US6) are not covered. Restate
  AC7.3.5 as a global invariant — *"no operation the product performs issues a
  write to OpenStreetMap"* — with the observable named (zero calls to any OSM
  write path), or the row over-claims.
- **`FR1.4` → US1.2 `OK`**. FR1.4 requires a prompt "when nothing is selected",
  unqualified. AC1.2.2's Given is *"I am in the tool for the first time"*. The
  deselect-later case is uncovered. Broaden the Given.
- **`FR4.4` → US5.3 `OK`** is technically correct but is backed by AC5.3.2,
  which is not executable (§4). An `OK` row backed by a non-executable criterion
  will not be caught by any test.

Two rows under-claim rather than over-claim, which is the safe direction, but
worth correcting: `FR3.3` should also name US11.2 (AC11.2.3 carries provenance
into the Stage 2 export), and `FR7.2` should also name US1.2 (AC1.2.3 carries
the "sign-in not required" clause).

The `Deferred` and `N/A` rows are otherwise sound. `NFR5.1`, `NFR6.1`, `NFR6.3`
and the whole `NFR7` group are correctly identified as practices rather than
user-observable behaviour, with real justifications rather than placeholders.
That is better discipline than I usually see at this stage.

### 11. The walking skeleton's three-part pass criterion

`team-practices.md` requires all three: correct lane count and order on import;
edit saved, reloaded, identical; mapped visibly distinguished from inferred.

- **Part 1 — partially covered.** AC3.1.4 covers lane count and order against a
  committed fixture, testably. But B-0's criterion is that the street "imports
  **through osm2streets**", and no criterion asserts the pinned crate is
  actually invoked (§8). B-0 can pass its lane-count test with a broken
  dependency build.
- **Part 2 — fully covered and well covered.** AC8.1.1 and AC8.1.2 together are
  the strongest pair in the draft. AC8.1.2 names its comparands (lane order,
  types, widths, every provenance state), so "identical" is executable rather
  than rhetorical. Every other "identical"/"unchanged" criterion in the set
  (AC9.3.2, AC2.3.2, AC9.2.2) should copy its wording.
- **Part 3 — covered only if Domain Design picks DOM.** AC4.1.2 is unbounded,
  AC4.1.4 is the meta/canvas-dependent one, and AC4.1.5 is testable only if an
  accessibility tree exists. B-0's criterion literally says "visibly
  distinguished… in what is **rendered**", and under a canvas choice there is no
  automated route to that at all without banned pixel comparison. This needs
  saying out loud in the story set, because B-0 gates every other Bolt: if canvas
  is chosen, the walking skeleton's third pass criterion falls to the manual
  walkthrough on the very first Bolt, indefinitely.

### 12. What is right, briefly

AC1.1.3 is exactly the shape the "works on phones" objection demanded — a
number, a viewport, and a body-scroll observable. AC3.1.4 and AC4.1.3 honour the
committed-fixture rule without reaching for a live fetch. AC2.3.2 (state
equivalence between the pointer and keyboard paths) and AC2.3.4 (focus not
stolen when the map view changes) are the two assertions that make the keyboard
path real rather than aspirational, and AC2.3.4 in particular is a failure mode
most story sets never think of. AC5.2.4 turns the overlay key-stability trap into
a test of the stored artifact. AC6.2.3 (the fit warning must say when the
comparison itself rests on an inferred width) carries provenance discipline into
the corridor logic, which nothing upstream demanded. AC9.1.3 (the access gate
enforced server-side, not by hiding the interface) and AC10.1.2/AC10.1.3
(object-level authorization asserted as a response, not a UI state) are the
checkable forms of the two controls that matter most. US8.2 as a whole is the
best story in the set — storage vanishing through a private window, cleared site
data, or eviction is exactly what happens, and it is the story most sets omit.

## Positions

AGREE: AC1.1.3 is the correct discharge of the "works on phones" objection — a stated viewport (360 CSS pixels) plus a body-horizontal-scroll observable, testable for free.
AGREE: AC3.1.4 and AC4.1.3 honour the committed-fixture rule properly — deterministic, no live OSM fetch at test time.
AGREE: AC8.1.2 names its comparands, making "identical" executable; AC9.3.2, AC9.2.2 and AC2.3.2 should copy its wording.
AGREE: AC2.3.2 and AC2.3.4 make the keyboard path testable rather than aspirational, and AC2.3.4 catches a failure mode most story sets omit.
AGREE: AC5.2.4 turns the overlay key-stability trap into an assertion against the stored design.
AGREE: AC6.2.3 carries provenance into the corridor fit check — a real criterion nothing upstream required.
AGREE: AC9.1.3, AC10.1.2 and AC10.1.3 state the access gate and object-level authorization as response assertions rather than UI states, which is the only form that can fail correctly.
AGREE: US8.2's sad paths are the ones that actually happen, and the story exists at all only because someone thought about it.
AGREE: the `Deferred` and `N/A` traceability rows for NFR1.3, NFR2.2, NFR3.x, NFR4.1, NFR4.6, NFR4.7, NFR5.1, NFR6.1, NFR6.3 and NFR7.1–7.3 are honest and correctly justified.
AGREE: Q4's width-only fit rule keeps the corridor check checkable and the core jurisdiction-neutral; AC6.2.5 records the right decision.

OBJECT: AC3.1.2 — "within 10 seconds" states no device, no network, no street and no measurement boundary, and OQ1 leaves the OSM fetch (plausibly the dominant term) unplaced; not testable as written.
OBJECT: AC5.1.2 — "within 100 milliseconds" states no device, no starting state, and does not say whether it is input-to-model or input-to-paint; replace with the synchronous-model-update assertion and send the figure to nfr-requirements with conditions.
OBJECT: `traceability.json` NFR1.1 → US3.1 `OK` and NFR1.2 → US5.1 `OK` — claim coverage that AC3.1.2 and AC5.1.2 cannot deliver; both should read `Deferred` to `nfr-requirements`, as NFR1.3 and NFR2.2 already do.
OBJECT: AC3.2.3 — no arrangeable starting state ("a city-scale network", "work continuously for a session") and no observable, while NFR2.2 itself says no figure exists; replace with the deterministic eviction-count assertion.
OBJECT: `traceability.json` NFR2.1 → US3.2 `OK` — rests entirely on AC3.2.3, so the row is only as good as a criterion nobody can test.
OBJECT: AC4.1.4 — asserts that an automated test can exist rather than asserting a system behaviour, and its truth depends on the deferred OQ2; under canvas the only automated route is pixel comparison, which is forbidden. Fold it into AC4.1.5 with the fixture binding.
OBJECT: AC12.2.3 — "printed", "read without colour" and "discernible" give no CI-executable assertion at any budget; restate as a non-colour-channel structural property and route the legibility judgement to the existing pre-release manual pass.
OBJECT: AC5.3.4 — testable and affordable under DOM (emulated touch plus bounding box), but untestable under canvas unless the hit-test region is queryable, and "at true scale" states no zoom; NFR4.5's own DOM/canvas sentence should be carried into the criterion.
OBJECT: AC4.2.3 and AC12.2.2 — the precondition "a view/medium where the marking cannot be rendered" cannot be constructed; restate as a property of the rendering path with marking unavailable.
OBJECT: AC5.3.2 — "When I inspect that action" is a review step, not an execution step; parameterise it and AC5.3.1 over a committed enumeration of editing actions so a new action without a keyboard path fails the build.
OBJECT: AC5.4.3 contradicts AC7.3.4 — reverting to "the imported baseline value with its original provenance" silently discards a correction that AC7.3.4 promises survives a full re-import; a test author must pick one.
OBJECT: AC1.2.3, AC4.2.2 and AC11.3.3 cannot be made to fail in the stage that implements them (no sign-in, no export, and data the service never held), so under tests-first they produce green-from-birth tests; mark each as binding from the stage that makes it falsifiable.
OBJECT: `traceability.json` NFR4.4 → US1.1 `OK` — NFR4.4 covers viewing *and editing* at 360 CSS pixels, and AC1.1.3 tests the landing page only; no criterion asserts the editing surface at 360px.
OBJECT: `traceability.json` NFR5.2 → US3.1 `OK` — no criterion in US3.1 asserts the import runs client-side; add the mirror of AC8.1.3, which would also catch OQ1's budget-breaking proxied-fetch case.
OBJECT: `traceability.json` FR2.3 → US7.3 `OK` — AC7.3.5 covers the correction save path only, not the design-edit or corridor-apply paths, so a global prohibition is discharged by one story's save.
OBJECT: `traceability.json` FR1.4 → US1.2 `OK` — AC1.2.2's Given is first-run only, while FR1.4 requires the prompt whenever nothing is selected.
OBJECT: no criterion asserts the pinned osm2streets crate is actually invoked to produce the committed fixtures, so the fixture suite cannot detect the dependency moving — which `team-practices.md` states is its entire purpose, and which B-0's exit is supposed to confirm.
OBJECT: the committed fixture set's one-way, cycleway and intersection fixtures are referenced by no criterion, and the failing-import fixture US7.1/US7.2/US7.4 depend on does not exist in the committed set at all.
OBJECT: US6.1 (partial corridor apply), US5.1 (invalid width), US8.3 (failed upload), US3.2 (empty or failed coarse pass) and US11.1 (partial deletion) carry no sad-path criterion, and in each case the missing case is the one that actually happens.
OBJECT: AC6.1.1 gives no definition of "connected streets", so the expected set is unknowable and the test author will invent it; bind it to the committed intersection fixture with a stated connectivity rule, or record it as an open question.
