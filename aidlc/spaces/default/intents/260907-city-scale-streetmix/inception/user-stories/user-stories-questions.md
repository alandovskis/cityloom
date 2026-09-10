# User Stories — Story Plan and Questions

## Sources

- [desc] Initial description: "Streetmix at city scale"
- [scope] Workflow-selected scope: `feature`.

Upstream inputs: `requirements.md` (requirements-analysis), `team-practices.md`
(practices-discovery), `scope-document.md` and `stakeholder-map.md` (ideation).

## Plan

### Persona development approach

`stakeholder-map.md` names four groups: the project owner (the only actual
stakeholder today), city agency staff, elected officials and the public, and
planning consultants. Three of those four have never been spoken to — the map
says so explicitly. Personas will be written at the level the evidence
supports and will state their own basis, so a later stage cannot mistake a
plausible sketch for research. Q1 settles how far to go.

### Story format

Standard form — "As a [persona], I want [action], so that [benefit]" — with
stable `US{group}.{seq}` IDs and Given/When/Then acceptance criteria carrying
`AC{group}.{seq}.{n}` IDs, per the inception guardrail on BDD acceptance
criteria. Every story states its INVEST position; a story that fails a criterion
is split rather than annotated.

### Story prioritization

MoSCoW per story. The MVP boundary itself is decided at Delivery Planning, not
here. Priorities will follow `scope-document.md`'s existing three-stage split
rather than re-deciding it: Stage 1 capabilities (C1–C4) are Must Have, Stage 2
(C5–C6) and Stage 3 (C7) carry their own priorities within their stage, and
C8–C9 are Won't Have this time.

### Breakdown approach

Four candidates, settled by Q3: by capability (C1–C7), by user workflow, by
persona, or by delivery stage.

### What is already settled and is not re-asked

Carried forward rather than re-elicited: the ten capabilities and their
three-stage split; the public-release gate on Stage 2; performance budgets
(NFR1), city-scale working set (NFR2), 99.5% availability (NFR3), the
accessibility set (NFR4.1–NFR4.7), the $5 budget (NFR5); the three provenance
states and the rule that a correction never becomes `mapped` (FR3.1, FR3.4);
the prohibition on writing to OpenStreetMap (FR2.3); tests-first with the
osm2streets fixture committed before the adapter; and G1 as a product goal
rather than an acceptance criterion.

---

## Q1. How far should the personas go beyond what has been validated?

`stakeholder-map.md` is unusually honest: the project owner is "the only actual
stakeholder today", and the interests recorded for city staff, officials and the
public "have not been validated with anyone in those groups." Personas are the
place where that honesty is most easily lost, because a well-written persona
reads like a finding.

- A. Write one grounded persona for the project owner, and thin
  evidence-stated sketches for the three target groups — each carrying an
  explicit "not validated with anyone in this group" line. Stories reference
  them, but nobody can mistake them for research.
- B. Write full personas for all four groups with goals, pain points and tech
  comfort, marked as hypotheses at the top of the file.
- C. Write only the project owner and a single generic "person redesigning a
  street" persona. Add the others when someone has actually been spoken to.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q2. Which stages get stories now?

`requirements.md` covers all three stages (FR1–FR6 and FR11 are Stage 1; FR7–FR9
are Stage 2; FR10 is Stage 3). Writing stories for Stage 2 and 3 now costs
effort on work whose shape may change once Stage 1 is real. Writing none of them
means Delivery Planning sequences a backlog that stops at Stage 1.

