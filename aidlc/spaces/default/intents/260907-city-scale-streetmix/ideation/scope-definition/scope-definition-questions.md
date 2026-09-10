# Scope Definition — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `intent-statement.md` (intent-capture); `feasibility-assessment.md`
and `constraint-register.md` (feasibility).

The candidate capabilities referenced below are drawn from what the approved
upstream artifacts already establish as needed. They are proposals for you to cut
down, not decisions.

| Ref | Capability | Where it comes from |
|-----|-----------|---------------------|
| C1 | Map view — find a place, see its street network | Problem statement: corridors and networks on a real map |
| C2 | Street import — turn a real street into an editable cross-section via osm2streets | `feasibility-assessment.md`: the central technical problem |
| C3 | Cross-section editor — change the lanes of a selected street | Table-stakes per market research Q3 |
| C4 | Corridor and network editing — work across connected streets, not one at a time | Problem statement (intent capture Q1) |
| C5 | Accounts — sign in, own your work | Feasibility Q6 |
| C6 | Save and share — persistent designs with shareable links | Feasibility Q6 |
| C7 | Meeting-ready output — exports, printable plans, presentation views | Intent capture Q7; the differentiator per market research Q3 |
| C8 | City GIS import and export | Feasibility Q7 |
| C9 | Pluggable jurisdiction standards | Feasibility Q8 |
| C10 | Privacy rights — data export and erasure for account holders | `constraint-register.md` RC-1 |

---

## Q1. Which core design capabilities must be in the first release? (select all that apply)

These four are the design surface itself. Anything not selected is not excluded —
it moves later in the backlog.

- A. C1 — map view: find a place and see its street network.
- B. C2 — street import: turn a real street into an editable cross-section.
- C. C3 — cross-section editor: change the lanes of a selected street.
- D. C4 — corridor and network editing: work across connected streets.
- X. Other (please specify)

[Answer]: A, B, C, D

## Q2. Which supporting capabilities must be in the first release? (select all that apply)

These surround the design surface. C7 deserves particular thought: it is your
stated differentiator (market research Q3), which is an argument for including it
early, but it is also the last step of a workflow, which is an argument for it
following the rest.

- A. C5 and C6 — accounts, saved designs, shareable links.
- B. C7 — meeting-ready output: exports, printable plans, presentation views.
- C. C10 — privacy rights: data export and erasure for account holders.
- D. None of these in the first release — keep it to the design surface only.
- X. Other (please specify)

[Answer]: A, B

## Q3. What is the thinnest end-to-end slice that would prove the approach works?

Your org practice runs a walking-skeleton first when the scope calls for it: a
minimal end-to-end implementation that proves the approach before features are
added. `raid-log.md` records R-1 — that osm2streets may not integrate as expected
— as the top risk, with "build and run osm2streets against a real area" as the
first technical task.

- A. Exactly that risk: pick one real street, run it through osm2streets, and
  render its lanes. No editing, no saving, no map browsing.
- B. One street, imported and editable — add the cross-section editor so the
  round trip from real data to changed design is proven.
- C. One street, imported, edited and saved — prove persistence too, since
  accounts and storage carry their own unknowns.
- D. No skeleton — go straight at the first release scope.
- X. Other (please specify)

[Answer]: C

## Q4. How should the remaining work be sequenced?

`workflow-planning-guide.md` names four heuristics. Your situation — one person,
no deadline, one dominant technical unknown — argues for one of them, but the
choice is a value judgment rather than something the artifacts decide.

- A. Risk-first — sequence the highest-uncertainty work early so later decisions
  are calibrated before anything depends on them.
- B. Value-first — ship in order of user value; risk is low enough to absorb.
- C. Dependency-first — follow what technically enables what, and let value fall
  out of that order.
- D. Not yet defined — record the choice as open and settle it at Delivery
  Planning.
- X. Other (please specify)

[Answer]: B

## Q5. Are shared design links public, or access-controlled?

`raid-log.md` records this as assumption A-7 and notes it must be decided before
accounts ship, because it changes what personal data is disclosed and to whom.
Your audience is advocates and the public, and the point of a design is often to
show it to people.

- A. Public by default — anyone with the link can view; that is the point.
- B. Access-controlled by default — the owner chooses who can see each design.
- C. Public, but the owner can make a design private.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q6. Are planning and engineering consultants a stakeholder, or only a customer group?

`raid-log.md` carries this as open issue I-3, inherited from intent capture: you
named consultants as a customer group (intent capture Q2) but not among the
stakeholders (Q5 there). Settling it decides whether their needs shape scope
decisions or merely benefit from them.

- A. Customer group only — they may use it, but their needs do not shape scope.
- B. Stakeholder too — their needs should influence what gets built.
- C. Neither, on reflection — drop them from the customer set as well.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q7. What is explicitly out of scope? (select all that apply)

