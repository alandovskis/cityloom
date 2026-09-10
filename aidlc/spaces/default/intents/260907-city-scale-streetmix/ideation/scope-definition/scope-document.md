# Scope Document — Streetmix at City Scale

## What This Covers

`intent-statement.md` sets the problem as designing corridors and networks on a
real map rather than one disconnected cross-section, for an audience of city
staff, advocates and the public, with the differentiator being output a city will
accept in a formal process. `feasibility-assessment.md` found the initiative
technically viable for one person on a roughly $5-per-month budget, because
osm2streets supplies the OSM-to-lane-geometry step, and named that dependency as
the top risk. `constraint-register.md` fixes the boundaries this scope must
respect, including the privacy obligations that follow from user accounts.

This document draws the in/out boundary and stages the work.

## In Scope

Ten candidate capabilities were derived from the approved upstream artifacts and
put to a scope decision. Six are in the first release, staged across three
deliveries; two are Should Have beyond it.

### Stage 1 — the design surface

The usable core. No accounts, no identity, no obligations under privacy
regulation.

| Ref | Capability | Source |
|-----|-----------|--------|
| C1 | Map view — find a place, see its street network | Q1; `intent-statement.md` problem statement |
| C2 | Street import — turn a real street into an editable cross-section via osm2streets | Q1; `feasibility-assessment.md` |
| C3 | Cross-section editor — change the lanes of a selected street | Q1; table-stakes per market research Q3 |
| C4 | Corridor and network editing — work across connected streets | Q1; `intent-statement.md` problem statement |

### Stage 2 — accounts and sharing

| Ref | Capability | Source |
|-----|-----------|--------|
| C5 | Accounts — sign in, own your work | Q2, Q9 |
| C6 | Save and share — persistent designs with access-controlled links | Q2, Q5, Q9 |

Shared design links are **access-controlled by default**: the owner chooses who
can see each design (Q5).

### Stage 3 — the differentiator

| Ref | Capability | Source |
|-----|-----------|--------|
| C7 | Meeting-ready output — exports, printable plans, presentation views | Q2, Q9; the differentiator per market research Q3 |

### Should Have — after the three stages

| Ref | Capability | Source |
|-----|-----------|--------|
| C8 | City GIS import and export | Feasibility Q7; not selected as must-have at Q1 or Q2 |
| C9 | Pluggable jurisdiction standards | Feasibility Q8; not selected as must-have at Q1 or Q2 |

Neither is excluded. C9 in particular is how the jurisdiction-neutral core
(`constraint-register.md` TC-5) is eventually reconciled with the
city-acceptance differentiator; deferring it means early releases produce legible
output without claiming conformance to any standard.

## The Public-Release Gate

Data subject rights — at minimum erasure and export of an account holder's own
data — are **a gate on public availability**, not a capability scheduled into a
stage (Q8).

- The gate attaches to **stage 2**, because that is where accounts land.
- **Stage 1 is unaffected.** It carries no accounts and therefore no obligation.
- **Stage 2 may be built and used before the gate is met**, by the project owner
  and invited testers.
- **Stage 2 may not be opened to the public until it is met.**

This follows from `constraint-register.md` RC-1: accounts and saved designs tied
to people place the initiative in scope for GDPR and comparable regimes from the
first public release, and data subject rights are functional requirements rather
than operational work. Deferring the capability while withholding public release
satisfies the constraint rather than deferring it.

## Out of Scope

Recorded as Won't Have **this time** — not permanent decisions (Q7).

| Exclusion | Rationale |
|-----------|-----------|
| Traffic simulation and modelling | Left to the tools that already do it well. `competitive-analysis.md` records SUMO and Aimsun as covering this; competing there is not the differentiator. |
| Construction- or engineering-grade output | This produces proposals, not buildable drawings. Bounds the accuracy claim the tool makes and keeps it clear of the professional CAD category. |
| Editing the underlying OpenStreetMap data | Designs are proposals over the map, never changes to it. Keeps a one-way dependency on OSM and avoids the responsibilities of an OSM editing client. |

