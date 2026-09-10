# Feasibility Assessment — Streetmix at City Scale

## What Was Assessed

`intent-statement.md` sets the problem as designing corridors and networks on a
real map, with the success measure that a corridor redesign taking about a week
of planner work today should take under a day. `build-vs-buy.md` ruled buying out
and recommended carrying three candidate foundations into this stage.
`competitive-analysis.md` established that section-level design is well served
and free while network-level design is either commercial or dormant, and
`market-trends.md` flagged the suitability of open street data for street-level
design as the central unresolved unknown.

All three foundations were assessed (Q1). This document records the technical
viability of the initiative under the constraints confirmed at this stage, and
the resulting foundation recommendation.

## Verdict

**Technically viable, with one significant dependency risk.**

The initiative is buildable by one person with AI assistance (Q3) on a roughly
$5-per-month hosting budget (Q9), because the hardest technical problem — turning
OpenStreetMap data into lane-level street geometry — has an existing,
permissively licensed solution rather than needing to be solved from scratch.

The dependency that makes it viable is also the largest risk, and that tension is
the substance of this assessment.

## The Central Technical Problem, and Why It Is Solved

A tool that designs street cross-sections on a real map needs, for any street a
user clicks: the lanes that street has, their types, their order, and their
widths. OpenStreetMap is the intended base data (Q7).

**OSM does not straightforwardly provide this.** The OSM wiki defines the `width`
tag as the carriageway measured kerb-to-kerb, explicitly excluding sidewalks,
cycle paths and street-side parking [1]. Sidewalk width in particular is largely
unmapped, and several competing, unapproved tagging schemes are in concurrent use
[2][3]. Taken alone this would put the initiative's premise at risk: a
map-grounded cross-section tool needs cross-section data, and the map does not
carry it reliably.

**osm2streets exists precisely to close that gap.** Its stated purpose is that
OSM "has many details about streets, but the schema presents many challenges for
rendering, routing, and analyzing done at the detail of lanes, especially in the
presence of dual carriageways, separated cycletracks and footways, and complex
intersections" [4]. It provides a simplified street network schema in which each
road carries a list of lanes from left to right with type, direction and width;
intersections are polygon areas; and transformations collapse unnecessary
intersections, merge dual carriageways, and snap parallel cycletracks and
footways to the main road. It renders to GeoJSON [4].

That schema is, structurally, a Streetmix cross-section derived from OSM — which
is the initiative's premise made tractable.

osm2streets is Apache-2.0, comes from the A/B Street organisation, and was last
pushed on 2025-10-02 [5]. Its core is Rust; the repository carries
`osm2streets-js` and `osm2lanes-js` bindings and a `release_npm.sh`, and the
published packages `osm2streets-js` 0.1.4 and `osm2lanes-js` 0.1.0 are both
Apache-2.0 [5][6].

## Foundation Assessment

| Candidate | Language / licence | Activity | Verdict |
|-----------|--------------------|----------|---------|
| Extend Streetmix | TypeScript; AGPL-3.0-or-later per its LICENSE file, though repository metadata reports NOASSERTION [7][8] | Actively maintained — last pushed 2026-09-04 [7] | **Not selected.** Assessed as a serious candidate; set aside for the reasons below. |
| Build on A/B Street (the application) | Rust; Apache-2.0 [9] | Dormant — last pushed 2025-09-10, roughly a year before this assessment; not archived [9] | **Not selected.** A dormant Rust application is a poor base for a solo builder, and the parts worth having are available separately as osm2streets. |
| Build new against the osm2streets schema | This project's own choice of licence; osm2streets dependency is Apache-2.0 [5] | osm2streets last pushed 2025-10-02 [5] | **Selected** (Q10). |

### Why not extend Streetmix

Streetmix is actively maintained, in a language that suits a solo builder with AI
assistance, and its purpose is aligned with the chosen audience. The case against
it is not about quality:

