# Domain Design — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `requirements.md` (requirements-analysis), `stories.md` and
`personas.md` (user-stories), `mockups.md`, `interaction-spec.md` and
`accessibility-checklist.md` (refined-mockups), `team-practices.md`
(practices-discovery).

## What is already settled and is not re-asked

The three inward-pointing layers are affirmed in `team.md` and are not a
decomposition question: the osm2streets adapter is its own crate and the only
place permitted to depend on osm2streets; the core street model and editing
operations are a separate crate with no dependency on osm2streets, no rendering
crate, no UI framework crate and no jurisdiction constants; the outer ring —
UI, persistence, jurisdiction packs, export — depends inward and is organised by
feature. Cargo's dependency graph is the enforcement mechanism, so an
inward-pointing violation is a compile error rather than a lint.

Also carried forward: the imported baseline is immutable and edits live in an
overlay; every dimension crossing the adapter boundary carries provenance at the
type level; metres are the only unit in the core; the overlay key is the OSM way
id plus the direction-normalised bounding node-id pair plus a project-owned lane
discriminator (amended at user-stories); errors at integration boundaries are
typed results, never panics.

**What decomposition options exist within the outer ring is not asked here.**
Per the stage contract, viable alternatives are presented in `components.md`
with their trade-offs and chosen at the approval gate.

---

## Q1. Five amendments were required before this stage, and none landed

`stories.md` carries a table headed "Amendments this stage requires of approved
artifacts", each row marked "before Domain Design". Checked just now, none has
happened:

| Required | State |
|---|---|
| `requirements.md` gains a requirement for undo | Absent — no mention of undo anywhere |
| `requirements.md` gains a requirement for device-storage loss | Absent |
| `requirements.md` gains a requirement for opt-in upload | Absent |
| FR9.3 amended to bind uploaded designs only | Unamended: "Anonymous designs shall expire 30 days after creation" |
| `scope-document.md` records Stage 1 server-side upload | Absent — no mention of upload |

This matters because `requirements.md` is what every later stage reads.
US5.5 (undo), US8.2 (storage loss) and US8.3 (upload) are Stage 1 Must Haves
that trace to no requirement, and FR9.3 as approved asserts an obligation over
data the product never holds.

- A. Proceed, and treat `stories.md` as the operative source where the two
  disagree. Record the divergence in `decisions.md` as an ADR so a later stage
  reads it rather than trusting `requirements.md` alone. Nothing is blocked;
  the inconsistency is documented rather than removed.
- B. Amend the two artifacts first, then run this stage. Both stages are
  approved and closed, so this means jumping back — `/aidlc --stage
  requirements-analysis` — and re-approving. Correct, and it costs two stage
  re-runs.
- C. Proceed, and additionally have this stage's `components.md` and
  `traceability.json` cover the three orphan stories explicitly, so the design
  is complete even though the requirements are not.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q2. Is the editing surface built from real page elements, or drawn on a canvas?

**This is the decision three stages have deferred to this one.** It is not a
rendering-technique preference; it decides what can be verified.

`team-practices.md` states the consequence plainly: Leptos, Yew and Dioxus all
render real DOM, so Rust/WASM does not force a canvas. If a canvas is chosen,
axe-core cannot see inside it, the lane strip's focus, names, states and
activation targets become unverifiable unless the canvas exposes an
accessibility tree *and* queryable hit-test regions, and **the walking
skeleton's third pass criterion — mapped visibly distinguished from inferred in
what is rendered — falls to a manual screen-reader walkthrough on the very first
Bolt, indefinitely.**

- A. Real page elements (DOM), via a Rust framework that renders them. Each lane
  is a focusable, announceable element; the accessibility floor stays
  automatable; AC4.1.4, AC4.1.5 and AC5.3.4 are all testable for free.
- B. Canvas, accepting manual-only accessibility verification of the editing
  surface and committing to expose an accessibility tree and queryable hit-test
  regions so the criteria are not simply unmet.
- C. DOM for the lane strip and its controls; canvas only for the map, which is
  a basemap library's concern anyway and carries no editing affordances.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q3. Where does OpenStreetMap data come from?

**Qualified by Q8.** The Railway path is chosen, but the egress it implies is
unmeasured; Q8 settles what happens about that.

`requirements.md` OQ1 records this as unsettled with a direct cost consequence,
and leaves it to Domain Design or Infrastructure Design. AC3.1.7 already asserts
that a full import issues no request to the application's own server — which
constrains but does not settle it.

- A. The browser fetches directly from a public OpenStreetMap API (Overpass or
  equivalent). Costs nothing against the $5 budget, and AC3.1.7 holds by
  construction. The product depends on a free public service's availability and
  rate limits, and must be polite about it.
- B. Through the Railway service, which fetches and caches. Egress bills at
  $0.05 per GB and AC3.1.7 would have to be rewritten. Gives control over rate
  limiting and caching; at city scale the bill is the risk OQ1 names.
