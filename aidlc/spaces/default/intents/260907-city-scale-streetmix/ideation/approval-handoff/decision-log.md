# Decision Log — Ideation

Every decision made across the Ideation phase, where it was made, and what it
rests on. Compiled from `intent-statement.md`, `stakeholder-map.md`,
`competitive-analysis.md`, `market-trends.md`, `build-vs-buy.md`,
`feasibility-assessment.md`, `constraint-register.md`, `raid-log.md`,
`scope-document.md`, `intent-backlog.md`, and `wireframes.md`.

"Where" cites the stage and its question number. A decision with no question
reference was derived in an artifact rather than answered directly.

## Product Decisions

| # | Decision | Where | Rests on |
|---|----------|-------|----------|
| D-1 | The problem is corridors and networks on a real map, not one disconnected cross-section | Intent Capture Q1 | The initiative description |
| D-2 | Three customer groups: city staff; advocates and the public; consultants | Intent Capture Q2 | — |
| D-3 | Success is measured by speed: a week of planner work to under a day | Intent Capture Q3, Q10 | Q10 supplied the threshold after Q3 gave only a direction |
| D-4 | The trigger is a gap in existing tools, not an external deadline | Intent Capture Q4 | — |
| D-5 | The project owner is the only actual stakeholder; city staff and the public are target user groups, not people currently involved | Intent Capture Q5, Q9 | Q9 resolved a literal contradiction in Q5 |
| D-6 | The differentiator is output a city will accept in a formal process | Market Research Q3 | D-2, D-5 |
| D-7 | The addressable audience is primarily advocates and the public, not cities | Market Research Q6 | — |
| D-8 | The product is open source and free | Market Research Q7 | — |
| D-9 | Consultants are a customer group only; their needs do not shape scope | Scope Definition Q6 | Settled an issue open since Intent Capture |
| D-10 | Shared design links are access-controlled by default | Scope Definition Q5 | Privacy exposure, `raid-log.md` A-7 |

## Technical Decisions

| # | Decision | Where | Rests on |
|---|----------|-------|----------|
| D-11 | All three foundations assessed: extend Streetmix, build on A/B Street, build new | Feasibility Q1 | `build-vs-buy.md` recommendation |
| D-12 | **Build new against the osm2streets schema**, with Streetmix as an interaction-design reference rather than a code dependency | Feasibility Q10 | Reversed the market-research leaning. See "Decisions That Changed" below |
| D-13 | osm2streets is adopted as the OSM-to-lane-geometry dependency | Feasibility, human-supplied at the summary checkpoint | Apache-2.0; its schema gives lanes left-to-right with type, direction, width |
| D-14 | The street model is jurisdiction-neutral, with pluggable per-jurisdiction standards | Feasibility Q2, Q8 | Q8 reconciled neutrality with the city-acceptance differentiator |
| D-15 | Hosting is Railway, on the Hobby tier at roughly $5 a month | Feasibility Q5, Q9 | Q9 converted "near zero" into a figure against published pricing |
| D-16 | User accounts, saved designs and shared links are in scope | Feasibility Q6 | Drives D-19 |
| D-17 | Integration targets are OpenStreetMap as base data and city GIS import/export | Feasibility Q7 | — |
| D-18 | Buying an existing product is not viable | `build-vs-buy.md` | Commercial products sell to institutions; the model is free (D-8) |

## Scope and Delivery Decisions

| # | Decision | Where | Rests on |
|---|----------|-------|----------|
| D-19 | Data subject rights are a **gate on public availability**, not a scheduled capability. The gate attaches to stage 2 | Scope Definition Q8, with `constraint-register.md` RC-1 | Q2 there left privacy rights out of the first release while including accounts, contradicting an approved constraint |
| D-20 | Three delivery stages: design surface; accounts and sharing; meeting-ready output | Scope Definition Q9 | The first release as first answered held six of ten capabilities including the two largest |
| D-21 | A walking skeleton runs first: one street imported, edited and saved | Scope Definition Q3 | Retires `raid-log.md` R-1 |
| D-22 | Sequencing after the skeleton is value-first | Scope Definition Q4 | Consistent with D-21: the skeleton retires the dominant unknown, then value governs |
| D-23 | Out of scope: traffic simulation; engineering-grade output; editing OSM data | Scope Definition Q7 | Recorded as Won't Have this time, not permanent |
| D-24 | Stage 1 is the minimum viable scope — a design exists at the end of it | `scope-document.md` | D-20 |
| D-25 | **Accounts and sharing precede meeting-ready output**, settling the disagreement the backlog left open | Approval & Handoff Q3 | D-20 over a strict reading of D-22 |
| D-26 | Hero landing page and visual identity are added as stage 1 proto-Units | Approval & Handoff Q2 | Gaps flagged by `wireframes.md` |

## Design Decisions