- **The scope mismatch inverts the core rather than extending it.** Streetmix's
  README states its scope as individual street sections, "not broader networks or
  maps" [8]. Adding a map, a network model, corridors and pluggable standards
  makes the network model the centre and the section editor one view within it.
  That is a different product wearing a familiar interface.
- **osm2streets changes what Streetmix contributes.** The lane schema — the hard
  part of the cross-section model — comes from osm2streets [4]. What extending
  would still supply is the drag-and-drop editing interface and segment artwork:
  real value, but a smaller share once the model comes from elsewhere.
- **Two schemas instead of one.** Extending requires reconciling Streetmix's
  segment model with osm2streets' lane schema on an ongoing basis. Building
  requires one schema with an editor over it.
- **AGPL inheritance is permanent.** AGPL-3.0-or-later [7] is fully compatible
  with the open-source-free model chosen at market research, so there is no
  conflict today. It does permanently foreclose an open-core model and makes this
  project's licence a consequence of a decision made by Streetmix LLC. This is a
  cost, not an objection.

### Why not build on A/B Street

A/B Street is map-grounded and network-scale, which is exactly what the
initiative needs, and its Apache-2.0 licence is permissive [9]. But the
application has been dormant for roughly a year [9], it is written in Rust, and
its focus is traffic simulation rather than street design. For one person
building with AI assistance, a dormant codebase in a less familiar language is a
materially worse bet than the alternatives. The valuable, separable part of that
ecosystem is osm2streets, which this recommendation adopts directly.

## Viability Against the Confirmed Constraints

| Constraint | Source | Assessment |
|------------|--------|------------|
| One person with AI assistance | Q3 | Viable. The heaviest single component — OSM to lane geometry — is a dependency rather than work. Remaining work is a lane editor over a known schema, a map view, persistence, and accounts. |
| Roughly $5/month hosting | Q4, Q9 | Viable on Railway's Hobby tier, which is $5/month including $5 of usage; the Free tier's $1 monthly credit will not cover a continuously reachable backend and database [10]. Per-second billing means non-production services cost nothing when stopped [10]. |
| Jurisdiction-neutral core with pluggable standards | Q2, Q8 | Viable and well matched to the osm2streets schema, which describes lanes physically (type, direction, width) rather than by any jurisdiction's classification [4]. Standards become a layer that reads that description rather than something baked into it. |
| Success measure: a week of planner work to under a day | `intent-statement.md` | Plausible but untested. Starting from imported real geometry rather than a blank cross-section removes the largest manual step, which is the mechanism by which the target would be met. No measurement supports it yet. [assumption] |
| Accounts, saved designs, shared links | Q6 | Viable, and the main driver of both the hosting cost and the compliance obligations below. |
| OSM plus city GIS import/export | Q7 | OSM is addressed by osm2streets. City GIS import/export is unassessed — no city's data has been examined and formats vary by city. [assumption] |

## Regulatory and Compliance Position

