# Delivery Planning — Questions

Upstream inputs: `requirements.md` (requirements-analysis), `stories.md`
(user-stories), `mockups.md` (refined-mockups), `components.md`
(domain-design), `unit-of-work.md`, `unit-of-work-dependency.md` and
`unit-of-work-story-map.md` (units-generation), `contract-summary.md`
(contract-design), `team-practices.md` (practices-discovery).

**What this stage decides.** The order in which the work gets built, as a
sequence of **Bolts** — a Bolt being one build pass over a piece of the work,
ending in something that runs. Units Generation produced the dependency graph,
which says what *can* come before what; this stage chooses a path through it,
which is a judgement about what is worth proving first and cannot be read off
the graph.

## What is already settled and not re-asked here

- **The first Bolt is the walking skeleton** — a thin end-to-end slice that
  proves the architecture hangs together before features are added to it.
  `team-practices.md` names it B-0 and defines it: one street imported, edited
  and saved. Its pass criterion is three things holding at once — the import
  produces the right lane count and order, an edit survives save and reload
  identically, and mapped values are visibly distinguished from inferred ones.
- **Every Bolt is gated through product Stage 1** — you approve each one.
  `team-practices.md` sets that deliberately, because the gates cost your time
  rather than a reviewer's, and there is no deadline pushing the other way.
- **Bolts are squash-merged to `main`, one commit per Bolt**, trunk-based,
  no release branches.
- **There is one person building this.** Team Formation was skipped, so there
  is no roster and no mob to allocate; every Bolt is you working with AI
  support.

So the questions below are about what the affirmed practices do *not* settle.

---

## Q1. B-0 touches seven of the twelve Units. What does that make it?

This is the question this stage exists to answer, and the plan's shape follows
from it.

A Bolt is defined as one or more Units of Work. But B-0 — one street imported,
edited and saved, with provenance visible — needs something from
`street-core`, `osm2streets-build`, `osm-extract-proxy`, `street-import`,
`design-editing`, `client-surfaces` and `local-persistence`. That is seven of
the twelve, including `client-surfaces`, the largest Unit in the set.

A. **B-0 is a thin vertical slice: it partially implements seven Units and
   completes none of them.** Each of those Units is finished by a later Bolt.
   This is what a walking skeleton normally is, and it keeps B-0 small enough
   to be the risk-retirement exercise it is meant to be — but it means a Unit
   is no longer the thing a Bolt completes, and several Units stay open across
   many Bolts.

B. **B-0 bundles those seven Units in full.** The Unit boundary and the Bolt
   boundary stay aligned. But B-0 becomes most of product Stage 1 in one pass,
   which is the opposite of a thin slice, and the osm2streets risk it exists to
   retire would not be tested until very late in it.

C. **Narrow B-0 to the two or three Units that carry the actual risk** —
   `osm2streets-build`, `street-import` and `street-core` — and prove the
   import and provenance halves of the criterion without the edit-and-reload
   half. The saved-and-reloaded criterion moves to the second Bolt.

X. Other (please specify)

[Answer]: A

---

## Q2. After B-0, what decides the order?

The dependency graph permits many orderings. Something has to choose among
them, and `team-practices.md` does not say which.

A. **Risk-first.** Take the most uncertain work early, so later work is built
   on settled ground. After osm2streets, the open uncertainties are the
   corridor correspondence rule, the basemap choice, and the proxy's real cost.

B. **Value-first, following the product stages.** Finish everything in product
   Stage 1 — a usable single-street and corridor editor — before touching
   Stage 2 accounts or Stage 3 output. `scope-document.md` already stages the
   product this way; this makes the Bolt order follow it.

C. **A scoring model** — WSJF-style, where each piece of work scores (value +
   urgency + risk reduction) ÷ size and the highest score goes first. Explicit
   and defensible, but it needs numbers you would have to supply for twelve
   Units, and with one builder and no deadline the "urgency" term is close to
   meaningless.

D. **Dependency order, and no more.** Build in an order the graph allows and
   do not optimise further. Simplest; makes no claim it cannot support.

X. Other (please specify)

[Answer]: B

---

## Q3. How big is a Bolt after B-0?

A. **One Unit per Bolt.** Eleven more Bolts after the skeleton. Each ends with
   one Unit genuinely finished, and the gate is a real checkpoint about a
   coherent thing.

B. **Bundle related Units.** Roughly five or six Bolts — for example the three
   client model Units together, the Stage 2 server Units together. Fewer gates,
   but a Bolt's Definition of Done covers more ground and a failure is harder
   to localise.

