# Requirements Analysis — Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `intent-statement.md` (intent-capture), `scope-document.md`
(scope-definition), `team-practices.md` (practices-discovery).

## What is already settled and is not re-asked

The functional picture, the business context and the technical context are
established and carried forward rather than re-elicited: the ten capabilities
C1-C10 and their three-stage split, the audience and their pain, the
differentiator, the Rust/WASM client on a pinned osm2streets crate, Railway
hosting, OpenStreetMap plus city GIS as integration targets, WCAG 2.1 AA
including the editing canvas, tests-first, and the walking skeleton's three-part
pass criterion.

## Where the real gaps are

A completeness pass across the six dimensions finds functional, business and
technical context well covered, and **non-functional requirements almost
entirely absent**. Every mention of availability, performance or scale in the
approved artifacts is a note that no target has been established. Two further
gaps were explicitly deferred to this stage:

- `scope-document.md` defers what actually satisfies the public-release gate —
  which data subject rights, to what standard.
- `team-practices.md` carries forward that the "week to under a day" measure must
  not become an acceptance criterion until a real baseline exists.

The questions below target those gaps and nothing else.

---

## Q1. What has to feel fast?

Performance, and the first real NFR. Importing a street means fetching
OpenStreetMap data and running it through osm2streets in the browser. Editing a
lane is local. These have very different budgets, and stating one number for
"the app" would be meaningless.

- A. Import under 3 seconds for a single street; edits feel instant, under 100ms.
- B. Import under 10 seconds is fine — it happens once per street; edits under
  100ms.
- C. Only edits matter — import can take as long as it takes, with clear
  progress shown.
- D. Not yet defined — set targets once the walking skeleton shows what is
  actually achievable.
- X. Other (please specify)

[Answer]: B

## Q2. How much street can someone work on at once?

Scale. "Corridor and network editing" has no size yet, and the answer changes the
data model, the rendering approach and the memory budget of a WebAssembly client.

- A. A corridor — up to roughly 10 connected streets in one design.
- B. A neighbourhood — up to roughly 100 streets.
- C. A whole city network — thousands of streets, with only the visible part
  loaded.
- D. Not yet defined — let the walking skeleton establish what is practical.
- X. Other (please specify)

[Answer]: C

## Q3. What actually satisfies the public-release gate?

`scope-document.md` made data subject rights a gate on public availability and
explicitly left their content to this stage. Under GDPR the full set is access,
rectification, erasure and portability. Not all of them cost the same to build,
and for a tool holding street designs rather than sensitive personal data, the
proportionate set is a judgment.

- A. Erasure and export only — delete my account and everything in it, and give
  me my designs in a usable format. The two that matter most in practice.
- B. Erasure, export, and access — add a view of what is held about me.
- C. The full set — access, rectification, erasure and portability, built
  properly.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q4. How long is data kept?

Retention. Two different things need an answer: designs made without an account,
and data belonging to an account. Storage costs money against a $5-a-month
budget, and "keep everything forever" is a decision rather than a default.

- A. Anonymous designs expire after 30 days; account data is kept until the
  account is deleted.
- B. Anonymous designs expire after 30 days; account data is kept until deletion,
  and inactive accounts are removed after 2 years with warning.
- C. Nothing expires automatically — everything is kept until explicitly deleted.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q5. How should the speed goal be recorded?

`intent-statement.md` sets success as a corridor redesign taking a week of
planner work today dropping to under a day. `team-practices.md` carries a warning
that this must not become an acceptance criterion until a real baseline exists —
nobody has been timed doing either.

- A. Record it as a product goal, explicitly not a testable requirement, and
  revisit once a real user can be timed.
- B. Record it as a requirement with the baseline named as an assumption, so it
  is testable but honestly caveated.
- C. Replace it with something measurable now — for example, a target number of
  interactions to redesign one street.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q6. What availability is expected?

Reliability. One Railway environment with deploy on merge means every merge is a
brief interruption, and there is no second environment to fail over to.

- A. Best effort — it is a free tool; occasional downtime is acceptable and needs
  no target.
- B. A stated target, around 99% monthly, so there is something to measure
  against.
- C. Higher than that, because a city meeting depending on it is the point.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C. 99.5% or better