User accounts, saved designs and shared links tied to people (Q6), offered to an
audience of advocates and the general public with no geographic restriction
(Q2, and `intent-statement.md`'s audience), place this initiative in scope for
general privacy regulation from its first public release.

- **GDPR applies** to processing the personal data of EU/EEA residents regardless
  of where the operator is located, and a jurisdiction-neutral product invites
  exactly that. Its principles most relevant here are data minimisation, storage
  limitation, and the data subject rights of access, rectification, erasure and
  portability.
- **Comparable regimes** (CCPA and similar) apply on the same reasoning.
- **What this means concretely:** account data and saved designs need a retention
  position, an export path, and a deletion path, and these are functional
  requirements rather than operational afterthoughts.
- **What reduces the burden:** no payment data (the model is free, per market
  research Q7), and no special-category data, so PCI-DSS and HIPAA are out of
  scope. Street designs are not personal data; the account wrapper around them is.
- **Not established:** whether shared design links are public or access-controlled
  materially changes the disclosure surface, and no position has been taken.
  [assumption]

`constraint-register.md` records these as constraints and `raid-log.md` records
the associated risk.

## Principal Risks

Detailed entries with likelihood, impact and treatment are in `raid-log.md`. In
order of significance:

1. **osm2streets integration and inference quality.** The published npm packages
   are roughly three years old (June and October 2023) while the repository was
   pushed in October 2025 [5][6], so the practical path may be building the
   WebAssembly binding from source rather than installing a package. The project
   has itself been quiet for about eleven months. And where OSM tagging is thin —
   which the OSM wiki and the sidewalk literature confirm is common [1][2][3] —
   the derived cross-sections rest on inferred defaults rather than mapped
   values, so output quality varies by how well an area is mapped.
2. **Unvalidated success metric.** The under-a-day target has no measured
   baseline. [assumption]
3. **City GIS integration is unassessed.** Named as a requirement (Q7 C) with no
   evidence gathered. [assumption]
4. **Solo capacity.** One person is the entire delivery capability, with no
   redundancy.
5. **Privacy obligations from first release.** Manageable, but they are
   requirements rather than later work.

## Assumptions & Open Questions

- The claim that osm2streets' schema is sufficient for the initiative's editing
  needs rests on its README's description of that schema [4]; the library has not
  been run against a real area for this assessment. [assumption]
- Whether the WebAssembly bindings build cleanly from source at current toolchain
  versions has not been tested. [assumption]
- Streetmix's interaction design is proposed as a reference rather than a code
  dependency; whether its segment artwork is separately licensed for reuse has
  not been established, and no reuse of it should be assumed. [assumption]
- No effort estimate is attached to any option; the comparison is qualitative,
  since no timeline or rate has been established (Q4). [assumption]
- Railway's suitability is assessed from published pricing only [10]; no
  deployment has been attempted and no usage estimate exists, so the $5/month
  figure is a plan rather than a projection. [assumption]
- Whether shared design links are public or access-controlled is undecided and
  affects the privacy analysis. [assumption]
- The compliance position is a first-pass regulatory scan, not a privacy impact
  assessment or legal advice. [assumption]

## Sources

1. [Key:width — OpenStreetMap Wiki](https://wiki.openstreetmap.org/wiki/Key:width)
2. [Sidewalks — OpenStreetMap Wiki](https://wiki.openstreetmap.org/wiki/Sidewalks) and [Key:sidewalk — OpenStreetMap Wiki](https://wiki.openstreetmap.org/wiki/Key:sidewalk)
3. [OpenSidewalks — Rules for Mapping Pedestrian Pathway Features](https://tcat.cs.washington.edu/wp-content/uploads/OpenSidewalks_mapping_rulesForMapping.pdf)
4. [a-b-street/osm2streets on GitHub](https://github.com/a-b-street/osm2streets) — README schema, features and purpose statements; [StreetExplorer demo](https://a-b-street.github.io/osm2streets/)
5. GitHub API `repos/a-b-street/osm2streets`, read 2026-09-07: Apache-2.0, last pushed 2025-10-02, not archived; repository contents include `osm2streets-js`, `osm2lanes-js`, `release_npm.sh`
6. npm registry, read 2026-09-07: `osm2streets-js` 0.1.4 (modified 2023-06-04) and `osm2lanes-js` 0.1.0 (modified 2023-10-06), both Apache-2.0
7. Streetmix `LICENSE` file via GitHub API `repos/streetmix/streetmix/contents/LICENSE`: AGPL-3.0-or-later, relicensed from BSD 3-clause; repository licence metadata reports NOASSERTION
8. [streetmix/streetmix on GitHub](https://github.com/streetmix/streetmix) — README scope statement; GitHub API `repos/streetmix/streetmix`, read 2026-09-07: TypeScript, last pushed 2026-09-04
9. GitHub API `repos/a-b-street/abstreet`, read 2026-09-07: Rust, Apache-2.0, 8,166 stars, last pushed 2025-09-10, not archived
10. [Railway pricing](https://railway.com/pricing), read 2026-09-07
