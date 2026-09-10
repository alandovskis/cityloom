# Intent Capture & Framing — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

---

## Q1. What business problem is this solving?

Streetmix (the existing open-source tool) lets one person design a single street
cross-section — you drag segments like a bike lane, a parking lane, a sidewalk
into a slice of roadway. "At city scale" implies something bigger than one
slice. Which of these best names the problem you want to solve?

- A. Corridors and networks on a real map — designing whole streets and how they
  connect, grounded in actual geography, rather than one disconnected slice.
- B. Decision-grade analysis at city scale — costs, capacity, safety, mode shift
  computed across many streets at once.
- C. Collaboration and process — shared city projects, versions, review and
  sign-off, instead of one-off shareable links.
- D. Not yet defined — I want to explore the problem before committing to it.
- X. Other (please specify)

[Answer]: A

## Q2. Who is the customer, and what pain are they living with today? (select all that apply)

Naming the primary user changes almost every downstream decision — a city
transportation department, an advocacy group, and a consulting firm want very
different things from the same tool.

- A. City / municipal transportation and planning staff — pain: CAD and GIS
  tooling is too slow and too specialised for early-stage street design
  conversations.
- B. Community advocates, neighbourhood groups, the general public — pain: no
  credible way to propose a street change that a city will take seriously.
- C. Planning and engineering consultants working for cities — pain: repeating
  the same manual cross-section and corridor work on every project.
- D. Not identified yet — the user group is still open.
- X. Other (please specify)

[Answer]: A, B, C

## Q3. What does success look like, and what would you measure? (select all that apply)

We need at least one outcome you could actually check later — a number, a
threshold, or an observable event, not "it works well".

- A. Adoption — a number of cities or teams using it for real projects within a
  defined period.
- B. Speed — a corridor redesign that takes weeks today takes hours or days.
- C. Decision impact — designs produced in the tool are what actually go into
  public consultation or a capital plan.
- D. Not yet defined — I'd like help proposing measurable outcomes later.
- X. Other (please specify)

[Answer]: B

## Q4. What is triggering this now?

Why is this worth building at this moment rather than a year ago or a year from
now?

- A. A concrete opportunity — a city, client, grant, or program that needs this
  and has a timeline.
- B. A gap in existing tools — Streetmix and its alternatives stop at the single
  cross-section and nothing fills the gap above it.
- C. New data or technology makes it feasible now (open street data, better
  basemaps, cheaper compute, LLM assistance).
- D. Exploratory — you want to build it; there is no external trigger.
- X. Other (please specify)

[Answer]: B

## Q5. Who are the key stakeholders, and what does each one care about? (select all that apply)

A stakeholder here is anyone whose needs shape the product or who can block it —
not just the end user.

- A. Just you — no other stakeholders yet.
- B. City agency staff (planning, transportation, public works) — care about
  standards compliance, feasibility, and defensibility.
- C. Elected officials and the public — care about legibility, fairness, and
  being able to weigh in.
- D. A funder, client, or sponsoring organisation — cares about deliverables and
  timeline.
- X. Other (please specify)

[Answer]: A, B, C

## Q6. Who decides scope and priority, and who only influences it?

We need to know whose "no" stops work and whose input is advice, so the workflow
knows who to bring decisions to.

- A. You decide everything — no external approval needed.
- B. You decide, but a specific partner, client, or city contact must sign off on
  direction.
- C. A committee, organisation, or open-source governance body decides; you
  propose and implement.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q7. Are there communication or reporting requirements?

This is about what the product itself must produce for other people — not how we
talk during this workflow.

- A. None — no reporting obligations; the tool's own screens are enough.
- B. Shareable outputs for public meetings — exports, printable plans,
  presentation views.
- C. Formal deliverables for a city or client on a defined cadence — reports,
  data handoffs, standards documentation.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q8. Does the workflow-selected scope match the product boundary you intend?

This workflow was started with the `feature` scope — a full lifecycle at
practical depth. That is a process-sizing choice and it is separate from what the
product itself covers. This question is about the product boundary.

- A. Confirm — treat "Streetmix at city scale" as one substantial build, whether
  that lands as a new product or as an extension of the existing Streetmix
  codebase.