## Q7. What happens when a street cannot be imported?

Error behaviour, and the requirement most likely to be under-specified. Some
streets will fail: OpenStreetMap data may be missing, malformed, or shaped in a
way osm2streets cannot resolve. `wireframes.md` sketches a retry and a
start-from-blank option; this decides what is actually required.

- A. Explain and offer a blank cross-section to start from — never a dead end.
- B. That, plus report the failure somewhere the maintainer can see it, so
  patterns become visible.
- C. That, plus let the user fix the input by hand — edit what was imported
  before designing on it.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q8. Follow-up — what does "loaded" mean at city scale?

Q1 and Q2 describe different loading models, and the difference drives the data
model rather than being a detail.

Q1 sets an import budget of under 10 seconds and describes it as happening "once
per street". Q2 says the working scale is a whole city network — thousands of
streets, with only the visible part loaded. Those fit together only if it is
clear what arrives when.

There is also a cost dimension. The client is Rust/WASM running in the user's
browser, so the computation is free to you. But where the OpenStreetMap data
comes from is not settled: if it passes through your Railway service, egress is
billed at $0.05 per GB against a $5 monthly budget; if the browser fetches it
directly from an OpenStreetMap API, it does not touch your budget at all.

- A. Street-at-a-time — the map shows the network from a basemap, and osm2streets
  runs only on streets the user actually selects. The 10-second budget is
  per-street and nothing else is imported.
- B. Viewport-at-a-time — everything visible is imported and processed as the map
  moves, so a street is already there when selected. The 10-second budget applies
  to a viewport, not a street.
- C. Both, in layers — a coarse pass over the viewport for what is shown, and a
  full osm2streets pass on the selected street. Two budgets, and more to build.
- D. Not yet defined — let the walking skeleton establish which is practical.
- X. Other (please specify)

[Answer]: C

## Consolidated Summary Confirmation

Your answers:

- Performance: import under 10 seconds, edits under 100 milliseconds (Q1 B).
- Working scale: a whole city network — thousands of streets, with only the
  visible part loaded (Q2 C).
- Public-release gate: erasure and export only — delete my account and everything
  in it, and give me my designs in a usable format (Q3 A).
- Retention: anonymous designs expire after 30 days; account data is kept until
  the account is deleted (Q4 A).
- The speed goal is recorded as a product goal, explicitly not a testable
  requirement, and revisited once a real user can be timed (Q5 A).
- Availability: 99.5% monthly or better (Q6 C).
- Failed import: explain and offer a blank cross-section, report the failure so
  patterns become visible, and let the user correct what was imported (Q7 C).
- Loading model: both in layers — a coarse pass over the viewport for what is
  shown, and a full osm2streets pass on the selected street (Q8 C).

Three things I will write in that follow from your answers rather than being
stated by them:

- **A third provenance state.** Q7 lets a user correct imported data. A corrected
  value is neither mapped nor inferred, so the provenance model needs `user-set`
  alongside them. Without it, correcting an inferred width would either look like
  a measurement or stay marked as a guess, and both are wrong. This also keeps Q7
  clear of the affirmed prohibition on editing OpenStreetMap data: the user edits
  this project's imported model, never the source.
- **The application service holds no attached volume.** Railway states that a
  service with a volume takes downtime on every redeploy. The 99.5% target and
  the free zero-downtime path both depend on state living in a separate database
  service. Nobody stated this directly; it follows from Q6.
- **Two performance budgets, not one.** Q8's layered model means the coarse
  viewport pass and the per-street osm2streets pass are measured separately. The
  10-second budget belongs to the street pass.

Two things I will record as open rather than answer:

- Where OpenStreetMap data is fetched from is unsettled and has a direct cost
  consequence — through the Railway service, egress bills at $0.05 per GB against
  a $5 budget; fetched by the browser directly from an OpenStreetMap API, it
  costs nothing. At city scale that is the difference between a negligible and a
  budget-breaking bill.
- 99.5% allows about 3.6 hours of downtime a month against a single environment
  with no failover. Zero-downtime deploys make it plausible; a platform incident
  has no fallback.

Does this all look correct before I generate the requirements artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
