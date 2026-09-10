**Collaborator:** aidlc-design-agent

## Contribution

Reviewed through the UX and persona-fidelity lens: are these personas honest and
useful, do the stories describe an experience a person could actually have, and
do the interaction commitments hold together against the approved wireframes and
flow.

The draft is strong on structure. The two accessibility stories (US2.3, US5.3)
are correctly promoted to stories rather than buried as criteria; the provenance
group is the best-specified part of the set; the bad-news case in US8.2 exists at
all, which is the failure mode most sets omit. What follows is where it does not
hold.

**None of my findings turn on OQ2** (DOM versus canvas editing surface). Every
fix below is writable without knowing that answer — status messages live in the
surrounding DOM either way, and a viewport-width criterion is a layout assertion,
not a rendering-technique one. I have not assumed either answer anywhere.

---

### 1. Three direct contradictions with the approved wireframes

The brief asks for these specifically. All three are cases where a story and an
approved screen describe different products, and someone building from S4 or S7
would build the wrong thing.

**1a. AC6.2.2 contradicts wireframes.md S4 on what happens to a design that does
not fit.** S4's note reads: *"Oak Street is narrower than Elm. **Lanes will be
scaled to fit.** Review after applying."* AC6.2.2 says: *"the design is applied
and the street remains marked as not fitting"* — applied unchanged, not scaled.

The story is right and the wireframe is wrong, and it is worth saying why rather
than just picking a side: scaling lanes to fit would silently produce widths the
user did not choose, that OpenStreetMap did not measure, and that osm2streets did
not infer. Under FR3.1 every attribute carries exactly one of `mapped`,
`inferred`, `user-set` — a scaled width is none of the three. Scaling to fit
breaks the provenance model, which is the product's whole credibility mechanism.

Fix: keep AC6.2.2 as written, and record that S4's "Lanes will be scaled to fit"
line must be corrected at refined-mockups. Do not leave the wireframe standing as
an approved screen that contradicts a Must Have criterion.

**1b. US10.2 contradicts wireframes.md S7 on the sharing model.** S7 draws three
*mutually exclusive* radio options (Only me / People I invite / Anyone with the
link). AC10.2.1 and AC10.2.2 describe two *independent* mechanisms — named access
and link, each separately enabled. FR8.2 ("as separate explicit choices") reads
with the story, not the radio group.

This matters beyond consistency. Under S7's radio model, a user who has invited
three people and then selects "Anyone with the link" has just done something to
those three invitations that no artifact specifies — and a user who narrows back
to "People I invite" has silently killed a link that may already be in a meeting
agenda. That is a privacy surprise in both directions, in the one dialog where
surprises are least acceptable.

Fix: keep the story's independent model, and record S7 as needing redrawing —
"Only me" as the default *state* rather than one of three exclusive options, with
named access and link as two separately-toggled controls beneath it. Add an
AC10.2.x for what happens to named access when a link is enabled and disabled,
because "separate explicit choices" does not by itself say they are independent.

**1c. Retry is missing entirely, and both approved artifacts commit to it.**
wireframes.md S3's error state: *"states plainly that this street's data could
not be read, **offers retry**, and offers starting from a blank cross-section
instead."* user-flow.md's Error and Recovery table, first row: *"**Retry**, or
start from a blank cross-section."* US7.1 explains and US7.2 offers a blank
cross-section. Neither offers retry, and no AC anywhere mentions it.

This is not a cosmetic omission. The likely failure modes here are transient —
an OSM fetch that timed out, a network blip on a phone, a rate limit. Forcing a
user to a blank cross-section after a transient failure throws away the imported
street they actually wanted and drops them into US7.2's world where everything
they enter is `user-set` — the exact opposite of what they came for. A blank
cross-section is the right *last* resort and the wrong *only* resort.

Fix: add AC7.2.x — the failure offers retry alongside the blank cross-section,
and retry is the first-offered action. If the intent was that retry is out of
Stage 1, say so and correct both approved artifacts; do not leave two approved
screens promising an action no story delivers.