| # | Decision | Where | Rests on |
|---|----------|-------|----------|
| D-27 | Select-then-act is the primary editing interaction; drag is an accelerator, never the sole route | `wireframes.md` | Derived from D-29 and D-30 converging |
| D-28 | Wireframes cover all three stages | Rough Mockups Q1 | — |
| D-29 | Fully responsive, including editing on a phone | Rough Mockups Q4 | Audience is the general public |
| D-30 | WCAG 2.1 AA throughout, including the editing canvas | Rough Mockups Q5 | — |
| D-31 | Visual identity is distinctive from the start | Rough Mockups Q6 | Drives D-26 |
| D-32 | A hero landing page is the first-time entry point | Rough Mockups Q7 | Human-supplied at the checkpoint |
| D-33 | Layout: map primary with cross-section in a drawer — **recommended, overturnable** | `wireframes.md` | One layout for every form factor (D-29) |
| D-34 | Corridor: edit one street then extend — **recommended, overturnable** | `wireframes.md` | Value at the first street, before the corridor concept appears |

## Process Decisions

| # | Decision | Where | Rests on |
|---|----------|-------|----------|
| D-35 | Market research is conducted with cited sources rather than from the owner's knowledge | Market Research Q1 | The ideation evidence standard |
| D-36 | Team Formation is skipped | Conductor judgement at the stage's condition check | The stage's condition names solo projects; capacity is one person |
| D-37 | Risks R-1, R-2 and R-3 are accepted with their recorded mitigations | Approval & Handoff Q1 | — |
| D-38 | Market evidence supports proceeding, with the caveat that it is descriptions rather than hands-on evaluation | Approval & Handoff Q4 | — |
| D-39 | Go — proceed to Inception | `initiative-brief.md`, Approval & Handoff Q6 | The four reasons in the brief's recommendation |

## Decisions That Changed

Two positions were reversed by evidence during Ideation. Both are recorded
because a reader who sees only the outcome would not know they were contested.

**The foundation.** `build-vs-buy.md` leaned toward extending Streetmix, on the
reasoning that the section editor is table-stakes and Streetmix is a mature
implementation. Two things overturned it. First, checking live repository state:
A/B Street was last pushed in September 2025 in Rust, while Streetmix was pushed
three days before the assessment in TypeScript — which initially strengthened the
extend case. Then osm2streets was introduced, supplying the lane schema itself.
Once the hard part of the model came from a dependency, what extending
contributed dropped to the editing UI, while its costs — AGPL inheritance,
schema reconciliation, a scope mismatch that inverts the core — stayed. The
decision moved to building new (D-12).

**The market position.** Market research initially read the gap as "nothing
exists above the cross-section". The comparison found the gap narrower: Remix
Streets occupies network-level commercially, and A/B Street occupies it in open
source. The claim was narrowed to the join — free, map-grounded, network-scale,
institutionally credible — which nothing in the set occupies.

## Post-Approval Amendment

`wireframes.md` was amended **after** the Rough Mockups gate was approved, on the
authority of Approval & Handoff Q5 ("fix the findings before Inception builds on
it"). What changed:

| Finding | Change |
|---------|--------|
| R-01 | The corridor recommendation now carries the same explicit "this is a recommendation, not a decision" framing as the layout recommendation |
| R-02 | The unsourced "Open source" footer item is removed; the text fallback now states that no licensing claim is made there |
| R-03 | Every framed line in all 14 diagram blocks now shares one width, with border rows padded with dashes |

The `## Review` section in `wireframes.md` is deliberately unchanged, with its
findings still marked `New`. Editing a recorded reviewer verdict would falsify
the history of what was found and when; this entry is the record that the
required actions were subsequently carried out.

Separately, earlier in the Approval & Handoff stage a table-parsing defect in
that same review appendix — an unescaped pipe character inside a code span —
blocked both the review brief and the gate. Repairing it invalidated the terminal
review receipt and consumed the single stale-receipt recovery pass, which
returned READY with the three findings carried forward unchanged.

## Decisions Explicitly Deferred

| Deferred | To | Why |
|----------|----|----|
| What satisfies the public-release gate — which rights, to what standard | Requirements Analysis | D-19 establishes the gate, not its contents |
| Bolt composition and ownership | Delivery Planning | D-25 settles order, not grouping |
| A first supported jurisdiction's standards | After the three stages | D-14 requires neutrality; a standards layer is Should Have |
| City GIS formats | Later | No city's data has been examined |
| Whether corridors commonly vary along their length | First real use | Decides whether D-34 is the right primitive |

## Assumptions & Open Questions

- This log records decisions and their stated basis, not every alternative
  weighed. The artifacts hold the fuller reasoning. [assumption]
- Attribution to a question number reflects where a decision was confirmed, which
  is not always where it was first proposed. [assumption]
- D-36 was a conductor judgement against the stage's own condition rather than a
  human answer, and is recorded as such. [assumption]
- No decision here has been validated against a real user; Ideation produced no
  user contact. [assumption]
