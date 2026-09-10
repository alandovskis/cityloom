# Personas — Streetmix at City Scale

## Evidential basis — read this before quoting any persona below

`stakeholder-map.md` records that the project owner is "the only actual
stakeholder today", and that the interests attributed to city agency staff, to
officials and the public, and to consultants "have not been validated with
anyone in those groups."

Three of the four personas below are therefore **hypotheses**, and each carries
that marking on its own line rather than relying on this header. A persona is
the artifact most likely to be quoted away from its file, and a disclaimer at
the top does not travel with it.

Every persona uses they/them. These are archetypes, not people.

## Priority ranking

| Rank | Persona | Why this rank | Basis |
|---|---|---|---|
| 1 | P3 — the neighbourhood advocate | Anchors the persona class of Must Have (Q7). `intent-statement.md` names advocates and the public as who the product serves | Hypothesis |
| 2 | P2 — the city transportation planner | The differentiator is output a city will accept; P2 is who accepts it | Hypothesis |
| 3 | P1 — the solo builder | The only validated stakeholder, and the sole decision-maker, but not the product's target user | Grounded |
| 4 | P4 — the transport consultant | Named as a customer group, never as a stakeholder; the map itself flags the ambiguity | Hypothesis |

P3 outranks P1 deliberately. P1 decides what gets built; P3 is who it is built
for, and MoSCoW is answered from the user's side, not the builder's.

---

## P1 — Dana, the solo builder

**Basis: grounded.** Derived from `stakeholder-map.md`'s project-owner row,
`constraint-register.md` OC-1 and OC-4, and the practices interview. This is the
one persona backed by a real, present stakeholder.

- **Role**: Builds and maintains the product alone, with AI assistance. Decides
  scope and priority; no external sign-off exists.
- **Goals**: Get a working street-design tool into people's hands. Retire the
  osm2streets integration risk early. Keep the whole thing inside about $5 a
  month.
- **Pain points**: No second pair of eyes, so the machine gate is the only
  reviewer. Every stale command in the project's own instructions costs a round.
  A dependency shifting underneath the build has no one else to catch it.
- **Tech comfort**: High. Embedded and Linux background; web is the less
  familiar half.
- **Frequency**: Daily while building; the product's own features are used the
  way a builder uses them, not the way a user does.

**What this persona is not for**: Dana's fluency is the reason not to design for
Dana. A tool that only Dana can operate satisfies P1 and fails P3.

---

## P2 — Priya, the city transportation planner

**Basis: hypothesis. Nobody in this group has been spoken to.** Interests taken
from `stakeholder-map.md`'s "standards compliance, feasibility, and
defensibility" row, which was confirmed by selection from an offered list rather
than gathered from a planner.

- **Role**: Works inside a city transportation or public-works department.
  Receives proposals from the public and from consultants; has to decide what
  survives a formal process.
- **Goals**: Assess a proposal quickly enough to respond within a consultation
  window. Know which numbers in a proposal are measured and which are assumed.
  Avoid re-drawing someone else's idea to evaluate it.
- **Pain points**: Proposals arrive as pictures with no dimensions, or as
  dimensioned drawings whose dimensions came from nowhere. A confident-looking
  design built on guessed widths wastes more of their time than a rough sketch,
  because it takes longer to disprove.
- **Tech comfort**: Medium to high — fluent in GIS and CAD, but not willing to
  install anything to read one proposal.
- **Frequency**: Bursty. Nothing for weeks, then several proposals during a
  consultation.

**Why they matter to the design**: P2 is the reason FR3 exists. Provenance is
not a nicety for this persona; it is the difference between a usable submission
and one they must reject.

---

## P3 — Marcus, the neighbourhood advocate — PRIMARY

**Basis: hypothesis. Nobody in this group has been spoken to.** Constructed from
`intent-statement.md`'s stated audience and the problem statement, not from
interviews. The advocate is the active case-maker within
`stakeholder-map.md`'s "elected officials and the general public" row.

- **Role**: Not a planner and not paid for this. Wants a specific street changed
  — a crossing, a bike lane, a bus stop — and has to convince people who can
  authorise it.
- **Goals**: Produce something a city cannot wave away as a sketch. Show the
  same idea along a whole corridor rather than one block, because the objection
  is always "that only works there". Bring it to a meeting where people can see
  what is being proposed.