Also outside this scope, by omission rather than exclusion:

- No street-design standard is supported at first release, following from
  jurisdiction-neutrality (`constraint-register.md` TC-5). [assumption]
- Consultants are a customer group only; their needs do not shape scope
  decisions, though they may use the product (Q6). This settles the open issue
  carried from intent capture.

## Minimum Viable Scope

**Stage 1 is the minimum viable scope**: it delivers the problem statement's core
value — designing corridors and networks on a real map — without accounts,
sharing, or export.

The first release as originally answered contained six of ten capabilities
including the two largest, which is closer to the whole product than a minimum.
Staging it in three (Q9) keeps all six as must-have while allowing feedback after
stage 1 rather than after everything.

## Value Stream

How a capability becomes value for the person using it:

```
  A person wants to propose a change to a street
            |
            v
  [C1] Find the street on a map            <- stage 1
            |
            v
  [C2] Import its real lanes               <- stage 1  (osm2streets)
            |
            v
  [C3] Change the cross-section            <- stage 1
            |
            v
  [C4] Extend along the corridor           <- stage 1
            |
            v            ... value already delivered: a design exists
            |
            v
  [C5/C6] Keep it, and show chosen people  <- stage 2
            |
            v
  [C7] Produce something a city will accept <- stage 3
            |
            v
  A city has to respond to the proposal
```

<!-- Text fallback: a single vertical chain. Stage 1 covers finding a street,
importing its real lanes, editing the cross-section, and extending along the
corridor; value is already delivered at that point because a design exists.
Stage 2 adds keeping the design and showing it to chosen people. Stage 3 adds
producing output a city will accept, which is what makes the city respond. -->

The chain shows why stage 1 stands alone: a design exists at the end of it. It
also shows why C7 is last and still the differentiator — the earlier steps
produce a design, and C7 is what converts a design into something an institution
must engage with.

## Sequencing

- **A walking skeleton runs first**: one street imported, edited and saved (Q3).
  Its purpose is to retire `raid-log.md` R-1, the risk that osm2streets does not
  integrate as expected. Its "save" is persistence without accounts, since
  accounts belong to stage 2. [assumption]
- **Everything after the skeleton is sequenced value-first** (Q4). This is
  consistent with the skeleton rather than in tension with it: the skeleton exists
  to retire the one dominant technical unknown, after which risk is low enough
  that value ordering governs.
- Detailed Bolt sequencing is Delivery Planning's work, not this stage's.

## Dependencies Between Capabilities

| Capability | Depends on | Nature |
|-----------|-----------|--------|
| C2 street import | C1 map view | You must find a street before importing it |
| C3 cross-section editor | C2 street import | The editor edits what was imported |
| C4 corridor editing | C3 cross-section editor | A corridor is composed of sections |
| C6 save and share | C5 accounts | Ownership requires identity |
| C7 meeting-ready output | C4 corridor editing | Output renders what was designed |
| C9 pluggable standards | C4 corridor editing | Standards evaluate a design |
| C8 city GIS | C2 street import | Shares the import and geometry model |

The chain C1 → C2 → C3 → C4 is the critical path and is entirely within stage 1.

## Assumptions & Open Questions

- Stage 1's persistence-without-accounts reading of the walking skeleton is an
  interpretation of Q3 against Q9, not something either answer states.
  [assumption]
- No effort estimate attaches to any stage; the staging is by coherence and
  dependency, not by size. [assumption]
- Access-controlled sharing by default (Q5) sits slightly against an audience
  whose purpose is often to show a proposal widely. The reading here is that C7's
  explicit meeting-ready output carries the showing rather than the link, but
  Refined Mockups or User Stories should confirm the sharing flow does not make
  the common case awkward. [assumption]
- What counts as satisfying the public-release gate — which rights, to what
  standard — is not specified here and belongs in Requirements Analysis.
  [assumption]
- The ten candidate capabilities were derived from the approved upstream
  artifacts and offered for selection. A capability nobody thought to propose is
  not covered by this scope. [assumption]
- No availability, performance or scale target is in scope, because none has been
  established. [assumption]
