# Feasibility & Constraints — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `intent-statement.md` (intent-capture); `competitive-analysis.md`,
`market-trends.md`, `build-vs-buy.md` (market-research).

---

## Q1. Which foundation should this be assessed against?

At market research you said the build-vs-extend question was genuinely open and
should be settled by evidence (Q5 there). `build-vs-buy.md` then recommended
carrying three candidates into this stage, because A/B Street turned up as a
third option that is already network-scale and map-grounded — the two things
extending Streetmix would have to add. This question decides what this stage
actually assesses.

- A. All three — assess extending Streetmix, building on A/B Street, and building
  new, then recommend one.
- B. Streetmix vs. new build only — A/B Street's slowed development makes it too
  risky to depend on.
- C. One in particular — you already have a preference and want the assessment
  focused on it.
- D. Not yet defined — assess the general technical viability without committing
  to a foundation.
- X. Other (please specify)

[Answer]: A

## Q2. Which geography and street-design standards apply?

`intent-statement.md` records the absence of any geographic or
street-design-standards context as an open assumption, and `market-trends.md`
found that all the funding and policy evidence available was United
States-specific. Street design standards are jurisdictional, so this determines
what "a design a city will accept" (your differentiator) actually means.

- A. United States — design to US practice and standards.
- B. United Kingdom or Europe — design to that practice instead.
- C. Deliberately jurisdiction-neutral — the tool should not encode any one
  country's standards.
- D. Not yet defined — leave this open and treat it as a risk.
- X. Other (please specify)

[Answer]: C

## Q3. Who is building this, and with what skills?

Feasibility depends heavily on who is available. Your intent statement records
you as the only stakeholder today with sole decision authority.

- A. You alone, with AI assistance — no other developers.
- B. You plus a small number of collaborators.
- C. A funded team to be assembled.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q4. What are the budget and timeline constraints?

Your intent statement records no external deadline and the trigger as a gap in
existing tools. This question is about what limits actually bind.

- A. No deadline; minimal budget — hosting and tooling costs need to stay near
  zero.
- B. No deadline, but real budget available for infrastructure and services.
- C. There is a target date you want to work back from.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q5. Where would this run?

The intended model is open source and free (market research Q7), which makes
running cost a design constraint rather than an afterthought.

- A. Not decided — assess the options as part of feasibility.
- B. A cloud provider account you already have (AWS, or another).
- C. Deliberately cheap or free hosting — static hosting, a free tier, or
  self-hosted.
- D. Not applicable yet — this is too early to matter.
- X. Other (please specify)

[Answer]: X. Railway

## Q6. Does this handle personal data or user accounts?

This determines whether privacy regulation (GDPR, CCPA and similar) is in scope
at all. Your audience is advocates and the public (market research Q6), so any
account or sharing feature touches ordinary members of the public.

- A. Yes — user accounts, saved designs, and shared links tied to people.
- B. Minimal — designs can be saved and shared, but without accounts or
  identifying data.
- C. No — everything is anonymous and nothing personal is stored.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q7. What existing systems or data must this work with? (select all that apply)

Integration constraints are usually where feasibility assessments find their real
risks. Your problem statement requires designing on a real map.

- A. OpenStreetMap — street network and geometry as the base data.
- B. Commercial or public basemap and imagery services.
- C. City GIS systems — importing a city's own street data or exporting back to
  it.
- D. Nothing external yet — assess what is needed rather than starting from a
  fixed list.
- X. Other (please specify)

[Answer]: A, C

## Q8. Follow-up — jurisdiction-neutral, but the differentiator is city acceptance

You answered Q2 as deliberately jurisdiction-neutral: the tool should not encode
any one country's street-design standards. Your differentiator, set at market
research (Q3 there), is producing outputs a city will actually accept in a formal
process. Acceptance is jurisdictional by nature — a US city and a UK council
judge a street design against different standards. These pull in opposite
directions, so this decides how the tool reconciles them.

- A. Pluggable standards — the tool is neutral at its core and standards are
  supplied per jurisdiction, so a community can add its own.
- B. Neutral output, human judgement — the tool produces credible, legible output
  and does not attempt to certify compliance with any standard.
- C. Neutral now, one jurisdiction later — start neutral and pick a first
  jurisdiction to support properly once there is a real user.
- D. Not yet defined — record the tension as a risk and decide later.
- X. Other (please specify)

[Answer]: A

## Q9. Follow-up — accounts and Railway against a near-zero budget

Three of your answers pull against each other on cost. Q4 says minimal budget,
with hosting and tooling near zero. Q5 names Railway. Q6 says user accounts,
saved designs, and shared links tied to people, which needs a database and a
backend that is reachable whenever someone opens a link.

Railway's published pricing: a $5 one-time trial credit over 30 days; a Free tier
at $0/month with $1 of monthly credit; Hobby at $5/month including $5 of usage;
then usage-based charges of roughly $10 per GB of memory per month, $20 per vCPU
per month, and $0.05 per GB of egress. Billing is per-second, so stopped services
cost nothing.

A backend plus a database running continuously will exceed the $1 Free credit.
This question sets what "near zero" actually means.

- A. Hobby is fine — about $5 a month is near zero for this purpose, and that is
  the working budget.
