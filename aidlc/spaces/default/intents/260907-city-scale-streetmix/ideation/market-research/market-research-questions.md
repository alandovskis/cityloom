# Market Research — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream input: `intent-statement.md` from the intent-capture stage.

---

## Q1. How should I establish the competitive landscape?

An ideation-phase rule requires market research claims to carry citations or
explicit source attribution, so this determines whether the artifacts cite real
sources or record your own assessment as the stated source.

- A. Research it — I look up the current landscape on the web and cite sources
  for each claim.
- B. Work from what you know — you tell me the players and I attribute the
  claims to you.
- C. Both — I research, and you correct or add to what I find.
- D. Not yet defined — keep this light and flag the whole landscape as
  unvalidated.
- X. Other (please specify)

[Answer]: A

## Q2. Which products or approaches should the comparison cover? (select all that apply)

The comparison set determines what "the gap above the cross-section" is measured
against. Your intent statement says existing tools stop at the single
cross-section (Q4 of intent capture).

- A. Streetmix itself — the baseline this initiative scales up from.
- B. Professional CAD and GIS tooling used by cities today (the tooling your
  intent statement calls too slow and too specialised for early-stage work).
- C. Public-participation and community-engagement platforms — the tools
  advocates and cities use to gather input on street changes.
- D. Simulation and analysis tools — traffic modelling, safety analysis, network
  performance.
- X. Other (please specify)

[Answer]: A, B, C, D

## Q3. What is table-stakes versus what would actually differentiate this?

Table-stakes is what a user assumes without noticing; a differentiator is the
reason they switch. Getting this split right determines what has to be in the
first build.

- A. Table-stakes is the Streetmix editing experience; the differentiator is
  working across a whole network on a real map.
- B. Table-stakes is map and network handling (any serious tool has it); the
  differentiator is the speed and approachability of the design experience.
- C. Table-stakes is both of those; the differentiator is producing outputs a
  city will actually accept in a formal process.
- D. Not yet defined — I'd rather work this out after seeing the comparison.
- X. Other (please specify)

[Answer]: C

## Q4. Which trends or shifts make this timely? (select all that apply)

Your intent statement records the trigger as a gap in existing tools rather than
an external deadline. This question is about the wider currents that make the gap
worth filling now.

- A. Open data and basemaps — street network and imagery data are now broadly
  available for many cities.
- B. Policy and funding shifts toward street redesign — safety programmes,
  climate targets, active-travel investment.
- C. Rising expectation of public participation in infrastructure decisions.
- D. Not identified — I don't want to claim a trend I can't support.
- X. Other (please specify)

[Answer]: A, B, C

## Q5. Build, buy, or extend the existing Streetmix codebase?

Your intent statement left this deliberately open: the product boundary was
confirmed as one substantial build "whether that lands as a new product or as an
extension of the existing Streetmix codebase". This stage is where that gets
examined rather than assumed. This question records your current leaning, not a
final commitment — Feasibility will test it.

- A. Extend Streetmix — build on the existing open-source codebase and its
  segment model.
- B. Build new — a fresh product that borrows the interaction idea but not the
  code.
- C. Genuinely open — assess both against real criteria and let the evidence
  decide.
- D. Not yet defined — defer this entirely to Feasibility.
- X. Other (please specify)

[Answer]: C

## Q6. What addressable audience are you targeting?

This sizes the opportunity. Your intent statement names city staff, advocates and
the public, and consultants as customer groups; this question is about how many
of them are actually reachable.

- A. A specific country or region's cities — a bounded, countable set.
- B. Cities globally, wherever open street data is available.
- C. Not primarily cities — the reachable audience is advocates and the public,
  who are far more numerous and easier to reach.
- D. Not yet defined — sizing isn't something you want to claim at this stage.
- X. Other (please specify)

[Answer]: C

## Q7. What commercial or sustainability model do you have in mind?

Competitors' pricing models only mean something relative to your own. This also
affects who the product has to satisfy.

- A. Open source, free — sustained by grants, sponsorship, or goodwill, like
  Streetmix itself.
- B. Commercial — subscriptions or licences sold to cities or consultants.
- C. Open core — free for the public, paid for institutional features.
- D. Not yet defined — this is a product question you'd rather settle later.
- X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

Your answers:

- Research approach: I research the landscape and cite sources for each claim
  (Q1 A).
- Comparison set: Streetmix itself, professional CAD and GIS tooling, public
  participation and engagement platforms, and simulation and analysis tools
  (Q2 A, B, C, D).
- Table-stakes vs differentiator: the editing experience and map/network
  handling are both table-stakes; the differentiator is producing outputs a city
  will actually accept in a formal process (Q3 C).
- Relevant trends: open data and basemaps; policy and funding shifts toward
  street redesign; rising expectation of public participation (Q4 A, B, C).
- Build vs extend: genuinely open — assess both against real criteria and let the
  evidence decide (Q5 C).
- Addressable audience: not primarily cities — advocates and the public, who are
  more numerous and easier to reach (Q6 C).
- Commercial model: open source and free, sustained by grants, sponsorship or
  goodwill (Q7 A).

What the research turned up, and what will go into the artifacts:

- Streetmix is licensed AGPL-3.0-or-later, relicensed from BSD 3-clause. This is
  the single most consequential finding for the build-vs-extend decision.
- Streetmix's own README states the project focuses on individual street
  cross-sections, not networks or maps — the gap your intent statement names is
  confirmed by the incumbent's own scope statement.
- Streetmix is already used by NYC DOT, Transport for London, SFMTA, and firms
  including Nelson\Nygaard and Kimley-Horn.
- Two Streetmix-derived alternatives exist: StreetDesign.ai (Beyond CAD) and
  StreetPlan.net (Urban Innovators). Neither is a network-scale tool by the
  descriptions found.
- Remix Streets (now under Via) is the commercial incumbent for corridor-level
  street design decisions.
- A/B Street is the closest open-source network-scale tool, built on
  OpenStreetMap; its own docs record that development slowed as of January 2024.
- Konveio and Social Pinpoint are the engagement-platform incumbents; both are
  document- and comment-centred rather than design-centred.
- SUMO and Aimsun cover simulation; SUMO is open source, Aimsun is commercial.

How I read one tension in your answers: Q3 points at city-institutional output
while Q6 points away from cities as the audience. I read these as coherent rather
than contradictory — your intent statement records the advocate pain as having no
credible way to propose a street change a city will take seriously, so
city-acceptable output is precisely what that audience needs. Say so if that
reading is wrong.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