- **Pain points**: Professional tools cost money and take weeks to learn. Free
  tools produce a picture of one cross-section that answers none of the
  questions asked in a meeting. Being told the proposal is not credible, without
  being told what would make it credible.
- **Tech comfort**: Low to medium. Comfortable with a browser and a map; will
  not install software, read documentation, or complete a tutorial before
  seeing whether the tool is worth it.
- **Frequency**: Intense for a few weeks around a campaign, then nothing for
  months. Returns to a design they made and half-remember.

**Why this persona anchors Must Have**: if a story is not needed for Marcus to
produce a credible proposal without professional tooling, it is not Must Have by
persona.

That is not the whole test, and the draft that claimed it was then overrode it
three times without saying so. `stories.md` states three Must Have classes —
persona (this one), access (what any user needs to reach or keep using the
product at all), and policy (WCAG 2.1 AA, and the RC-1/RC-2 release gate). A
story that fits none of the three is not Must Have. Marcus anchors the first
class, which is the largest.

---

## P4 — Sam, the transport consultant

**Basis: hypothesis. Nobody in this group has been spoken to.**
`stakeholder-map.md` explicitly flags that whether consultants are a stakeholder
as well as a customer group is unresolved — this persona rests on the shakiest
evidence of the four.

- **Role**: Paid by cities to produce corridor studies and options appraisals.
  Does the same cross-section work repeatedly across projects.
- **Goals**: Skip the repetitive early stage — getting a real street into an
  editable form — and spend the time on the judgment part. Produce options fast
  enough to show a client three of them.
- **Pain points**: Every project starts by re-drawing streets that already exist
  in public data. Client-ready output and working drawings are different
  artifacts, and the early tool usually produces neither.
- **Tech comfort**: High. Already has CAD and GIS; will only adopt something
  that gets data out again.
- **Frequency**: Project-driven — weeks of heavy use, then a gap.

**Standing caution, and what this persona is for**: P4's needs pull toward
engineering-grade output and GIS interchange, both of which `requirements.md`
places out of scope or in Should Have (C8). Serving P4 fully would change the
product's scope.

P4 anchors exactly one story — US11.2, whose export satisfies "will only adopt
something that gets data out again", written for Marcus but serving Sam equally.
Beyond that, P4's function here is to be the persona the product deliberately
does not chase. That is a legitimate role for a persona and a more honest one
than pretending it drives requirements it does not.

---

## Coverage gap recorded rather than papered over

**Elected officials and passive readers are not separately modelled.**
`stakeholder-map.md` groups "elected officials and the general public" into one
row, and P3 represents its active half — the person making a case. Someone who
only needs to *understand* a proposal at a meeting, and an elected official who
must weigh it against other claims, have different needs from an advocate: they
consume output rather than produce it, and they arrive with no context.

FR10 (meeting-ready output) and FR11.1 (a landing page with a worked example)
are the requirements that serve them, and two criteria carry that weight
explicitly rather than by assertion: **AC1.1.2** (the worked example shows the
same street before and after, with a real dimension) and **AC12.1.3** (produced
output identifies the street and its location and states the before-and-after
relationship, legibly to someone who was not present when it was made). The
second exists because the first draft of the stories served this reader nowhere —
every output criterion was written from the producer's side.

Modelling the persona properly needs someone in the group to be spoken to first,
which has not happened. See `stories.md` OQ-US4.

## Traceability

Derived from `stakeholder-map.md` (all four groups, their standing, and the
unresolved-consultant flag), `intent-statement.md` (audience and problem),
`constraint-register.md` (OC-1 solo builder, OC-4 budget), `requirements.md`
(FR3 provenance, FR5 corridor, FR10 output, FR11 entry), and the Q1/Q7 answers
in `user-stories-questions.md`.

Amended after the design contribution (`contributions/aidlc-design-agent.md`):
P4's role restated, because the file claimed stories referenced it and none did;
the coverage gap's mitigation named against specific criteria, because it was
recorded honestly and delivered nowhere; and the Must Have rule corrected to the
three classes `stories.md` now states, because a single persona rule was
overridden three times in the draft without acknowledgement.
