# Phase Boundary Verification — Ideation to Inception

Traceability check run at the end of the Ideation phase, per
`.claude/knowledge/aidlc-shared/verification.md`. The Ideation to Inception
boundary checks intent to scope to intent-backlog consistency, and whether every
scope item has feasibility backing.

**Result: PASS, with three advisories.**

## Boundary Checks

| Check | Result | Evidence |
|-------|--------|----------|
| Intent captured | Pass | `intent-statement.md` carries problem, target customer, success metric, trigger and scope signal, each source-tagged |
| Scope defined | Pass | `scope-document.md` gives an in/out boundary, three stages, and a stated minimum viable scope |
| Feasibility confirmed | Pass | `feasibility-assessment.md` returns "technically viable, with one significant dependency risk" |
| Initiative approved | Pass | `initiative-brief.md` recommends Go; the human approved each Ideation stage in turn |

## Intent to Scope to Backlog Consistency

Every capability in the scope traces to the intent, and every proto-Unit traces
to a capability.

| Intent element | Scope capability | Proto-Unit |
|---------------|-----------------|-----------|
| Corridors and networks on a real map | C1 map view; C2 street import; C4 corridor editing | B-1, B-2, B-4 |
| Cross-section design (table-stakes) | C3 cross-section editor | B-3 |
| Communication requirement: shareable outputs for public meetings | C7 meeting-ready output | B-8 |
| Success measure: a week to under a day | Served by C2 removing the largest manual step | B-2 |
| Accounts and saved designs | C5, C6 | B-5, B-6 |
| Privacy obligations arising from accounts | Public-release gate | B-7 |
| Jurisdiction-neutral with pluggable standards | C9 | B-9 |
| City GIS integration | C8 | B-10 |
| Top technical risk (R-1) | — | B-0 walking skeleton |

**No orphan capability.** Every capability in `scope-document.md` traces upward
to `intent-statement.md` or to a constraint in `constraint-register.md`.

**No orphan proto-Unit.** Every entry B-0 to B-10 in `intent-backlog.md` carries
a traceability row citing an approved upstream artifact or a confirmed answer.

**No unserved intent element.** Each element of the intent statement maps to at
least one capability.

## Feasibility Backing for Scope Items

| Capability | Feasibility backing | Status |
|-----------|--------------------|--------|
| C1 map view | Not individually assessed; conventional | OK |
| C2 street import | Central subject of `feasibility-assessment.md`; osm2streets assessed | OK |
| C3 cross-section editor | Table-stakes; the schema comes from osm2streets | OK |
| C4 corridor editing | Assessed as the largest stage 1 unit; no separate technical assessment | Advisory A-1 |
| C5, C6 accounts and sharing | Assessed for cost and regulatory consequence | OK |
| C7 meeting-ready output | Not technically assessed | Advisory A-2 |
| C8 city GIS | Named; no city's data examined (`raid-log.md` R-5) | Advisory A-3 |
| C9 pluggable standards | Assessed as reconciling neutrality with the differentiator | OK |
| Hero page, visual identity | Added at Approval & Handoff Q2; no feasibility assessment | Advisory A-2 |

## Advisories

Three items pass the boundary but carry gaps worth naming rather than leaving
implicit. None blocks the transition.

**A-1 — Corridor editing has no separate technical assessment.** C4 is the
capability that distinguishes this product, and `feasibility-assessment.md`
concentrates on the osm2streets dependency rather than on what network-scale
editing requires beyond it. `wireframes.md` designs the interaction but not its
technical demands. Domain Design should treat the network model as unassessed.

**A-2 — Three capabilities entered scope without feasibility backing.**
Meeting-ready output (C7) was never technically assessed, and the hero page and
visual identity were added after Feasibility completed. C7 is the differentiator,
so the gap matters most there: what a city will accept in a formal process
determines what the output must contain, and nothing establishes that.

**A-3 — City GIS integration rests on an unexamined requirement.** Recorded in
`raid-log.md` as R-5 and carried unchanged.

## Contradictions

**None outstanding.** Four were detected and resolved during the phase:

| Contradiction | Resolution |
|--------------|-----------|
| "Just you" selected alongside two other stakeholder groups | Intent Capture Q9: sole stakeholder today; the others are target user groups |
| Success measure with no checkable threshold | Intent Capture Q10: a week of planner work to under a day |
| Jurisdiction-neutral core against a city-acceptance differentiator | Feasibility Q8: pluggable per-jurisdiction standards over a neutral core |
| Accounts in the first release without privacy rights, against constraint RC-1 | Scope Definition Q8: rights become a gate on public availability |

One further tension was recorded as coherent rather than contradictory: a
city-institutional differentiator alongside an audience that is not primarily
cities. The advocate pain recorded at Intent Capture is having no credible way to
propose a change a city will take seriously, so city-acceptable output is what
that audience needs.

## Artifact Completeness

| Stage | Required artifacts | Present |
|-------|-------------------|---------|
| Intent Capture | intent-statement, stakeholder-map, questions | Yes |
| Market Research | competitive-analysis, market-trends, build-vs-buy, questions | Yes |
| Feasibility | feasibility-assessment, constraint-register, raid-log, questions | Yes |
| Team Formation | — | Skipped: the stage's condition names solo projects |
| Scope Definition | scope-document, intent-backlog, questions | Yes |
| Rough Mockups | wireframes, user-flow, questions | Yes |
| Approval & Handoff | initiative-brief, decision-log, questions | Yes |

## Known Divergence Between Artifacts

`intent-backlog.md` no longer states the current position. Two decisions at
Approval & Handoff changed it after it was written:

- Q2 added the hero landing page and visual identity as stage 1 proto-Units.
- Q3 settled the sequencing question the backlog explicitly left open, placing
  accounts and sharing before meeting-ready output.

`initiative-brief.md` carries the current position. Inception should read it
alongside the backlog rather than the backlog alone. This is recorded rather than
repaired because amending an approved artifact from a closed stage was already
done once this stage, for the wireframes findings, and is noted in
`decision-log.md`.

## Verification Method

This check was performed by reading each Ideation artifact and cross-referencing
its claims against its declared upstream inputs. It verifies that traceability
links exist and that no artifact contradicts another. It does not verify that any
claim is true, that any estimate is accurate, or that the design will work.

## Assumptions & Open Questions

- The check is structural. It confirms that every capability traces to an intent
  element and every proto-Unit to a capability; it does not judge whether the
  capabilities are the right ones. [assumption]
- No artifact has been validated against a user, a city, or a running system.
  Ideation produced no external contact. [assumption]
- Advisories A-1 through A-3 are gaps in coverage, not defects. They are recorded
  so Inception does not mistake absence of assessment for absence of risk.
  [assumption]