### 2. Undo is required by three artifacts and owned by no story

This is the largest hole in the set.

- AC5.3.3 exercises undo: *"select a lane, change its type, change its width and
  **undo**, Then each step produces the expected model state."*
- `team.md` Testing Posture makes it a merge gate: *"a keyboard-only interaction
  test for every editing action (select a lane, change its type, change its
  width, extend along the corridor, **undo**)"*.
- wireframes.md Screen States, Success row: *"A brief confirmation naming what
  happened **and how to undo it**"*, applied to S4 after applying a corridor.

No story defines undo. It appears in no US, no FR, and the only thing resembling
it is AC5.4.3 — reverting a single changed attribute to its imported baseline.
That is a different operation: revert-to-baseline is not undo-last-action, and
neither of them is "undo the corridor apply that just wrote a design onto four
streets", which is the one the approved wireframe promises and the one with the
highest cost of not having.

Fix: add a story in US5 for undoing the last editing action, with an AC covering
the corridor apply as a single undoable unit; then AC5.3.3 has something to point
at. Until that exists, AC5.3.3 is untestable and the merge gate in `team.md`
cannot be satisfied.

Related, same area: **US6.1 has no criterion for what the user sees after a
corridor apply succeeds.** AC6.1.2 ends at "the design is applied to each chosen
street." The approved Success state names a confirmation. Applying a design to
four streets in one action, with no confirmation naming what happened, is the
single largest silent state change in the product.

### 3. The two bad-news moments — US8.2 and US7.1/US7.2

The brief asks whether these are good enough to build a decent experience from.
US7.1/US7.2 are close (see 1c for the missing retry). US8.2 is not, for three
separate reasons.

**3a. AC8.2.1's `Given` is not detectable, and its failure mode is telling a
first-time visitor their work is gone.** The criterion reads *"Given I return
expecting a design and device storage no longer holds it."* The product cannot
observe "expecting". When browser storage is wiped, there is nothing left to say
a design ever existed — that is what wiped means. A build that implements this
literally has only one detectable signal available: empty storage. Empty storage
is also what a genuine first-time visitor has. Show them "your design is not on
this device" and you have opened the product by telling a stranger you lost
something of theirs.

The only honest detectable case is arriving at a route that *names* a specific
design which is not present — which requires designs to have addressable routes,
and no story says they do in Stage 1.

Fix: split it. One AC for the detectable case (a route referencing a named design
that device storage does not hold). One AC stating explicitly that a cold start
with empty storage and no referenced design shows the normal first-run prompt
from AC1.2.2, *not* a loss message. Without the second AC this is a coin-flip at
implementation time, and the wrong side of the flip is the worst first impression
the product can make.

**3b. AC8.2.2 promises a capability that a Should Have story delivers.** It
requires the message to explain *"that designs are kept on the device unless
uploaded."* Uploading is US8.3, which is Should Have. If US8.3 does not ship,
this Must Have criterion instructs the product to tell users about an action that
does not exist — from a message whose entire job is to be trusted.

This is worth calling a priority inversion rather than a wording bug. US8.2 exists
*because* browser storage vanishes without warning; the only mitigation in the
whole set is US8.3; and the mitigation is optional while the apology is mandatory.
Under the P3 anchor I would argue US8.3 is Must: a campaign that runs over weeks
(P3's stated frequency) on a device whose storage can be evicted, with no way to
put the work anywhere else, is not "a credible proposal without professional
tooling" — it is a proposal with a coin-flip attached.

Fix: raise US8.3 to Must Have, or rewrite AC8.2.2 so it does not name a
capability from a Should Have story and add the alternative copy for the
no-upload build.

**3c. Nothing covers storage failing mid-session, which US8.2's own INVEST note
names.** The note says storage can vanish through *"eviction under pressure"* —
and then no criterion covers it. AC8.2.3 covers storage known-unavailable at the
start; AC8.2.4 covers the tool staying usable. Neither covers: user edits for
twenty minutes, a save silently fails, they close the tab. Add an AC — a save
failure during a session is surfaced at the moment it happens, and the session's
work stays available in the open tab until it is closed.

**3d. AC8.2.3's Given/When defeats its own Then.** *"Given I am browsing in a
mode where storage is unavailable, When I make an edit, Then I am told **before I
invest work** that it will not persist here."* If the trigger is making an edit,
work has already been invested. Storage availability is detectable on load. Move
the trigger to opening the editor.

**3e. AC7.1.1 is not checkable as written.** *"it describes what happened to the
street, not the underlying error"* has no pass/fail test — which the inception
guardrail rules out. AC7.1.2's negative form (no stack trace, no library text, no
internal identifier) is checkable and is currently carrying the whole story.