- C. Browser-direct, with a small server-side cache only for extracts that have
  already failed or proved expensive. Two paths, and AC3.1.7 needs a stated
  exception.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q4. How does a design's lane change map onto a different street's lanes?

`stories.md` OQ-US6. Q13 at user-stories settled that a corridor apply is
per-lane mapping rather than wholesale replacement — the target keeps what the
design does not speak to. The rule that performs the matching was left to this
stage, and AC6.1.3 requires the product to say which lanes it could not match.

Elm has six lanes; you widened its bike lane. Oak has four, arranged
differently. Which of Oak's lanes receives the change?

- A. Match by lane type and ordinal from the kerb — the design's second bike
  lane from the left maps onto the target's second bike lane from the left.
  Unmatched changes are reported. Simple, explicable in one sentence to a user.
- B. Match by role within the cross-section — the design's changes are
  expressed as intent ("the bike lane nearest the kerb becomes 2.5 m") and
  applied to whatever fills that role. More forgiving on dissimilar streets;
  needs a role vocabulary the core model does not have yet.
- C. Match by type only, applying a change to every lane of that type on the
  target. Predictable and blunt; widens two bike lanes when the user meant one.
- D. Not yet defined — record as an open question for Functional Design.
- X. Other (please specify)

[Answer]: A

## Q5. What identifies an uploaded anonymous design?

AC8.3.5 requires an identifier carrying no name, no contact details and no
cross-site linkage. `refined-mockups` explicitly declined to specify it and
named this stage. Q9 at user-stories shrank this surface deliberately: only
designs the user explicitly uploads are affected.

- A. A random identifier minted in the browser at upload, stored with the design
  and held in device storage so the user can reach it again. The service never
  sees anything else; there is no account, no cookie, and no way to link two
  uploads by the same person.
- B. That, plus a server-issued edit token returned once at upload, without
  which the design cannot be deleted or replaced. Protects against someone
  guessing an identifier; the token is another secret the user can lose.
- C. Not yet defined — leave to Infrastructure Design.
- X. Other (please specify)

[Answer]: A

## Q6. Does someone you share a design with need an account to view it?

`stories.md` OQ-US5. This lands on P2, the city planner, whose stated barrier is
being unwilling to sign up for anything to read one proposal — and P2 is the
persona whose acceptance is the product's differentiator. It also decides
whether US10.2's named-access mechanism can exist without accounts on the
viewing side.

- A. No account to view. A link or a named grant opens the design read-only for
  anyone holding it. Named access is then identified by email at the point of
  sharing, and the recipient follows a link rather than signing in.
- B. An account to view anything granted by name; the public link needs none.
  Named access becomes real authorisation rather than a shared secret, at the
  cost of putting a sign-up in front of the persona the product must win.
- C. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q7. How are the three layers stored?

The model has three layers — the imported baseline, corrections to it (US7.3),
and the proposed design (US5). `team.md` requires the baseline to be immutable
and edits to live in an overlay. What it does not say is whether corrections and
design edits are one overlay or two, and US7.5's fingerprint behaviour depends
on the answer.

- A. Two separate overlays. Corrections carry the import fingerprint and the
  three re-import outcomes (AC7.5.2–AC7.5.4); design edits carry none, because a
  design edit is a proposal and does not claim to describe reality. Clean
  separation; two things to key and merge.
- B. One overlay with a per-entry layer tag. Simpler storage and one key scheme;
  the fingerprint logic then has to filter by tag, and a bug that mislabels an
  entry silently converts a proposal into a claim about the street.
- C. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q8. Follow-up — routing OSM data through Railway crosses two standing lines

Q3 chose the Railway path. Two things follow that are larger than the option
text stated, and both need settling rather than recording.

**It contradicts an approved acceptance criterion.** AC3.1.7 reads: "*Given* a
full import, *When* it runs, *Then* no request is made to the application's own
server." That is a Must Have criterion of US3.1, approved at the user-stories
gate. Under Q3 it is false by design, so it must be rewritten rather than
quietly failed — and `requirements.md` NFR5.2, which AC3.1.7 was written to make
observable, needs the same treatment.

**It is a constraint change, and `project.md` requires that to be justified
rather than absorbed.** The affirmed prohibition reads: "NEVER let
hosting/tooling spend grow past the stated ~$5/month working budget without
treating that growth as a constraint change requiring explicit justification,
not something silently absorbed." `requirements.md` OQ1 names this exact path as
"the difference between a negligible and a budget-breaking bill" at the city
scale NFR2.1 sets.

The honest position is that nobody has measured what an OSM extract for one
street costs in egress, let alone a viewport at city scale. So the question is
what bounds it.

- A. Proceed, with a hard cost ceiling designed in: the service caches
  aggressively, refuses to proxy beyond a stated monthly egress budget, and
  falls back to browser-direct fetching when that ceiling is reached. The
  ceiling is a first-class requirement, not an alarm. AC3.1.7 and NFR5.2 are
  rewritten to match, and the justification is recorded as an ADR.