- A. All three stages, with Stage 2 and 3 stories written at lower fidelity —
  story and priority, acceptance criteria only where a requirement already
  fixes them (FR9's erasure and export, for instance).
- B. All three stages at equal fidelity. One pass, complete backlog.
- C. Stage 1 only. Stage 2 and 3 stories are written when those stages are
  approached, against what Stage 1 actually taught.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B

## Q3. How should the stories be broken down?

- A. By user workflow — find a street, import it, edit it, extend it along a
  corridor, recover from a failed import, save, share, export. Vertical slices
  that each cut through the whole stack.
- B. By capability (C1–C7) — mirrors `scope-document.md` exactly, so the
  traceability from scope to story is one-to-one.
- C. By persona — everything the advocate does, then everything the planner
  does, then the public.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q4. What makes a street unable to accommodate a design?

FR5.2 requires a warning "where a target street cannot accommodate the design
being extended to it." Nothing yet says what that means, and it is the acceptance
criterion for the corridor feature — the one thing that makes this product
different from editing streets one at a time.

- A. Total width only — the design's lanes do not fit between the target
  street's kerbs. One rule, checkable, no jurisdiction knowledge needed.
- B. Width, plus lane types the target street cannot carry — for example a
  tram lane extended onto a street with no track. Needs a small set of
  compatibility rules.
- C. Width, lane types, and a soft warning where the target's own mapped data
  contradicts the design (a one-way given two-way lanes, say). Most useful and
  most to build.
- D. Not yet defined — let the walking skeleton establish what is detectable.
- X. Other (please specify)

[Answer]: A

## Q5. How much may a user correct when an import is wrong?

FR6.4 allows correcting imported data before designing on it, and FR3.4 makes
any correction `user-set`. The size of that correction surface is not fixed.
Standing prohibition: none of this ever writes to OpenStreetMap (FR2.3).

- A. Values only — change a lane's width or type where the import got it wrong.
  The lane list itself stays as imported.
- B. Values, plus adding and removing lanes — the import missed a cycleway, or
  invented one. The full cross-section becomes correctable.
- C. Whichever of those, plus the correction persists as a note on that street
  so a later import of the same street does not silently discard it.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q6. Where does a design live before there is an account?

Stage 1 has no accounts, but B-0's pass criterion requires an edit to be saved,
reloaded and come back identical, and FR7.2 requires a design made before
sign-in to survive signing in. Something stores it. FR9.3 expires anonymous
designs after 30 days, which reads as server-side. NFR5.2 wants computation in
the browser so cost does not scale with usage, and NFR5.1 caps spend at about
$5 a month.

This is a real fork, not a detail: it decides whether Stage 1 has a server at
all.

- A. In the browser only — the design lives on the device that made it. No
  server storage, no cost, nothing to expire. FR9.3's 30-day rule then applies
  only to designs saved server-side once accounts exist, and a design does not
  follow you to another device until you sign in.
- B. Server-side from Stage 1, keyed to an anonymous identifier, expiring after
  30 days exactly as FR9.3 says. A link works from any device immediately.
  Stage 1 now needs a database and carries storage cost.
- C. Browser-first with an explicit "keep this" action that uploads it —
  nothing is stored server-side unless the user asks. Two paths to build.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: B — superseded by Q9 C. See Q9: the answer recorded here was
server-side from Stage 1; the follow-up moved it to browser-first with an
explicit opt-in upload (Q6's option C). The operative answer is C.

## Q7. Which persona anchors the Must Have decisions?

MoSCoW needs someone specific to be unusable-without. `intent-statement.md`
serves advocates and the public; the differentiator is output a city will
accept. Those pull in different directions when a story is marginal.

- A. The advocate — someone outside government making a case. Must Have means
  they can produce a credible proposal without professional tooling.
- B. City agency staff — Must Have means the output survives a formal process.
- C. The project owner — Must Have means the thing they set out to build works,
  and the other groups are served as it grows.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q8. Follow-up — Q5 chose "whichever of those", which does not settle the surface

Q5's option C read "Whichever of those, plus the correction persists as a note on
that street". The persistence half is settled. The half it was stacked on is not:
option A (values only) and option B (values plus adding and removing lanes)
describe different products, and a story cannot be written against "whichever".

The difference is not cosmetic. Under A the imported lane list is fixed and the
overlay only ever carries replacement values, so an overlay entry always has a
baseline lane to attach to. Under B the overlay can hold lanes that exist in no
import, which means the overlay's own key has to survive a re-import that still
does not contain them — the same key stability problem `team.md` already flags
for osm2streets' positional indices, one level harder.

- A. Values only — width and type on lanes the import produced.
- B. Values, plus adding and removing lanes — the full cross-section is
  correctable, including a cycleway OpenStreetMap never recorded.
- X. Other (please specify)

[Answer]: B

## Q9. Follow-up — Q6 moves Stage 1 across the privacy line that Q6's own source drew

`scope-document.md` states the public-release gate plainly: "**Stage 1 is
unaffected.** It carries no accounts and therefore no obligation." That sentence
is true of a Stage 1 that stores nothing about anybody.

Q6 chose server-side storage from Stage 1, keyed to an anonymous identifier. An
identifier assigned to a device and stored alongside content that person created
is an online identifier, and under GDPR Article 4(1) an online identifier is
personal data. This is not a certainty about how a regulator would rule, and it
is not legal advice — but it is close enough to the line that "Stage 1 carries no
obligation" can no longer be asserted as fact in the artifacts.

`constraint-register.md` RC-1 and `project.md`'s first affirmed mandate both make
data subject rights a gate on **public release** wherever they apply. If they
apply to Stage 1, the gate moves with them. This is the contradiction to resolve
now rather than to discover at the Stage 1 release.

- A. Keep server-side storage, and move erasure and export to Stage 1 — anything
  stored server-side is deletable and exportable from the first release, with no
  account needed. The gate holds; Stage 1 pays for it.
- B. Keep server-side storage, and treat Stage 1 as out of scope for the gate on
  the reasoning that a random identifier with no account, no contact details and
  no cross-site linkage is a weak case. Record it as a stated risk with a named
  owner rather than an unexamined assumption.
- C. Revisit Q6 — browser-first with an explicit "keep this" upload (Q6's option
  C). Nothing reaches the server unless the user asks, so the anonymous-identifier
  question only arises for people who opted in, and Stage 1's default path stays
  genuinely obligation-free.
- D. Not yet defined.
- X. Other (please specify)

[Answer]: C

## Q10. When you discard an edit, does the value go back to being "measured"?

**The design and developer reviews disagree with each other on this**, which is
why it is here rather than integrated.

AC5.1.3 says a changed width becomes `user-set` and never returns to `mapped`.
AC5.4.3 says reverting an attribute restores its original provenance — which for
a width that was `mapped` means `mapped`. Both are Must Have and they cannot
both hold.

- The **design review** argues AC5.4.3 is right: discarding an edit is not a
  correction, the value genuinely is the OpenStreetMap-measured one again, and
  marking it `user-set` would understate its authority to a planner — the
  opposite failure from the one FR3.4 guards against, but still a failure.
- The **developer review** endorses AC5.1.3's one-way transition as "exactly the
  right invariant", because a one-way rule is clean, testable, and cannot be
  got wrong by a later refactor.

Both reviews, plus quality, separately agree that a third layer exists —
imported baseline, corrections (US7.3), design edits (US5) — and that revert must
target the *corrected* baseline where a correction exists. That part is settled
and will be written in regardless. Q10 is only about the provenance label.

- A. Revert restores the underlying value's own provenance. Discarding an edit
  on a mapped width shows `mapped` again, because that is what the value is.
  Provenance describes the value, not the history of the field.
- B. Once touched, always `user-set`. The one-way transition holds no matter
  what; a field the user has edited stays marked as user-influenced even after
  a revert. Simplest to state and hardest to get wrong.
- C. Not yet defined — carry both positions to Domain Design as a named dispute.
- X. Other (please specify)

[Answer]: A

## Q11. An affirmed practice does not survive contact with the dependency

`team.md` Code Style states: "The overlay is keyed on OSM way id plus a
project-owned lane discriminator, never on osm2streets' positional (`usize`)
indices." That rule was affirmed at the practices gate and stamped into memory.

The developer review read the osm2streets source and found the way-id half does
not work. `streets_reader/src/split_ways.rs` splits one OSM way into several
`Road`s at intersections, each constructed with the same `osm_ids`, and
`Road.osm_ids` is a vector documented as "One road may consist of multiple ways".
The relation is many-to-many in both directions. An overlay keyed on way id
applies an edit made on one block to every block of that street, or picks one
non-deterministically. `RoadID` cannot substitute — it is allocated from a
build-order counter.

The positional-index half of the rule is still correct and still needed. Only
the way-id half fails.

- A. Amend the practice now — the key becomes the OSM way id plus the
  direction-normalised pair of bounding OSM node ids plus a project-owned lane
  discriminator. Node ids are what `split_ways` actually splits on and are stable
  OSM identities. Fixes it at the source, before any code is written against the
  broken rule.
- B. Leave `team.md` alone and record the finding as a constraint on Domain
  Design, which owns the persistence model and will settle the key there.
- C. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q12. Is getting a design off the device a Must Have?

The design review argues US8.3 (explicitly upload a design) should be Must Have
rather than Should Have, and the argument is against your own stated anchor:
Marcus's persona is a campaign running over weeks, on browser storage that a
private window, cleared site data or eviction can take at any time, with no way
to put the work anywhere else. "That is not a credible proposal without
professional tooling — it is a proposal with a coin flip attached."

There is a second-order effect. AC8.2.2 is Must Have and requires the
work-is-gone message to explain that designs are kept on the device "unless
uploaded". If US8.3 does not ship, a mandatory message describes an action that
does not exist.

- A. Raise US8.3 to Must Have. The apology and its mitigation ship together.
- B. Keep US8.3 as Should Have and rewrite AC8.2.2 so it does not name a
  capability from an optional story, with alternative copy for the no-upload
  build.
- C. Not yet defined.
- X. Other (please specify)

[Answer]: A

## Q13. What does "apply a design to a street" mean when the lanes differ?

The developer review calls this "the biggest unanswered design question in the
set — bigger than the keying problem". AC6.1.2 says the design "is applied to
each chosen street" and never says what that means when the target's lane list
differs from the source's, which is the normal case rather than the exception.

Elm Street has six lanes; Oak Street has four. You designed on Elm. What arrives
on Oak?

- A. Wholesale replacement — the target's lane list becomes the source's,
  subject to the width warning from US6.2. Simplest to build and to explain;
  discards whatever the target actually had.
- B. Per-lane mapping — the design's changes are matched onto the target's
  existing lanes where they correspond, and the target keeps what the design
  does not speak to. Preserves the target's own data; needs a stated
  correspondence rule and will surprise people when it guesses wrong.
- C. Not yet defined — hand it to Domain Design as a named open question, and
  write US6.1's criteria so they do not foreclose either answer.
- X. Other (please specify)

[Answer]: B

## Consolidated Summary Confirmation

**Note on ordering.** Q1–Q9 were confirmed at this checkpoint before the
artifacts were written. Q10–Q13 came out of the design, developer and quality
contributions, which could only be written against a draft — so this
re-confirmation covers decisions taken after that draft existed. The artifacts
have been revised to match them. Confirming here therefore confirms Q1–Q9 as
before, and confirms the four triage outcomes as applied.

**What you settled before generation (Q1–Q9)**

- **Personas**: full personas for all four groups, marked as hypotheses (Q1).
- **Coverage**: all three delivery stages at equal fidelity (Q2).
- **Breakdown**: by user workflow — vertical slices (Q3).
- **Corridor fit**: total width alone; no lane-type rules, no data-conflict
  warnings (Q4).
- **Correction surface**: values plus adding and removing lanes, persisting
  against the street (Q5, Q8).
- **Pre-account persistence**: browser-first with an explicit opt-in upload
  (Q6 as superseded by Q9).
- **MoSCoW anchor**: the advocate (Q7).

**What you settled at the mob triage (Q10–Q13)**

- **Revert and provenance**: provenance describes the value, not the history of
  the field. Discarding an edit on a mapped width shows `mapped` again (Q10).
- **The overlay key**: amend the affirmed practice now — way id, plus the
  direction-normalised bounding node-id pair, plus a project-owned lane
  discriminator (Q11).
- **US8.3**: raised to Must Have; the apology and its mitigation ship together
  (Q12).
- **Corridor apply**: per-lane mapping, not wholesale replacement — the target
  keeps what the design does not speak to (Q13).

**What the three reviews changed, beyond those four**

The three ran blind to one another and converged independently on two things,
which is stronger evidence than any single lens: that NFR4.4's *editing* half
was asserted nowhere while traceability marked it covered, and that the
provenance revert rules contradicted themselves. Both are fixed.

Integrated without needing your decision:

- **Six stories added.** US5.5 (undo — exercised by AC5.3.3, required by your
  merge gate, promised by an approved wireframe, defined nowhere). US6.3
  (choosing a corridor, which owns the connected-street graph no story owned).
  US6.4 (bulk apply, its confirmation and its partial-failure behaviour). US7.5
  (corrections surviving a re-import). US13.1 (status changes are announced —
  every status in the draft was visual-only, against SC 4.1.3). US13.2 (the
  editing surface at 360 CSS pixels).
- **US6.1 narrowed** from the whole corridor feature to extending onto one
  adjacent street — a complete vertical slice that retires the real risk first.
- **The two performance criteria are deferred, not asserted.** AC3.1.2's ten
  seconds and AC5.1.2's hundred milliseconds named no device, no network, no
  street and no measurement boundary. They now carry deterministic CI
  assertions that can actually fail, with the user-facing figures sent to
  `nfr-requirements` with their conditions attached — the same discipline you
  already applied to branch coverage and to the week-to-a-day claim. The
  `traceability.json` rows moved from `OK` to `Deferred` to match.
- **The fixture suite could not detect the thing it exists for.** AC3.1.4 tests
  the adapter against a recorded fixture, so it stays green while the pinned
  osm2streets crate is broken. AC3.1.5 now asserts the pinned crate is actually
  invoked in the release build — the only test in the set that goes red the day
  the dependency moves.
- **AC6.2.1 got an operational definition.** osm2streets returns no
  kerb-to-kerb width, and `total_width()` includes sidewalks and verges, so the
  project now owns the definition rather than naming a quantity the dependency
  does not return. Where the target's width is wholly default-derived, the
  honest outcome is "fit could not be checked" rather than a warning about a
  comparison that carries no information.
- **Retry after a failed import** was committed by two approved artifacts and
  appeared in no story. Added, and offered first — the likely failures are
  transient, and forcing a blank cross-section discards the street you came for.
- **US8.2 was not buildable.** "I return expecting a design" is not detectable;
  the only signal is empty storage, which is also a first-time visitor. Split
  into the detectable case and an explicit criterion that a cold start shows the
  normal first-run prompt and no loss message.
- **Six criteria whose actor was the build** — the adapter, a test, memory
  layout, a cookie flag — moved out of the story set into a Verification Notes
  table, owned by `team-practices.md` and the test plan.
- **Three sad paths added** where the missing case is the one that happens:
  invalid width, failed upload, partial deletion. Plus a partial corridor apply,
  and an empty or failed coarse pass.
- **The MoSCoW rule now states its three classes** — persona, access, policy —
  because the single persona rule was silently overridden three times and could
  not be used by Delivery Planning to settle the next marginal story.

**Three approved wireframes now contradict a Must Have criterion**

Recorded in `stories.md` for Refined Mockups to correct, not silently ignored:

- **S4** says "Lanes will be scaled to fit." A scaled width is neither mapped,
  inferred nor user-set, so scaling produces a value with no valid provenance
  state. The story is right; the screen must change.
- **S7** draws three mutually exclusive sharing radios. FR8.2 requires two
  independent mechanisms, and the radio model silently destroys named grants
  when a link is enabled.
- **S8** draws the preview inside the export dialog, but US12.3 is Should Have.

**Two things I am recording as open rather than answering**

- **Your affirmed practice in `team.md` is still wrong on disk.** Q11 settled
  what the key should be, but the sanctioned path for changing an affirmed
  practice is the learnings ritual at the end of this stage, not a direct edit.
  I will surface it there as a learning candidate. Until you keep it, `team.md`
  still names a key that cannot distinguish two blocks of the same street.
- **OQ-US7**: whether B-0's third pass criterion is automatable at all depends
  on the deferred canvas-versus-DOM choice. If Domain Design picks canvas
  without an accessibility tree, "mapped visibly distinguished from inferred"
  falls to a manual walkthrough on the very first Bolt, indefinitely. That
  belongs in Domain Design's cost calculation, not in a discovery after B-0.

Does this all look correct?

- Looks correct
- Request changes

[Answer]: Looks correct