Fix, and it costs nothing because the shape already exists in an affirmed rule:
`team.md` requires the adapter to map every failure to *"a small closed set of
typed failure reasons in the project's own error enum."* Make AC7.1.1 require the
message to name the street by the same name shown at selection, and to state
which member of that closed set applies. Now it is testable and the copy has a
finite surface a solo builder can actually write well.

**3f. No behaviour is defined when import exceeds NFR1.1's 10 seconds.**
AC3.1.2 asserts import completes within 10 seconds; AC3.1.3 shows progress.
Nothing says what happens at 10.1 seconds. Without a stated ceiling the user
watches a spinner indefinitely, which is a worse experience than a clean failure.
Fix: a timeout is a failure and routes into US7.1/US7.2 with the retry from 1c.

### 4. The provenance model contradicts itself between two Must Have criteria

**AC5.1.3 and AC5.4.3 cannot both hold.**

- AC5.1.3: *"Given a lane whose width was `mapped`, When I change that width,
  Then its provenance becomes `user-set` **and never returns to `mapped`**."*
- AC5.4.3: *"When I revert a single changed attribute, Then it returns to the
  imported baseline value **with its original provenance**."* For a width that
  was `mapped`, its original provenance is `mapped`.

The inception guardrail forbids carrying an unresolved contradiction forward, and
this one sits on the mechanism the whole product's credibility rests on.

AC5.4.3 is the correct behaviour and AC5.1.3 is the criterion to amend. Discarding
an edit is not a correction: the value genuinely is the OpenStreetMap-measured one
again, and marking it `user-set` would understate its authority to Priya — the
opposite failure from the one FR3.4 guards against, but a failure. What FR3.4
actually forbids is a *changed* value masquerading as measured.

Fix: AC5.1.3 to read that the provenance is `user-set` for as long as the changed
value stands, and AC5.4.3 to state that discarding an edit restores the imported
provenance *because the value is once again the imported one*, and that this is
distinct from US7.3's correction, which is permanent (AC7.3.1) precisely because
the value is not the imported one.

### 5. The MoSCoW anchor is stated well and applied inconsistently

The anchor is good: *"if a story is not needed for Marcus to produce a credible
proposal without professional tooling, it is not Must Have."* It is discriminating
in three places (US7.4, US8.3, US12.3 all correctly demoted with the rule cited).
It is then broken in three places without acknowledgement.

- **US11.3 (Must) versus US7.4 (Should).** Both are Dana's stories. US7.4's
  demotion reasoning is explicit: *"Serves the maintainer, not the primary
  persona, which is why it is Should rather than Must under the P3 anchor."*
  US11.3 serves the maintainer too and is Must Have. It is Must because it gates
  a public release under RC-1/RC-2, which is a legitimate reason and is not the
  anchor.
- **US2.3 and US5.3 (both Must).** Their actors — "someone using a keyboard or a
  screen reader", "someone using a keyboard, a screen reader, or a phone" — are
  not any of the four personas. No persona in `personas.md` has an access need;
  Marcus's tech comfort is low-to-medium, not assistive. So the two stories
  carrying the entire WCAG 2.1 AA commitment are Must Have for a person the
  anchor does not describe. They are Must because NFR4.1 is a standing
  commitment.