- B. Proceed and measure first: the walking skeleton (B-0) records the egress
  cost of one real street import, and the decision is revisited at
  Infrastructure Design with a real number rather than a guess. Until then the
  proxy is built but the budget question stays open.
- C. Revisit Q3 — browser-direct with a server-side cache only for extracts that
  already failed or proved expensive (Q3's option B). Keeps AC3.1.7 true for the
  common path with one stated exception, and keeps the egress bounded by
  construction rather than by a ceiling that has to hold.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Consolidated Summary Confirmation

**Your answers**

- **The unlanded amendments**: proceed, and have `components.md` and
  `traceability.json` cover the three orphan stories explicitly, with the
  divergence recorded as an ADR (Q1).
- **Editing surface**: real page elements for the lane strip and its controls;
  canvas only for the map (Q2).
- **OSM data**: fetched through the Railway service (Q3), with the proxy built
  and its real egress measured at B-0, then revisited at Infrastructure Design
  with a number rather than a guess (Q8).
- **Corridor matching**: by lane type and ordinal from the kerb; unmatched
  changes reported (Q4).
- **Anonymous upload identity**: a random identifier minted in the browser, no
  edit token (Q5).
- **Viewing a shared design**: a public link needs no account; a named grant
  requires the viewer to sign in (Q6).
- **The three layers**: two separate overlays — corrections carry the import
  fingerprint, design edits do not (Q7).

**Seven things I will write that follow from your answers rather than being
stated by them**

- **The OSM proxy must log nothing that pairs an address with a location.**
  This is the one that would undo a decision already taken. Q9 at user-stories
  chose browser-first storage specifically so Stage 1's default path holds
  nothing about anybody, which is what keeps `scope-document.md`'s "Stage 1
  carries no obligation" true. A proxy that fetches extracts on a user's behalf
  necessarily sees their IP address and the place they are looking at, and a
  request log pairing those two is location data about an identifiable device.
  The design will state that the proxy logs neither, and that its cache is keyed
  on the extract rather than on the requester — otherwise Q3 quietly reopens
  exactly what Q9 closed, on a different surface.
- **Two more amendments join the list.** AC3.1.7 ("no request is made to the
  application's own server") is now false by design, and NFR5.2 — which AC3.1.7
  was written to make observable — needs the same treatment. They go into the
  same ADR as Q1's divergence, so there is one place recording where the
  approved artifacts and the real design differ.
- **Q2 closes two open questions and unblocks the walking skeleton.** OQ2 and
  OQ-US7 are answered: because the lane strip is real elements, its focus,
  names, states and activation targets are all automatable, AC5.3.4's
  canvas condition falls away, and **B-0's third pass criterion — mapped
  visibly distinguished from inferred — can be tested from the first Bolt
  rather than falling to a manual walkthrough forever.** Every
  canvas-conditional row in `accessibility-checklist.md` resolves to its DOM
  branch.
- **The map being canvas keeps one obligation alive.** `accessibility-checklist.md`
  M5 requires every map-only affordance to have a non-map equivalent. That now
  binds rather than being precautionary: the unresolved-correction marker (Q4 at
  refined-mockups), the current-design highlight and the post-apply fit status
  are all drawn in a canvas nothing can inspect, so each needs its twin in the
  document flow. Two already exist; the design will name the third.
- **The upload identifier is the only protection, so it has to be unguessable.**
  Q5 declines an edit token, which means anyone holding the identifier can
  delete the design. That is acceptable only if the identifier cannot be found
  by trying: a 128-bit random value from a cryptographic source, never derived
  from anything about the design or the device, and a rate limit on the delete
  path so guessing is not a viable strategy.
- **Q6 puts an account system on the viewing side of Stage 2.** Named access
  requiring sign-in means the recipient of a named grant needs an account before
  they can read anything, so US10.2's named path depends on US9.1 in a way the
  story graph does not currently draw. The public-link path stays account-free,
  which is what keeps P2 reachable.
- **"Ordinal from the kerb" needs the definition AC6.2.1 already owns.**
  osm2streets returns no kerb-to-kerb width, and `mockups.md` records a
  project-owned carriageway definition for the fit check. Q4's matching rule
  counts from the same edge, so it reuses that definition rather than inventing
  a second one — two definitions of "the kerb" in one product is a bug waiting
  for a thin street.

**Two things I will record as open rather than answer**

- **What the proxy actually costs.** Q8 defers this to a real measurement at
  B-0. Until that number exists, the design states the proxy exists and states
  that its cost is unquantified — it does not assert the $5 budget holds.
- **Whether the proxy needs its own rate limiting toward the upstream public
  API.** A server fetching on behalf of many browsers looks different to a
  public OpenStreetMap endpoint than many browsers fetching individually, and
  being a good citizen of a free service is a design obligation. Sizing that
  needs the same measurement Q8 defers.

Does this all look correct before I generate the component catalogue, the ADR
log and the traceability?

- Looks correct
- Request changes

[Answer]: Looks correct