C. **Thin slices across Units, like B-0.** Every Bolt is a user-visible
   capability that cuts through several Units. Best demos, and the Unit
   boundary stops being what a Bolt completes at all.

X. Other (please specify)

[Answer]: A

---

## Q4. Design everything first, or design and build one Unit at a time?

Construction runs five stages per Unit — functional design, NFR requirements,
NFR design, infrastructure design, and code generation. They can run in either
of two orders, and the choice changes when the first working code exists.

A. **One Unit at a time, all the way through.** A Unit is designed and built
   completely before the next one starts. The first running code arrives after
   one Unit's design rather than after all twelve — which is what a
   skeleton-first plan is usually asking for. Gates arrive in a cascade at the
   end of each Unit's block.

B. **Each design stage across every Unit, then build.** Functional design for
   all twelve, then NFR requirements for all twelve, and so on, with code
   generation last. Every design decision is made with the whole picture
   visible, and cross-Unit inconsistencies surface during design rather than
   during the build. But no code runs until the design work is finished.

X. Other (please specify)

[Answer]: A

---

## Q5. Three amendments to approved artifacts are outstanding. Before Construction, or carried?

Contract Design recorded three, and `decisions.md` ADR-001 already carried
seven before them. The three that matter most to Construction:

- `components.md` — nothing calls `DataRightsService`, yet erasure and export
  are user-facing Must Haves gating public release.
- `unit-of-work-dependency.md` — the graph is missing
  `client-surfaces → data-rights` and `data-rights → design-payload-spec`.

And from ADR-001, still open: `requirements.md` has no requirement behind
US5.5 (undo), US8.2 (work gone from this device) or US8.3 (keep a design off
this device) — three product Stage 1 Must Haves — and FR9.3 asserts an expiry
over designs the product never receives.

Construction reads `requirements.md` and the dependency graph directly.

A. **Apply them now, before Construction starts.** They are cheaper to fix
   before code exists than after, which is what ADR-001 itself says. Costs a
   jump back through approved stages and their gates.

B. **Carry them, and apply each one when the Bolt that needs it comes up.**
   The `data-rights` edges matter only when that Unit is built; the missing
   requirements matter when their stories are built. Nothing is lost as long
   as the record is read.

C. **Apply only the ones that block product Stage 1** — the three missing
   requirements — and carry the `data-rights` items to Stage 2.

X. Other (please specify)

[Answer]: C

---

## Q6. The basemap tile service is unchosen, and it blocks Stage 1 map work

`components.md` names a basemap tile service as an external dependency of
`MapView` and leaves the choice to Infrastructure Design. But it carries a
constraint most tile services do not meet: it has to supply stable
OpenStreetMap way identities so a street can be selected on the map
(AC2.2.4). Without it, US2.2 ("Select a street") has no mechanism — a product
Stage 1 Must Have.

Infrastructure Design runs per Unit during Construction, so under some plans
that choice would not be made until `client-surfaces` comes up.

A. **Treat it as a spike before or inside B-0.** Prove one tile service meets
   the identity constraint early, because a no on that question changes the
   map design and possibly the selection model.

B. **Leave it to Infrastructure Design for `client-surfaces`**, as the
   component catalogue says. It is a real risk but a bounded one, and B-0 may
   not need the map at all depending on Q1.

C. **Decide it now, in this plan**, and record the choice as a constraint
   Construction inherits.

X. Other (please specify)

[Answer]: A

---

## Q7. What worries you most about this build?

So the plan tackles it early rather than discovering it late. Pick as many as
apply.

A. **osm2streets does not do what we think it does** — the lane data comes
   back wrong, thin, or not at all for real streets. `raid-log.md` records it
   as the project's single Critical dependency.

B. **The accessibility commitment is harder than planned** — WCAG 2.1 AA on
   an editing surface, with every action keyboard-operable and every lane
   focusable and announceable, is a large commitment concentrated in one Unit.

C. **The cost runs past the ~$5/month budget** — the proxy's egress is
   unmeasured, and `project.md` treats growth past that budget as a constraint
   change rather than something to absorb.

D. **The corridor feature does not work in practice** — matching a design onto
   a structurally different street by lane type and ordinal may produce
   unmatched changes often enough to undermine the product's central claim.

E. **Losing the thread** — twelve Units, sixty-odd gated stages, one builder,
   no deadline. The risk is the project stalling rather than failing.

X. Other (please specify)

[Answer]: X. Other (please specify) — not answered. Asked twice; the user replied "continue". Recorded as a decision not to weight the plan by worry: the Bolt order follows Q2's value-first sequence, and the only risk aimed at deliberately is the one B-0 already targets (osm2streets) plus the basemap spike from Q6.

---

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