- **US1.1 (Must).** Marcus can produce a credible proposal without a landing page
  carrying a worked before-and-after example. AC1.1.2 in particular is a
  persuasion artifact, not a production capability. It is Must because reaching
  the tool at all is a precondition — again a legitimate reason, again not the
  anchor.

I am not proposing any of these be demoted. I am proposing the rule state its
exceptions, because a rule that is silently overridden three times cannot be used
by Delivery Planning to settle the next marginal story. Three declared classes:
what P3 needs to produce a proposal; what any user needs to reach the tool at
all; and standing commitments (WCAG 2.1 AA, and the RC-1/RC-2 release gate) that
are Must by policy rather than by persona.

The right fix for the accessibility half is the stated exemption, **not** adding a
screen-reader persona — that would be manufacturing research nobody did, against
the standing constraint on this stage.

### 6. Personas: honest, mostly distinct, one decorative, one claim not delivered

**Distinctness — yes, they hold.** P1/P2/P3/P4 pull in genuinely different
directions and I can point at where each changes a decision: P2 forces the whole
US4 group; P3 forces US1.2's no-tutorial constraint and US7.2's no-dead-end rule;
P1 forces US7.4. P2 and P4 are the closest pair but P2 *evaluates* what P4
*produces*, which is a real difference. This is better than the usual four-persona
set that collapses into one.

