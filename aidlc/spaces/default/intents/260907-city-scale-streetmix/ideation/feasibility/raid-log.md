# RAID Log — Streetmix at City Scale

Risks, Assumptions, Issues and Dependencies. Fixed boundaries live in
`constraint-register.md`; this log holds what is uncertain, unproven, currently
wrong, or relied upon.

Upstream inputs: `intent-statement.md`, `competitive-analysis.md`,
`market-trends.md`, `build-vs-buy.md`, and `feasibility-assessment.md`.

Likelihood and impact are scored High / Medium / Low. Treatment is one of
mitigate, accept, transfer, or avoid.

## Risks

| ID | Risk | Likelihood | Impact | Treatment | Action |
|----|------|-----------|--------|-----------|--------|
| R-1 | osm2streets does not integrate as expected: published npm packages are roughly three years old against an October 2025 repository, so the WebAssembly binding may need building from source at current toolchain versions | Medium | High | Mitigate | Build and run osm2streets against a real area as the first technical task, before any other design work depends on it |
| R-2 | Inferred cross-section quality is poor where OSM tagging is thin, so the tool produces confident-looking designs resting on defaults rather than mapped values | High | Medium | Mitigate | Establish what osm2streets infers versus reads, and make derived values visibly distinguishable from mapped ones in the product |
| R-3 | osm2streets becomes unmaintained: the project has been quiet for about eleven months and its parent application, A/B Street, has been dormant for roughly a year | Medium | High | Accept, with contingency | It is Apache-2.0, so forking is available. Record that the project would inherit maintenance of a Rust and WebAssembly component it did not write — a real cost for a solo builder |
| R-4 | The success metric is unvalidated: a week of planner work reduced to under a day has no measured baseline and no user to time | High | Medium | Mitigate | Treat the target as a hypothesis; find one real user or task to time before it hardens into an acceptance criterion |
| R-5 | City GIS integration proves harder than assumed: it is a stated requirement with no city's data examined and formats varying by city | Medium | Medium | Mitigate | Examine one real city's published street data before Domain Design fixes an import model |
| R-6 | Privacy obligations are under-served: GDPR-class rights apply from first release, and retrofitting erasure, export and retention is materially harder than building them in | Medium | High | Mitigate | Carry data subject rights into Requirements Analysis as functional requirements |
| R-7 | Solo capacity is the entire delivery capability, with no redundancy and no specialist cover | High | Medium | Accept | Keep scope small enough that a single person can finish something usable; prefer dependencies over bespoke work where the dependency is sound |
| R-8 | Cost overruns the stated budget once accounts, a database and a continuously reachable backend run: Railway's Free tier's $1 monthly credit will not cover them, and usage beyond Hobby's $5 is billed per GB of memory, vCPU and egress | Medium | Low | Mitigate | Set a Railway usage alert at the budget figure; keep non-production services stopped, since billing is per-second |
| R-9 | The jurisdiction-neutral core and the city-acceptance differentiator pull apart in practice: pluggable standards resolve the tension in principle, but no standards layer has been designed and none may materialise without contributors | Medium | Medium | Accept | Revisit at Scope Definition; a neutral tool with no standards plugged in still produces legible output, which is the fallback position |
| R-10 | Reuse of Streetmix's interaction design drifts into derivative work of an AGPL codebase | Low | High | Avoid | Do not read Streetmix source while building the editor; do not reuse its artwork without establishing separate permission |

## Assumptions

| ID | Assumption | Owner | Status | Validation |
|----|-----------|-------|--------|------------|
| A-1 | osm2streets' schema is expressive enough for the editing the product needs | Project owner | Unvalidated | Run it against a real area; compare its output to what the editor must manipulate |
| A-2 | Starting from imported real geometry rather than a blank cross-section is the mechanism that delivers the speed target | Project owner | Unvalidated | Depends on R-4's baseline |
| A-3 | Railway's Hobby tier accommodates the workload at roughly $5 per month | Project owner | Unvalidated | No usage estimate exists; validate after a first deployment |
| A-4 | An audience of advocates and the public is reachable without institutional distribution | Project owner | Unvalidated | Carried forward from market research; no distribution route has been established |
| A-5 | Interaction design may be referenced from Streetmix without creating a derivative work | Project owner | Unvalidated | The boundary has not been established with legal precision |
| A-6 | No street-design standard needs to be supported at first release | Project owner | Unvalidated | Follows from jurisdiction-neutrality (Q2, Q8); revisit when a real user appears |
| A-7 | Whether shared design links are public or access-controlled does not need deciding yet | Project owner | Unvalidated | It changes the privacy disclosure surface, so it must be decided before accounts ship |

## Issues

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| I-1 | Streetmix's repository licence metadata reports NOASSERTION while its LICENSE file states AGPL-3.0-or-later | Low | Open, external | Not this project's to fix. Recorded because automated licence tooling will misreport Streetmix, and this project's decision not to depend on that code makes the discrepancy harmless here |
| I-2 | No individual people are named for any stakeholder group; groups are identified by role only | Low | Open | Carried from `intent-statement.md`. Becomes material only when validation with real users is attempted |
| I-3 | Whether planning and engineering consultants are a stakeholder as well as a customer group remains unresolved | Low | Open | Carried from `intent-statement.md`; Scope Definition should settle it |

## Dependencies

| ID | Dependency | Type | Criticality | Fallback |
|----|-----------|------|-------------|----------|
| D-1 | osm2streets (Apache-2.0) for OSM-to-lane-geometry | External library | Critical — the initiative's premise rests on it | Fork it (the licence permits), or build lane inference from OSM tags directly, which is the work osm2streets exists to avoid |
| D-2 | OpenStreetMap data and its community's tagging practices | External data | Critical | None. There is no comparable open global alternative |
| D-3 | Railway for hosting | External service | Medium | Any comparable platform; nothing yet depends on Railway-specific behaviour |
| D-4 | A basemap or imagery service for the map view | External service | Medium | Unassessed. Named in the comparison set at market research but not selected or costed here |
| D-5 | City GIS data formats, per city | External data | Medium | Manual import, or deferring city GIS integration entirely |

## Escalations

None at this stage. R-1 is the item that would justify one: if osm2streets cannot
be made to run against a real area, the foundation decision recorded at Q10
reopens and the assessment in `feasibility-assessment.md` would need revisiting
before Scope Definition commits to it.

## Assumptions & Open Questions

- Likelihood and impact scores are the assessor's judgement from the evidence
  gathered in this stage, not the output of a scoring model or a workshop with
  other people. [assumption]
- No risk has a named owner other than the project owner, because there is only
  one person (Q3). [assumption]
- R-3 and D-1 both rest on osm2streets' observed repository activity rather than
  on any statement from its maintainers about the project's future. [assumption]
- The dependency list covers what this stage identified; a design stage will
  surface dependencies that ideation cannot see. [assumption]
