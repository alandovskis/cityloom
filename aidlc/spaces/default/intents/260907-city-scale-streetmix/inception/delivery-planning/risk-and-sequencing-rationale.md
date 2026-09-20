# Risk and Sequencing Rationale — Streetmix at City Scale

Upstream inputs: `requirements.md` (requirements-analysis), `stories.md`
(user-stories), `mockups.md` (refined-mockups), `components.md`
(domain-design), `unit-of-work.md`, `unit-of-work-dependency.md` and
`unit-of-work-story-map.md` (units-generation), `contract-summary.md`
(contract-design), `team-practices.md` (practices-discovery).

Why the **Bolts** — each one a build pass over a piece of the work, ending in
something that runs — are ordered the way `bolt-plan.md` orders them.

## The heuristic

**Walking-skeleton-first, then value-first by product stage.**

- **Walking skeleton** (Cockburn, *Crystal Clear*) — the first Bolt is a
  minimal end-to-end slice that proves the architecture works before features
  are added to it. This was not chosen here; `team-practices.md` had already
  affirmed it, named the skeleton B-0, and defined its pass criterion.
- **Value-first** — after the skeleton, build in the order the product stages
  already set, finishing product Stage 1 before Stage 2 or Stage 3 wherever
  the dependency graph allows. Chosen at Q2.

## Why not the alternatives

**Risk-first** (Boehm) was the closest competitor and is partly in the plan
anyway. Its argument is to take the most uncertain work early so later work
rests on settled ground. But the single Critical uncertainty — whether
osm2streets returns usable lane geometry for real streets — is already what
B-0 exists to retire, and the walking skeleton is therefore doing risk-first's
job for the risk that matters most. The remaining uncertainties are the
corridor correspondence rule (B-6), the basemap identity constraint (pulled
forward into B-0 by Q6), and the proxy's real egress cost (B-3, early by
dependency rather than by design). Applying risk-first on top of that would
reorder the tail of the plan to chase uncertainties that are smaller than the
disruption of reordering.

**WSJF** (Reinertsen's Cost of Delay ÷ Duration, as used in SAFe) — scoring
each piece of work as (user-business value + time criticality + risk
reduction) ÷ job size and shipping the highest score first — was rejected on
an honest reading of its inputs. Two of the three numerator terms do not apply
here. Time criticality needs a deadline, and `constraint-register.md` OC-3
records that there is none. Risk reduction is already spent on B-0. That
leaves value ÷ size, which is value-first with arithmetic wrapped around it,
and the arithmetic would invite more confidence in the ordering than the
inputs support.

**Dependency order alone** would have been defensible and is close to what the
graph forces anyway. It was not chosen because it makes no claim about what is
worth proving, and two decisions in this plan — pulling the basemap spike into
B-0, and bundling the Stage 2 Units so the access gate is never built without
its opener — are claims of exactly that kind.

## Deviation from topological order

**There is none.** The Bolt order in `bolt-plan.md` is a valid topological
order of the twelve Units. This was checked by parsing the fenced edge block in
`unit-of-work-dependency.md`, deriving each Unit's position from the Bolt
table as written, and comparing every dependency's position against its
dependant's.

That check earned its place. A first draft of the Bolt table placed
`local-persistence` before `design-storage`, and `client-surfaces` before both
`meeting-output` and `accounts-sharing` — three Bolts scheduled ahead of work
they depend on. The prose in that draft even stated one of the three
dependencies correctly in a sentence directly above the table that contradicted
it. Reading did not catch it; parsing did.

## Where the graph overruled the intent

Value-first says finish product Stage 1 before Stage 2 and Stage 3. The graph
does not allow it, in two places, and both are recorded rather than smoothed
over:

| Intent | What the graph forces | Why |
|---|---|---|
| Stage 1 surface early | `client-surfaces` is **last** (B-11) | It is the graph's single sink. `AppShell` presents the export surface and the sharing surface, so the Unit holding it cannot be finished until Stage 3's `meeting-output` and Stage 2's `accounts-sharing` exist |
| Stage 2 server work late | `design-storage` is **B-7**, in the middle of Stage 1 | US8.3 ("keep a design that is not tied to one device") is a Stage 1 Must Have, so anonymous upload storage is Stage 1 work, and `local-persistence` depends on it |

The consequence worth stating plainly: **the product is not usable until the
last Bolt.** Every Bolt before B-11 completes a Unit with no surface to exercise
it. That is the cost of a decomposition where all three views live in one Unit —
which `unit-of-work.md` records as forced by the acyclicity constraint, not
chosen.

**B-0 is what makes that acceptable.** The walking skeleton puts a working
end-to-end path on screen before any of the eleven, so the intervening Bolts
are filling in something that already runs. Without B-0 this plan would have
one demo, at the end, after eleven gates with nothing to look at. That is the
strongest argument for the skeleton in this particular plan, and it is separate
from the osm2streets risk the skeleton was originally affirmed to retire.

## Risks this sequence is aimed at

Q7 — what worries you most — was put twice and not answered; the record in
`delivery-planning-questions.md` says so in those terms. The plan therefore
carries **no additional risk weighting beyond what was already decided**, and
this section states what that leaves rather than inventing a risk appetite.

| Risk | Source | Where the plan addresses it | Standing |
|---|---|---|---|
| osm2streets does not return usable lane geometry for real streets | `raid-log.md` D-1, R-1 — the project's single Critical dependency | B-0, and confirmed by B-1's fixture suite and B-4's characterisation tests | Deliberately targeted. This is what the skeleton is for |
| A guess is presented as a measurement | `raid-log.md` R-2 | B-0's third pass criterion; the type-level provenance in B-2; the three rendering channels in B-11; the omit-rather-than-mislead rule in B-9 | Deliberately targeted, at four points |
| The basemap cannot supply stable OpenStreetMap way identities | `components.md`; AC2.2.4 | Pulled into B-0 as a spike by Q6, rather than left to Infrastructure Design when `client-surfaces` comes up | Deliberately targeted, by this stage's own decision |
| The proxy's egress breaks the ~$5/month budget | `project.md`; `contract-summary.md` open questions | Measured in B-3 against one real street and recorded | Addressed where the work lands, not pulled forward |
| The corridor rule does not match usefully in practice | `raid-log.md`; the product's central claim | B-6, at its natural position in the order | Not pulled forward. A risk-first plan would have moved it earlier |
| WCAG 2.1 AA on an editing surface is harder than designed | `requirements.md` NFR4; `team-practices.md` | B-11, last | Not pulled forward, and it is the largest untested commitment in the plan |
| The project stalls rather than fails | Not recorded upstream — the structural risk of twelve Bolts, sixty-odd gated stages and one builder | Nothing in this plan addresses it | Named here and not addressed |

**The last two rows are the honest cost of an unanswered Q7.** Had the
accessibility commitment been named as the chief worry, a keyboard and
screen-reader pass would have been pulled into an earlier Bolt rather than
waiting for B-11. Had stalling been named, the plan would have favoured fewer
and more visible milestones over a Bolt per Unit. Neither was, so neither was
done, and both remain available to a later revision of this plan.