- B. Bigger — this is a whole platform, materially larger than one feature, and
  should be framed that way.
- C. Smaller — this is a focused prototype or proof of concept to test whether
  the idea holds up.
- D. Not yet defined — I want to settle the product boundary later in the
  workflow.
- X. Other (please specify)

[Answer]: A

## Q9. Follow-up — you selected both "Just you" and two other stakeholder groups. Which is it?

On Q5 you selected `A. Just you — no other stakeholders yet` together with
`B. City agency staff` and `C. Elected officials and the public`. Those conflict
literally: A says there are no other stakeholders. Combined with Q6 (`A. You
decide everything`), the likely reading is that you are the only person with
authority today, while city staff and the public are the groups the product must
serve. Which is accurate?

- A. You are the only actual stakeholder today; city staff and the public are
  target user groups the product must serve, not people currently involved.
- B. City staff and/or officials are genuinely involved today — there are real
  people whose input already shapes this.
- C. Just you, full stop — remove city staff and the public from the stakeholder
  map entirely.
- X. Other (please specify)

[Answer]: A

## Q10. Follow-up — what speed improvement counts as success?

On Q3 you selected `B. Speed`. A phase rule requires success metrics to be
measurable, so "faster" needs a threshold we can check later. Roughly what should
be true?

- A. A corridor redesign that takes a planner about a week of work today should
  take under a day in the tool.
- B. A corridor redesign that takes weeks today should take about an hour —
  same-session, not same-week.
- C. The threshold matters less than the direction; record it as an assumption to
  refine once there is a real user to time.
- D. Not yet defined — I'd like to set this after Feasibility.
- X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

Summary of all answers:

- Problem: corridors and networks on a real map — designing whole streets and how
  they connect, grounded in actual geography, rather than one disconnected slice
  (Q1 A).
- Customers: city / municipal transportation and planning staff; community
  advocates, neighbourhood groups and the general public; planning and
  engineering consultants working for cities (Q2 A, B, C).
- Success measure: speed — a corridor redesign that takes a planner about a week
  of work today should take under a day in the tool (Q3 B, Q10 A).
- Trigger: a gap in existing tools — Streetmix and its alternatives stop at the
  single cross-section and nothing fills the gap above it (Q4 B).
- Stakeholders: you are the only actual stakeholder today; city agency staff and
  elected officials/the public are target user groups the product must serve,
  not people currently involved (Q5 A, B, C as clarified by Q9 A).
- Decision authority: you decide everything; no external approval needed (Q6 A).
- Reporting: the product must produce shareable outputs for public meetings —
  exports, printable plans, presentation views (Q7 B).
- Product boundary: confirmed as one substantial build, whether that lands as a
  new product or as an extension of the existing Streetmix codebase (Q8 A).

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Assumption Confirmation

Both artifacts carry open items under `## Assumptions & Open Questions`. They are
labelled as assumptions, not facts, and accepting them does not turn them into
facts — it records that the workflow may proceed with them still open.

From `intent-statement.md`:

1. Whether this lands as a new product or as an extension of the existing
   Streetmix codebase is deliberately left open at this stage.
2. Speed is the only success measure captured; adoption and decision-impact
   outcomes were offered and not selected.
3. The "about a week of planner work today" baseline is a stated estimate rather
   than a measured figure, so the speed target has not been validated against
   observed practice.
4. Planning and engineering consultants were named as a customer group but were
   not among the stakeholder selections; whether they are a stakeholder as well
   as a customer is unresolved.
5. No geographic, jurisdictional, or street-design-standards context has been
   established (which city or country's standards the tool must respect).

From `stakeholder-map.md`:

6. No individual people are named for any stakeholder group; the groups are
   identified by role only.
7. Whether planning and engineering consultants are a stakeholder as well as a
   customer group is unresolved.
8. No reporting cadence to a sponsor or funder has been established, and no
   sponsor or funder has been identified.
9. The interests recorded for city agency staff and for officials and the public
   are the ones offered with those stakeholder options and confirmed by
   selection; they have not been validated with anyone in those groups.

- A. Accept assumptions
- B. Convert to follow-up questions

[Answer]: A. Accept assumptions