The most valuable prioritisation category is the one that says "not this time".
Naming exclusions now prevents them arriving later as assumed requirements.

- A. Traffic simulation and modelling — leave that to the tools that already do
  it well.
- B. Construction-grade or engineering-grade output — this produces proposals,
  not buildable drawings.
- C. Editing the underlying OpenStreetMap data — designs are proposals over the
  map, never changes to it.
- D. Nothing yet — I would rather not fix exclusions at this stage.
- X. Other (please specify)

[Answer]: A, B, C

## Q8. Follow-up — accounts in the first release, but not privacy rights

Q2 put accounts, saved designs and shareable links in the first release (A) and
left privacy rights out (C not selected). That conflicts with an approved
constraint. `constraint-register.md` RC-1 states that accounts and saved designs
tied to people place this initiative in scope for GDPR and comparable regimes
from the first public release, and that data subject rights — access,
rectification, erasure, portability — are functional requirements rather than
operational work.

Access-controlled sharing (Q5 B) reduces what is disclosed to others, but it does
not change an account holder's own rights over their data.

- A. Include the minimum — account deletion that actually erases, and a design
  export. Enough to satisfy the rights that matter most, without a full rights
  console.
- B. Include the full set — access, rectification, erasure and portability, built
  properly the first time.
- C. Defer them, and don't release publicly until they exist — the first release
  is for you and invited testers only, so the obligation does not yet bite.
- D. Defer them and accept the exposure — ship publicly without them and treat it
  as a known risk.
- X. Other (please specify)

[Answer]: C

## Q9. Follow-up — is this one release, or several?

Between Q1 and Q2 the first release now contains six of the ten candidate
capabilities: map view, street import, cross-section editor, corridor and network
editing, accounts with sharing, and meeting-ready output. That includes the two
largest — C4, which is the network model itself, and C7, which is a workstream of
its own.

The stage asks for the minimum viable scope that delivers value, and this is
closer to the whole product. With one person building (feasibility Q3) and no
deadline (Q4 there), nothing forces a cut — but a first release that contains
everything defers all feedback until everything is done.

This question is about how the six are labelled, not about dropping any of them.

- A. One release, all six — they hang together and a partial version is not worth
  showing anyone.
- B. Two stages — a usable core first (map, import, editor, corridor), then
  accounts, sharing and output as a second release once the core is proven with
  real use.
- C. Three stages — core, then sharing and accounts, then meeting-ready output as
  the differentiator lands last on a proven base.
- D. Keep all six as must-have and decide the staging at Delivery Planning
  instead.
- X. Other (please specify)

[Answer]: C

## Consolidated Summary Confirmation

Your answers:

- First-release core: all four design capabilities — map view, street import,
  cross-section editor, corridor and network editing (Q1 A, B, C, D).
- First-release supporting: accounts with saved designs and shareable links, and
  meeting-ready output (Q2 A, B). Privacy rights were not selected.
- Walking skeleton: one street imported, edited and saved (Q3 C).
- Sequencing: value-first (Q4 B).
- Shared links: access-controlled by default; the owner chooses who sees each
  design (Q5 B).
- Consultants: a customer group only; their needs do not shape scope (Q6 A).
- Out of scope: traffic simulation and modelling; construction- or
  engineering-grade output; editing the underlying OpenStreetMap data (Q7 A, B, C).
- Privacy rights: deferred, with public release withheld until they exist — the
  first release is for you and invited testers only (Q8 C).
- Staging: three stages — core, then sharing and accounts, then meeting-ready
  output (Q9 C).

How I am reading two of these, so you can correct me:

- The skeleton's "save" (Q3 C) means persistence without accounts, since accounts
  belong to stage 2 (Q9 C). It proves the storage unknown without pulling
  identity into stage 1.
- The skeleton attacks the top technical risk while sequencing is value-first
  (Q4 B). Those are consistent: the skeleton exists to retire that one unknown,
  and value-first governs everything after it.

What the artifacts will say:

- Three delivery stages. Stage 1: map view, street import, cross-section editor,
  corridor and network editing. Stage 2: accounts, saved designs, access-controlled
  sharing. Stage 3: meeting-ready output.
- Privacy rights become a **release gate on public availability** rather than a
  deferred capability. The gate attaches to stage 2, because that is where
  accounts land; stage 1 carries no accounts and therefore no obligation. Stage 2
  may be built and used by you and invited testers before the gate is met, and may
  not be opened to the public until it is.
- City GIS import/export and pluggable jurisdiction standards are Should Have,
  after the three stages. Neither was selected as must-have, and neither is
  excluded.
- The three exclusions are recorded as Won't Have this time, not as permanent
  decisions.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
