# Intent Backlog — Streetmix at City Scale

Proto-Units for the initiative, prioritised with MoSCoW and ordered within each
band. These are candidate units of work, not the Units of Work that Units
Generation will define; they exist so that scope, priority and sequence are
recorded before design begins.

Upstream inputs: `intent-statement.md`, `feasibility-assessment.md`,
`constraint-register.md`, and `scope-document.md` from this stage.

Priority reflects the scope decisions at Q1, Q2, Q7, Q8 and Q9. Order within a
band reflects the value-first sequencing chosen at Q4, after the walking skeleton
required by Q3.

## Must Have — Stage 1 (the design surface)

| ID | Proto-Unit | Delivers | Depends on | Notes |
|----|-----------|----------|-----------|-------|
| B-0 | Walking skeleton: one street imported, edited and saved | Proof that osm2streets integrates and that a design round-trips through storage | — | Runs first (Q3). Retires `raid-log.md` R-1. Persistence here is without accounts. Not a shippable increment; a risk-retirement slice |
| B-1 | Map view — find a place, see its street network | C1 | B-0 | Entry point for everything else |
| B-2 | Street import via osm2streets — a real street as an editable cross-section | C2 | B-1 | The technical heart of the initiative |
| B-3 | Cross-section editor — change a street's lanes | C3 | B-2 | Table-stakes per market research Q3 |
| B-4 | Corridor and network editing — work across connected streets | C4 | B-3 | The capability that distinguishes this from Streetmix. Largest unit in stage 1 |

Stage 1 completes the minimum viable scope: at B-4 a design exists.

## Must Have — Stage 2 (accounts and sharing)

| ID | Proto-Unit | Delivers | Depends on | Notes |
|----|-----------|----------|-----------|-------|
| B-5 | Accounts — sign in, own your work | C5 | B-4 | Introduces personal data; from here the public-release gate applies |
| B-6 | Save and share — persistent designs with access-controlled links | C6 | B-5 | Access-controlled by default (Q5) |
| B-7 | Data subject rights — erasure and export of an account holder's own data | Satisfies the public-release gate | B-5 | **Gate, not a stage member.** Stage 2 may be built and used by the owner and invited testers before this exists; it may not be opened to the public until it does (Q8; `constraint-register.md` RC-1) |

## Must Have — Stage 3 (the differentiator)

| ID | Proto-Unit | Delivers | Depends on | Notes |
|----|-----------|----------|-----------|-------|
| B-8 | Meeting-ready output — exports, printable plans, presentation views | C7 | B-4 | The differentiator per market research Q3. Depends on the design, not on accounts |

## Should Have — after the three stages

| ID | Proto-Unit | Delivers | Depends on | Notes |
|----|-----------|----------|-----------|-------|
| B-9 | Pluggable jurisdiction standards | C9 | B-4 | How the jurisdiction-neutral core is eventually reconciled with the city-acceptance differentiator (`constraint-register.md` TC-5). Not excluded, not first-release |
| B-10 | City GIS import and export | C8 | B-2 | Named as a requirement at feasibility Q7 but not selected as must-have. `raid-log.md` R-5 notes no city's data has been examined |

## Could Have

None recorded. No capability was proposed that landed here: every candidate was
either selected as must-have, placed in Should Have, or excluded. [assumption]

## Won't Have — this time

| Exclusion | Rationale | Source |
|-----------|-----------|--------|
| Traffic simulation and modelling | Covered well by existing tools; not the differentiator | Q7 |
| Construction- or engineering-grade output | This produces proposals, not buildable drawings | Q7 |
| Editing the underlying OpenStreetMap data | Designs are proposals over the map, never changes to it | Q7 |

These are decisions for this initiative, not permanent product positions.

## Critical Path

```
  B-0 -> B-1 -> B-2 -> B-3 -> B-4 -+-> B-5 -> B-6
   |                               |     |
   |                               |     +-> B-7  (gate on public release)
   |                               |
   |                               +-> B-8
   |                               |
   |                               +-> B-9
   |
   +-- B-10 depends on B-2
```

<!-- Text fallback: B-0 through B-4 form a single chain and are the critical
path. B-4 then fans out three ways: to B-5 then B-6 for accounts and sharing,
with B-7 hanging off B-5 as a gate on public release rather than a sequential
step; to B-8 for meeting-ready output; and to B-9 for pluggable standards. B-10
depends only on B-2 and can be picked up any time after street import exists. -->

The chain B-0 to B-4 is the critical path and admits no parallelism worth
planning for at one person's capacity. After B-4 three independent branches open,
which is where sequencing choice starts to matter.

## Sequencing Rationale

- **B-0 first** regardless of value order, because `raid-log.md` R-1 is the risk
  that would invalidate the foundation decision recorded at feasibility Q10. A
  value-first order that discovered an osm2streets problem at B-2 would waste
  B-1 (Q3, Q4).
- **B-1 through B-4 in dependency order**, which is also value order: each step
  adds usable capability, and the chain has no alternative ordering.
- **After B-4, value-first governs** (Q4). B-8 delivers the differentiator, so a
  strict value reading places it before B-5 and B-6. The three-stage split (Q9)
  places accounts and sharing first instead, on the reasoning that output which
  cannot be kept or shown to anyone delivers less than the value ordering
  suggests. Delivery Planning owns the final call; both readings are recorded
  here rather than silently resolved. [assumption]
- **B-7 is a gate, not a step.** It does not sit in sequence; it conditions
  whether stage 2 may be made public.

## Traceability

| Proto-Unit | Traces to |
|-----------|-----------|
| B-0 | `raid-log.md` R-1; Q3 |
| B-1, B-2, B-3, B-4 | `intent-statement.md` problem statement; Q1 |
| B-2 | `feasibility-assessment.md` central technical problem |
| B-5, B-6 | Feasibility Q6; Q2, Q5 |
| B-7 | `constraint-register.md` RC-1; Q8 |
| B-8 | `intent-statement.md` communication requirement; market research Q3; Q2 |
| B-9 | `constraint-register.md` TC-5; feasibility Q8 |
| B-10 | Feasibility Q7; `raid-log.md` R-5 |

Every proto-Unit traces to an approved upstream artifact or a confirmed answer.
No proto-Unit was introduced without one.

## Assumptions & Open Questions

- The B-5/B-6 before B-8 ordering follows the three-stage answer (Q9) rather than
  a strict value reading (Q4), and the two are recorded as differing rather than
  reconciled. [assumption]
- No sizing or effort estimate attaches to any proto-Unit. B-4 and B-8 are
  described as large on structural grounds — the network model and a whole output
  workstream — not from any estimate. [assumption]
- Proto-Units are candidates for Units Generation, which may split, merge or
  rename them. Nothing here fixes the eventual Unit boundaries. [assumption]
- The Could Have band is empty because the candidate list was derived from
  upstream artifacts rather than brainstormed, so it contained little that was
  merely desirable. [assumption]
- B-7's content — which rights, to what standard — is not specified here and
  belongs in Requirements Analysis. [assumption]