**Evidential honesty — strong, and the per-persona marking is the right call.**
The reasoning given for it ("a persona is the artifact most likely to be quoted
away from its file") is correct and I want it on the record as a practice.

**P4 is decorative.** `personas.md` says *"Stories reference P4 only where the
need is already in scope."* No story references P4 at all. The closest need —
"will only adopt something that gets data out again" — is US11.2, written as
Marcus. Either say plainly that P4 currently anchors no story and exists as a
standing caution about scope creep (which is a legitimate function, and the
"Standing caution" paragraph already does the work), or point it at US11.2. As
written the file makes a claim the story set does not honour.

**The coverage gap is recorded honestly and mitigated only on paper.**
`personas.md` says FR10 and FR11.1 *"are the requirements that serve"* elected
officials and passive readers, and that *"stories in those groups name this gap
where it bites."* Checking:

- US1.1 does serve them. AC1.1.2's before/after with a visible dimension is
  genuinely reader-facing. It carries no gap note, but it does the work.
- **US12 does not serve them at all.** Every criterion in US12.1 and US12.2 is
  written from the producer's side — *I* choose a purpose, *I* am told which
  purpose failed. Nothing tests anything about the person who *receives* the
  output at a meeting, arriving with no context. AC12.1.2's *"suitable for that
  purpose at the stated size or medium"* is about physical fitness for print, not
  comprehension.

And the material for the fix is already sitting in an approved screen: S8's
"Include" checkbox group — before and after, street location map, lane dimensions
table — exists precisely so the output is legible without the producer standing
next to it. **That checkbox group has no story.** Fix: add an AC to US12.1
requiring the output to identify the street and its location, and to state the
before/after relationship, without the producer present. That is checkable, it is
already drawn, and it converts the coverage-gap claim from paper to delivered.
US12.1's existing gap note is about OQ5 (no city has been asked) — a different
gap; add the persona one or trim the claim in `personas.md`.

**One persona consequence with no story: does a viewer need an account?** P2's
stated pain is *"not willing to install anything to read one proposal."* An
account is the same class of barrier. AC10.2.1 grants access to "a named person"
and no criterion says whether Priya must sign up to open what Marcus shared. If
she must, the product has put its own adoption barrier in front of the persona
whose acceptance is the differentiator. Settle it in US10.2 or hand it to Domain
Design as a named open question.

### 7. Keyboard-first: the structure is right, three commitments are missing

The select-then-act principle is carried correctly. US2.3 and US5.3 as separate
stories rather than buried ACs is the right call and AC5.3.2 ("a drag interaction
exists → the same action is reachable by selection") is the correct invariant
form. No story I found quietly assumes a pointer where a keyboard path is absent.
Three things are missing rather than wrong:

**7a. Nothing is announced. Every status in the set is written as visual only.**
WCAG 2.1 AA includes SC 4.1.3 Status Messages, NFR4.1 commits to AA including the
editing surface, and `traceability.json` marks NFR4.1 "Deferred → nfr-requirements"
— which is where this fell through, because SC 4.1.3 is user-visible behaviour a
story must carry, not a conformance target a later stage can set.

Every one of these is "I see" or "I am told", with no announcement: AC2.1.4
(search found nothing), AC3.1.3 (import in progress) and import completion
(nowhere), AC6.2.1 (fit warning), AC6.1.2 (corridor applied), AC8.3.1 (upload
succeeded), AC12.1.3 (output failed). Every one is a state change that happens
away from focus. A screen-reader user selects a street and hears nothing for up to
ten seconds (AC3.1.2), then nothing when it arrives.

Only two ACs get this right — AC2.3.3 and AC4.1.5 — and both are about focus
moving onto an element, which is the easy case.

Fix, matching the draft's own pattern of promoting a cross-cutting rule to a
story (US4.2 and US5.3 both do this): one invariant story — every state change
the product reports visually is also announced without moving focus. This holds
whichever way OQ2 is settled; a live region lives in the surrounding DOM even if
the editing surface is canvas.

**7b. NFR4.4's editing half is not asserted anywhere, and `traceability.json`
marks it covered when it is not.** NFR4.4 requires *"viewing **and editing** at
viewport widths from 360 CSS pixels upward."* `traceability.json` maps NFR4.4 →
US1.1, whose AC1.1.3 asserts 360 pixels **for the landing page only**. No story
asserts the cross-section, the lane list, the type control, the width stepper or
the corridor checklist work at 360.

This is load-bearing, not a detail. `user-flow.md` says the audience arrives
*"frequently on a phone."* The entire Option B layout recommendation was chosen
over A and C on the strength of *"one layout for every form factor."* The one
form factor that choice was made for has no acceptance criterion.

Fix: add an AC to US5.3 in AC1.1.3's exact form — the cross-section and every
editing control are operable at a viewport 360 CSS pixels wide without horizontal
scrolling of the page body — and correct the NFR4.4 row in `traceability.json`
to point at it. AC5.3.4's 44-pixel target is the other half of the phone story
and is already correct.

**7c. FR1.4 is narrowed to first-run.** `traceability.json` maps FR1.4 → US1.2,
and AC1.2.2 is the prompt criterion — but it opens *"Given I am in the tool for
the first time."* FR1.4 has no such qualifier: a prompt rather than a bare map
*whenever* nothing is selected. AC2.2.3 creates exactly that state on a later
visit ("the previous selection is released"), with no prompt required. Small fix,
real gap: drop the first-time qualifier, and add FR1.4 to US1.2's `Traces` line,
which currently lists only FR11.2 and FR7.2.

### 8. Criteria whose actor is the build, not a person

A cluster of ACs describe implementation rather than observable experience. The
inception guardrail requires each story to identify the actor; these have no human
one, and several pre-empt decisions Domain Design owns.

- **AC3.1.4**, **AC4.1.3** — *"Given a committed fixture … When the adapter
  processes it"*. The actor is the adapter. Both restate the golden-fixture suite
  that `team.md` already mandates.
- **AC4.1.4** — *"Then an automated test can assert that value is rendered
  distinguishably"*. The subject is a test. What a user experiences is already
  covered by AC4.1.2 and AC4.1.5.
- **AC3.1.5** — *"When the design is held in memory"*. Memory layout is not
  observable. The user-facing form of FR4.5 already exists as AC5.1.4 and AC5.4.1.
- **AC5.2.4** — *"When I inspect the stored design, Then the lane is identified
  by an identifier this product minted rather than by a position index"*. Nobody
  inspects a stored design. This is `team.md`'s overlay-key rule, and it decides
  a Domain Design question inside a story.
- **AC9.1.4** — cookie flags. Already in `team.md` Deployment.
- **AC7.1.3** — the maintainer's log, sitting in Marcus's story; it belongs in
  US7.4, which is Dana's.

None of these are wrong as engineering requirements. They are in the wrong
artifact, and keeping them here makes stories that describe the build rather than
the experience — which is precisely how a story set stops being reviewable by the
person it is written for. Move them to the test plan and Domain Design, or keep
them under an explicit "verification notes" heading that is not a Given/When/Then
criterion with a machine as its actor.

### 9. Corridor fit: the unknown-width case falls through as a silent pass

AC6.2.3 handles a target whose kerb-to-kerb width is `inferred` — good, and
exactly right. Nothing handles a target whose width is **unknown**: a street whose
import failed (US7.2 gives it a blank cross-section, so it has no kerb-to-kerb
width at all) or whose data was too thin to produce one. Such a street falls
through AC6.2.1 (no exceedance detected), and then AC6.2.4 fires: *"a design that
fits every chosen street … no warning is shown."*

The user is told the design fits a street nobody checked. That is the R-2
credibility failure in its purest form — presenting an unchecked thing as checked
— committed by the very feature built to prevent it.

Fix: add AC6.2.x — where a target street's width is unknown, the user is told the
fit could not be checked for that street, and it is not counted among the streets
the design fits. The distinction between "fits", "does not fit", "rests on an
inferred width" and "could not be checked" is four states, and the story currently
has two.

### 10. Stage 1 storage: one design, or many?

US8.1 says *"the design is there"*, singular, throughout. Nothing says whether
device storage holds one design or many. P3's persona is a campaign running over
weeks, returning to *"a design they made and half-remember"* — and US6.1 produces
a design spanning several streets, so the model already holds more than one
street's worth of work.

If Stage 1 holds one design, Marcus opening a second street destroys his first,
with no warning and no list to notice it from. If it holds many, Stage 1 needs a
way to find them again — and naming and listing designs is US9.3, which is
Stage 2. Either answer has a consequence and neither is written down.

This is a Must Have area, it is the walking skeleton's own territory, and it is
cheap to settle now and expensive to discover at B-0. Fix: an AC in US8.1 stating
whether starting work on a second street replaces or adds to what is stored, and
if it adds, the minimum Stage 1 affordance for returning to a specific one.

### 11. Smaller items, grouped

- **AC3.1.3 is weaker than the approved screen it implements.** wireframes.md
  Screen States, Loading: *"The drawer opens immediately with the street name and
  a skeleton cross-section, so the selection is acknowledged before the data
  arrives."* AC3.1.3 says only *"I see that it is working."* Given NFR1.1 permits
  ten seconds, immediate acknowledgement with the street's name is the commitment
  worth keeping, and it is already approved.
- **The partial state has no explanation.** wireframes.md commits to *"marked as
  estimated, **with a note that the source data did not specify them**"*. AC4.1.2
  requires visual distinction and AC4.1.5 requires announcement; neither requires
  the distinction to be *explained*. For P3 — low-to-medium tech comfort, no
  street-design vocabulary — an unexplained hatch pattern is decoration. Add an
  AC that the distinction is stated in words at least once per view.
- **AC4.2.3 and AC12.2.2 omit an unmarkable value; nothing makes the omission
  visible as one.** Omitting is the right call and I endorse it (see Positions).
  But a silent gap where a number was, on a printed board handed to a planner, is
  its own credibility problem. Require the omission to read as a deliberate
  omission rather than as a blank.
- **US6.1 does not require streets in the connected set to be identifiable.**
  AC2.2.2 correctly requires an unnamed street to get "something stable and
  human-readable". The corridor checklist has the same problem — connected
  segments are frequently unnamed — and no equivalent AC. S4 already draws them
  with names and lengths.
- **AC2.1.4 drops the recovery half.** user-flow.md: *"No results, with a
  suggestion to try a broader place name."* The AC keeps the dead end and drops
  the way out.
- **US9.1 has no decline path.** wireframes.md S5 draws *"[Not now - keep
  designing]"* and user-flow.md makes it a decision point (*"Declining must not
  cost the work in progress"*). FR7.2 backs it. US1.2's AC1.2.3 covers Stage 1;
  nothing covers dismissing the Stage 2 modal with work intact.
- **US9.3 has no empty state**, though S6 draws one ("No designs yet. [Find a
  street]") and AC1.2.2 shows the draft knows how to write one.
- **US12.3 is Should Have and S8 draws the preview inside the export dialog.** If
  US12.3 does not ship, the approved screen has a hole in the middle. Either note
  that S8 needs a preview-less variant or reconsider the priority.
- **Dependency graph defects.** US1.2 appears nowhere in it. US3.2 is drawn under
  US3.1's branch but its stated dependency is US2.1. US2.2 is drawn as depending
  on US1.1 but its stated dependency is US2.1 alone.

---

Nothing above requires user research, a design team, or spend. Every fix is a
criterion edit, a story addition, or a correction to an approved wireframe — all
within reach of one builder.

## Positions

AGREE: Per-persona hypothesis marking rather than a header disclaimer is the right call, and the stated reason — a persona is the artifact most likely to be quoted away from its file — should survive into later stages as a practice.
AGREE: P1's "What this persona is not for" paragraph (Dana's fluency is the reason not to design for Dana) is the sharpest thing in `personas.md` and the kind of note that actually changes a decision.
AGREE: The four personas are genuinely distinct — each of P1, P2 and P3 can be pointed at a story that exists because of it — which is better than most four-persona sets manage.
AGREE: US2.3 and US5.3 as full stories rather than accessibility criteria buried inside US2.2 and US5.1; AC5.3.2's invariant form (a drag exists → the same action is reachable by selection) is the correct way to state FR4.4.
AGREE: AC4.2.3 and AC12.2.2 — omit an unmarkable value rather than show it unmarked — is the correct and braver call, and it is the criterion that makes the provenance commitment real rather than decorative.
AGREE: AC5.3.4 holds the 44-pixel activation target while explicitly keeping the lane's drawn width unchanged, correctly carrying NFR4.5's reasoning that rendered width is data.
AGREE: AC6.2.5 ties the width-only fit rule to the jurisdiction-neutral mandate in the criterion itself, which is what stops a later stage quietly adding a lane-type compatibility table.
AGREE: US8.2 exists at all — the story about work being gone is the one most sets omit entirely, and AC8.2.4 (the tool stays usable when storage is unavailable) is right.
AGREE: AC2.1.3 improves on user-flow.md by adding "does not ask again in the same session" — a real interaction commitment the flow did not make.
AGREE: OQ-US1 and OQ-US2 are raised against `requirements.md` rather than an approved requirement being quietly reinterpreted in place.