- B. Genuinely zero — the design must fit the free tier, which likely means
  deferring accounts or finding a way to avoid an always-on backend.
- C. Accounts matter more than the budget — accept whatever Railway costs once
  there are real users, and treat funding it as a later problem.
- D. Not yet defined — record the tension as a risk and decide when there is a
  real usage estimate.
- X. Other (please specify)

[Answer]: A

## Q10. Follow-up — extend Streetmix, or build against osm2streets?

You asked why the recommendation leaned toward extending Streetmix. On
re-examination the case is weaker than it was stated, because osm2streets changes
what Streetmix is actually contributing.

The original reasoning was that the section-editing experience is table-stakes
(market research Q3) and Streetmix is a mature implementation of it, so rebuilding
it buys no differentiation. But osm2streets supplies the lane *schema* — type,
direction and width, ordered left-to-right — which is the hard part of the
cross-section model. What Streetmix still contributes is the drag-and-drop
editing UI and its segment artwork.

Three points that were under-weighted:

1. Streetmix's README states its scope as individual street sections, "not
   broader networks or maps". Adding a map, a network model, corridors and
   pluggable standards does not extend that core; it inverts it. The network
   model becomes the centre and the section editor becomes one view within it.
2. Extending inherits AGPL-3.0-or-later permanently. That is compatible with the
   open-source-free model chosen at market research (Q7 there), but it forecloses
   open core and makes the licence a consequence of Streetmix LLC's decision
   rather than this project's.
3. Extending means reconciling two schemas — Streetmix's segment model and
   osm2streets' lane schema. Building means one schema with an editor over it.

- A. Extend Streetmix — the editing UI and segment artwork are worth the AGPL
  inheritance and the schema reconciliation.
- B. Build new against the osm2streets schema — borrow Streetmix's interaction
  design rather than forking its code; keep the licence and the data model this
  project's own.
- C. Prototype before deciding — the assessment records both as live and names
  the experiment that would settle it.
- D. Not yet defined — record the decision as open and carry both into Scope
  Definition.
- X. Other (please specify)

[Answer]: B

## Consolidated Summary Confirmation

Your answers:

- Foundations assessed: all three — extend Streetmix, build on A/B Street, build
  new (Q1 A).
- Foundation decision: build new against the osm2streets schema, borrowing
  Streetmix's interaction design rather than forking its code (Q10 B).
- Geography and standards: deliberately jurisdiction-neutral (Q2 C), reconciled
  with the city-acceptance differentiator through pluggable per-jurisdiction
  standards over a neutral core (Q8 A).
- Team: you alone, with AI assistance (Q3 A).
- Budget and timeline: no deadline, minimal budget (Q4 A), with roughly $5 a
  month on Railway's Hobby tier as the working figure (Q9 A).
- Hosting: Railway (Q5 X).
- Personal data: yes — user accounts, saved designs, and shared links tied to
  people (Q6 A).
- Integration: OpenStreetMap as base data via osm2streets, and city GIS
  import/export (Q7 A, C).

What was verified at source:

- Streetmix: TypeScript, last pushed 2026-09-04, actively maintained. Repository
  licence metadata reads NOASSERTION; its LICENSE file states
  AGPL-3.0-or-later, relicensed from BSD 3-clause.
- A/B Street (the application): Rust, Apache-2.0, 8,166 stars, last pushed
  2025-09-10. Not archived, but dormant.
- osm2streets: Apache-2.0, from the A/B Street organisation, last pushed
  2025-10-02. Its stated purpose is that OSM "presents many challenges for
  rendering, routing, and analyzing done at the detail of lanes"; its schema gives
  each road a list of lanes left-to-right with type, direction and width, plus
  intersection polygons, dual-carriageway merging, cycletrack snapping and GeoJSON
  rendering. The repository carries `osm2streets-js` and `osm2lanes-js` bindings
  and a `release_npm.sh`; the core is Rust compiled to WebAssembly.
- Published npm packages lag the repository: `osm2streets-js` 0.1.4 (June 2023)
  and `osm2lanes-js` 0.1.0 (October 2023), both Apache-2.0.
- Railway pricing: Free at $0/month with $1 monthly credit; Hobby at $5/month
  including $5 of usage; beyond that roughly $10 per GB memory per month, $20 per
  vCPU per month, $0.05 per GB egress, billed per second.
- OpenStreetMap's own wiki defines `width` as carriageway kerb-to-kerb, excluding
  sidewalks, and sidewalk width is largely unmapped with several competing tagging
  schemes in use.

What the artifacts will say:

- The recommendation is to build new against the osm2streets lane schema, with
  Streetmix as an interaction-design reference rather than a code dependency. The
  licence stays this project's own and there is one schema rather than two.
- A/B Street the application is assessed and set aside as dormant; the A/B Street
  ecosystem is adopted through osm2streets.
- The top technical risk is osm2streets integration and inference quality: the
  published npm packages are roughly three years old, the WebAssembly build path
  may need building from source, the project itself has been quiet for about
  eleven months, and where OSM tagging is thin the derived cross-sections rest on
  inferred defaults rather than mapped values.
- Accounts plus jurisdiction-neutral reach put GDPR and comparable privacy
  regimes in scope from the first release rather than later.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
