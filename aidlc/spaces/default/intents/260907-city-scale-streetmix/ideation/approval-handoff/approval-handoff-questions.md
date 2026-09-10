# Approval & Handoff — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: every Ideation artifact — `intent-statement.md` and
`stakeholder-map.md` (intent-capture); `competitive-analysis.md` (market-research);
`feasibility-assessment.md` and `constraint-register.md` (feasibility);
`scope-document.md` and `intent-backlog.md` (scope-definition); `wireframes.md`
(rough-mockups).

Several questions the stage normally asks do not apply here and are not asked:
stakeholder agreement (you are the only stakeholder, per `stakeholder-map.md`),
mob staffing (Team Formation was skipped as a solo project), and budget
commitment (settled at feasibility as roughly $5 a month). The questions below
are the decisions that actually remain open before Inception.

---

## Q1. Are the accepted risks genuinely accepted?

`raid-log.md` records ten risks. Three would change the plan rather than merely
complicate it if they landed:

- **R-1** — osm2streets does not integrate as expected. Its published packages
  are roughly three years old and the project has been quiet for about eleven
  months. If this fails, the foundation decision reopens.
- **R-2** — inferred cross-section quality is poor where OpenStreetMap tagging is
  thin, so the tool produces confident-looking designs resting on defaults.
- **R-3** — osm2streets becomes unmaintained, leaving a solo builder holding a
  Rust and WebAssembly component they did not write.

- A. Accepted — proceed with these as known risks and the mitigations recorded.
- B. Accepted, but B-0 must genuinely gate the rest — nothing else starts until
  osm2streets is proven against a real area.
- C. Not accepted — one or more needs more work before Inception.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q2. The two backlog gaps from Rough Mockups — in or out?

`wireframes.md` flagged two things it designed that `intent-backlog.md` does not
contain: the hero landing page (screen S1), and the visual identity work implied
by choosing "distinctive from the start". Neither has a proto-Unit. Leaving them
unresolved means Inception designs against a backlog that does not match the
wireframes.

- A. Add both as proto-Units in stage 1 — they are part of the first release.
- B. Add the hero page to stage 1; treat visual identity as ongoing rather than a
  unit of work.
- C. Add both, but after stage 1 — the first release gets a plain entry point and
  plain styling.
- D. Not yet defined — carry them as open and let Units Generation decide.
- X. Other (please specify)

[Answer]: A

## Q3. The unresolved sequencing disagreement — settle it or carry it?

`intent-backlog.md` records a disagreement it deliberately did not resolve. A
strict value-first reading (your Q4 answer at Scope Definition) puts meeting-ready
output (B-8) before accounts and sharing (B-5, B-6), because output is the
differentiator. The three-stage split (your Q9 answer there) puts accounts first.
Delivery Planning was named as the owner of the final call.

- A. Carry it — Delivery Planning owns it, as recorded.
- B. Settle it now: accounts and sharing first, as the three-stage split says.
- C. Settle it now: meeting-ready output first, as strict value-first says.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q4. Does the market evidence support proceeding?

`competitive-analysis.md` found that the gap is real but narrower than "nothing
exists": section-level design is well served and free, network-level design is
either commercial (Remix Streets) or dormant (A/B Street), and participation
platforms collect comments rather than designs. The unoccupied position is the
join — free, map-grounded, network-scale design whose output an institution will
accept.

- A. Yes — the join is a real gap and worth building into.
- B. Yes, with a caveat: the evidence is descriptions rather than hands-on
  evaluation, and that should be revisited if a competitor turns out to cover
  more than its marketing suggests.
- C. No — the gap is too narrow or too contested to justify the work.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q5. Do the wireframes reflect what you actually want to build?

`wireframes.md` recommends a map-primary layout with the cross-section in a
drawer, and an edit-then-extend corridor interaction. Both were presented as
overturnable. You approved the stage with three Minor findings outstanding,
including an unsourced "Open source" footer item that implies a licensing
decision you have not made.

- A. Yes — the recommendations stand, and the findings stay as recorded.
- B. Yes on the design, but fix the findings before Inception builds on it.
- C. No — one of the recommendations should be reconsidered before Inception.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q6. Anything to change before Inception starts?

Inception turns this into requirements, user stories, a domain model, units of
work and a delivery plan. Changes are cheapest now, before that structure exists.

- A. Nothing — proceed as recorded.
- B. Something about scope — the staging or the capability set needs adjusting.
- C. Something about the foundation — the osm2streets decision needs revisiting.
- D. Something else, which I will describe.
- X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

Your answers:

- Risks: accepted as recorded, with their mitigations (Q1 A).
- Backlog gaps: add both the hero landing page and visual identity work as
  stage 1 proto-Units (Q2 A).
- Sequencing disagreement: settled now — accounts and sharing before
  meeting-ready output, as the three-stage split says (Q3 B).
- Market evidence: supports proceeding, with the caveat that the evidence is
  vendor and project descriptions rather than hands-on evaluation, to be
  revisited if a competitor covers more than its marketing suggests (Q4 B).
- Wireframes: the recommendations stand, but fix the three findings before
  Inception builds on them (Q5 B).
- Anything else to change: nothing (Q6 A).

Already done, following your Q5 answer:

All three rough-mockups findings are fixed in `wireframes.md`.

- R-01 — the corridor recommendation now carries the same explicit "this is a
  recommendation, not a decision" framing as the layout recommendation.
- R-02 — the unsourced "Open source" footer item is removed, and the text
  fallback now states that no licensing claim is made there.
- R-03 — every framed line in all 14 diagram blocks now shares one width, with
  border rows padded with dashes. Verified by measuring rather than by eye, after
  a first attempt padded borders with spaces and broke them.

This amends an artifact from a stage you already approved. The `## Review`
section in `wireframes.md` is deliberately left untouched, with its findings
still marked `New`, because editing a recorded reviewer verdict would falsify the
history. The amendment is recorded in `decision-log.md` instead.

What the artifacts will contain:

- `initiative-brief.md` — a one-pager: problem, market validation, feasibility
  and risk, scope boundary, concept visuals, delivery approach, and a go/no-go
  recommendation.
- `decision-log.md` — every decision made across the seven Ideation stages, with
  where it was made and what it rests on, including the post-approval amendment
  above and the Team Formation skip.
- `verification/phase-check-ideation.md` — the Ideation to Inception traceability
  check: intent to scope to backlog consistency, and whether every scope item has
  feasibility backing.

One thing I will flag rather than smooth over:

`intent-backlog.md` now understates the first release. Your Q2 and Q3 answers
here added two proto-Units to stage 1 and settled a sequencing question the
backlog explicitly left open. The initiative brief will carry the current
position, and Units Generation should read it alongside the backlog rather than
the backlog alone.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