OBJECT: AC6.2.2 contradicts wireframes.md S4's "Lanes will be scaled to fit" — the story is right, but the approved screen must be corrected, because a scaled width has no valid provenance state under FR3.1.
OBJECT: US10.2's independent named-access and link mechanisms contradict wireframes.md S7's three mutually exclusive radio options, and no AC says what enabling a link does to existing named access.
OBJECT: Retry after a failed import is committed by both wireframes.md S3 and user-flow.md's recovery table, and appears in no story — forcing a blank cross-section after a transient network failure discards the street the user came for.
OBJECT: Undo is exercised by AC5.3.3, required as an editing action by `team.md`'s merge gate, and promised by wireframes.md's Success state, but is defined by no story; AC5.4.3's revert-to-baseline is a different operation.
OBJECT: AC6.1.2 ends at "the design is applied to each chosen street" with no confirmation, making a corridor apply the largest silent state change in the product against an approved Success state.
OBJECT: AC8.2.1's Given ("I return expecting a design") is not detectable — the only available signal is empty storage, which is also a first-time visitor, so the criterion as written risks telling strangers the product lost their work.
OBJECT: AC8.2.2 (Must Have) instructs the product to explain that designs are kept on the device "unless uploaded", where uploading is US8.3 (Should Have) — the apology is mandatory and its only mitigation is optional.
OBJECT: US8.3 should be Must Have under the P3 anchor — a weeks-long campaign on evictable device storage with no way off the device is not a credible proposal, it is a coin flip.
OBJECT: US8.2 has no criterion for storage failing mid-session, the case its own INVEST note names ("eviction under pressure").
OBJECT: AC8.2.3 triggers on "When I make an edit" but promises to warn "before I invest work" — storage availability is detectable on load, so the trigger should be opening the editor.
OBJECT: AC5.1.3 ("never returns to `mapped`") and AC5.4.3 ("returns to the imported baseline value with its original provenance") are a literal contradiction between two Must Have criteria on the product's central credibility mechanism.
OBJECT: The MoSCoW anchor is applied inconsistently — US11.3 is Must for the same maintainer reason US7.4 was demoted, US2.3/US5.3 are Must for an actor no persona describes, and US1.1 is Must for a reason the anchor does not cover; the rule needs its three exception classes declared, not the stories demoted.
OBJECT: No story announces any status change — AC2.1.4, AC3.1.3, AC6.1.2, AC6.2.1, AC8.3.1 and AC12.1.3 are all visual-only, against NFR4.1's WCAG 2.1 AA commitment, which includes SC 4.1.3 Status Messages.
OBJECT: NFR4.4's editing half is asserted by no story — AC1.1.3 covers only the landing page at 360 CSS pixels, while `traceability.json` marks NFR4.4 "OK" against US1.1; the form factor Option B was chosen for has no acceptance criterion.
OBJECT: AC1.2.2 narrows FR1.4 to "the first time", but AC2.2.3 creates the nothing-selected state on every later visit with no prompt required, and US1.2's `Traces` line omits FR1.4 that `traceability.json` assigns to it.
OBJECT: AC3.1.4, AC4.1.3, AC4.1.4, AC3.1.5, AC5.2.4 and AC9.1.4 have no human actor — their subjects are the adapter, a test, memory layout, a stored identifier scheme and a cookie flag — and several pre-empt Domain Design decisions from inside a story.
OBJECT: AC6.2.3 covers an `inferred` target width but nothing covers an unknown one, so a street whose import failed falls through to AC6.2.4 and is reported as fitting a design nobody checked.
OBJECT: US8.1 says "the design" throughout without settling whether Stage 1 device storage holds one design or many, though US6.1 already produces multi-street work and P3's persona returns to designs over weeks.
OBJECT: US12 carries no reader-side criterion at all, so `personas.md`'s claim that FR10 serves the unmodelled elected-official and passive-reader group is not delivered; S8's "Include" checkbox group, which exists for exactly that reader, has no story.
OBJECT: `personas.md` states "Stories reference P4 only where the need is already in scope", but no story references P4 anywhere — either say it anchors no story, or point it at US11.2.
OBJECT: No AC settles whether a person Marcus shares a design with needs an account to view it, which lands directly on P2's stated barrier ("not willing to install anything to read one proposal").
OBJECT: AC7.1.1 ("describes what happened to the street") has no pass/fail test; `team.md`'s closed set of typed failure reasons is the checkable form and costs nothing to adopt.
OBJECT: No behaviour is defined when import exceeds NFR1.1's 10 seconds, leaving an unbounded spinner where a clean failure with retry belongs.
OBJECT: AC3.1.3 is weaker than the approved Loading state it implements, dropping wireframes.md's commitment that the drawer opens immediately with the street name before data arrives.
OBJECT: The partial state has no criterion requiring the mapped/inferred distinction to be explained in words, though wireframes.md commits to a note and P3 has no street-design vocabulary to decode a visual marking with.
OBJECT: Smaller gaps against approved artifacts, each one line to fix — AC2.1.4 drops user-flow.md's "try a broader place name" recovery; US9.1 has no decline-sign-in path though S5 draws one; US9.3 has no empty state though S6 draws one; US6.1 does not require connected streets to be identifiable the way AC2.2.2 requires for map selection; the dependency graph omits US1.2 and misplaces US3.2 and US2.2 against their stated dependencies.
