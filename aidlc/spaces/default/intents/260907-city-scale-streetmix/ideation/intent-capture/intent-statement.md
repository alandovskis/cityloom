# Intent Statement — Streetmix at City Scale

## Problem Statement

The initiative is described by its originator as "Streetmix at city scale". [desc]

Existing street-design tools stop at the single cross-section: Streetmix and its
alternatives let someone compose one slice of roadway, and nothing fills the gap
above that level. [Q4]

The problem to be solved is designing corridors and networks on a real map —
whole streets and how they connect to one another, grounded in actual geography,
rather than one disconnected slice at a time. [Q1]

## Target Customer

Three customer groups are named, each with its own current pain:

- City and municipal transportation and planning staff. Their pain is that CAD
  and GIS tooling is too slow and too specialised for early-stage street design
  conversations. [Q2]
- Community advocates, neighbourhood groups, and the general public. Their pain
  is having no credible way to propose a street change that a city will take
  seriously. [Q2]
- Planning and engineering consultants working for cities. Their pain is
  repeating the same manual cross-section and corridor work on every project.
  [Q2]

## Success Metrics

| Metric | Target | Source |
|--------|--------|--------|
| Time to produce a corridor redesign | A corridor redesign that takes a planner about a week of work today should take under a day in the tool | [Q3] [Q10] |

Speed is the success measure that has been committed to. [Q3] The threshold is a
week of planner effort today reduced to under a day in the tool. [Q10]

## Initiative Trigger

The trigger is a gap in existing tools rather than an external deadline:
Streetmix and its alternatives stop at the single cross-section and nothing fills
the gap above it. [Q4]

Decision authority for the initiative rests entirely with the project owner; no
external approval is needed. [Q6]

## Initial Scope Signal

| Signal | Value | Source |
|--------|-------|--------|
| Workflow-selected scope (`workflow-selected`) | `feature` | [scope] |
| User-confirmed product boundary | One substantial build, whether that lands as a new product or as an extension of the existing Streetmix codebase | [Q8] |

The workflow-selected scope is a process-sizing signal only and does not by
itself establish the product boundary. [scope] The product boundary was
separately confirmed as one substantial build. [Q8]

## Assumptions & Open Questions

- Whether this lands as a new product or as an extension of the existing
  Streetmix codebase is deliberately left open at this stage. [Q8] [assumption]
- Speed is the only success measure captured. Adoption and decision-impact
  outcomes were offered and not selected; their absence here records what was
  chosen, not a decision to exclude them. [Q3] [assumption]
- The "about a week of planner work today" baseline is a stated estimate rather
  than a measured figure, so the speed target has not been validated against
  observed practice. [Q10] [assumption]
- Planning and engineering consultants were named as a customer group but were
  not among the stakeholder selections; whether they are a stakeholder as well as
  a customer is unresolved. [Q2] [Q5] [assumption]
- No geographic, jurisdictional, or street-design-standards context has been
  established (which city or country's standards the tool must respect).
  [assumption]

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-07T08:01:35Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260907-city-scale-streetmix/ideation/intent-capture/stakeholder-map.md > Key Stakeholders and Interests, row "Planning and engineering consultants working for cities" | Consultants were confirmed only as a customer group (Q2 A,B,C) and were explicitly NOT selected among the confirmed stakeholders (Q5 A,B,C as clarified by Q9 A). The grounding contract (Step 4, rule 6) requires content that cannot be confirmed to be confined to `## Assumptions & Open Questions`, yet this row places an unconfirmed stakeholder role in the main "Key Stakeholders and Interests" table. The artifact does separately flag the ambiguity under Assumptions, but the table row itself still asserts stakeholder standing that was never confirmed. | Remove the consultants row from the "Key Stakeholders and Interests" table (leaving the point solely under Assumptions & Open Questions), or convert it into a follow-up question so the human can confirm or reject consultants as a stakeholder before this ships as READY. | New |
| R-02 | Minor | aidlc/spaces/default/intents/260907-city-scale-streetmix/ideation/intent-capture/intent-statement.md > Initiative Trigger | The sentence "Decision authority for the initiative rests entirely with the project owner; no external approval is needed. [Q6]" is governance/authority content, not trigger-for-now content. The stage template scopes this section to "why now"; decision authority belongs under the stakeholder map's Decision-Makers vs. Influencers section, where it is in fact also recorded. Duplication in the wrong section reduces clarity for a non-technical reader skimming by heading. | Move that sentence out of Initiative Trigger (it is already captured correctly in stakeholder-map.md's Decision-Makers section). | New |
| R-03 | Minor | aidlc/spaces/default/intents/260907-city-scale-streetmix/ideation/intent-capture/intent-capture-questions.md > Q9 | Q9 is a follow-up question with options A/B/C/X but no `Not yet defined`/`None`/`Not applicable` escape option, unlike every other question in the file. For a contradiction-resolution follow-up this is a defensible design choice, but it is a deviation from the stage's blanket instruction that "every question MUST include" such an option. | No artifact change needed retroactively (the human already answered A), but note this for future follow-up questions in this workflow so an escape hatch is offered even on contradiction-resolution follow-ups. | New |

### Summary

The artifacts are well-grounded overall: nearly every claim carries a correct inline source tag, the workflow-selected scope is cleanly separated from the user-confirmed product boundary, the success metric is measurable ("under a day" vs. "about a week"), and open items are honestly captured as `[assumption]`. The one Major finding is a placement issue — an unconfirmed stakeholder role (consultants) appears in the main stakeholder table rather than being confined to Assumptions & Open Questions as the grounding contract requires — and is straightforward to fix or accept as a known risk at the gate.
