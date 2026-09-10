# Build vs. Buy vs. Extend — Streetmix at City Scale

## The Question

The upstream `intent-statement.md` deliberately left this open: the product
boundary was confirmed as one substantial build "whether that lands as a new
product or as an extension of the existing Streetmix codebase". The current
leaning recorded at this stage is that the question is genuinely open — assess
both against real criteria and let the evidence decide (Q5).

This assessment applies the standard build/buy criteria to three options: **buy**
(adopt or licence an existing product), **extend** (build on the Streetmix
codebase), and **build** (a new product borrowing the interaction idea but not
the code).

## Option 1 — Buy: Ruled Out

Buying is not viable, and the reason is the intended model rather than any
product shortcoming.

The intended commercial model is open source and free, sustained by grants,
sponsorship or goodwill (Q7). The addressable audience is advocates and the
public rather than institutions (Q6). The commercial products in the comparison
set — Remix Streets and Aimsun — sell to agencies and institutions
[1][2]. Licensing a commercial product and redistributing it free to the public
is not a model those products are sold under. [assumption]

The free products that could in principle be adopted rather than extended
(StreetPlan.net, StreetDesign.ai) are section-level tools, so adopting one would
not close the gap the initiative exists to close [3][4].

**Verdict: not viable.** The remaining decision is extend vs. build.

## Option 2 — Extend the Streetmix Codebase

### The licence, which is the decisive fact

Streetmix is licensed **GNU Affero General Public License v3 or later**. The
licence file records that the original codebase was BSD 3-clause and that the
current codebase and all future modifications are AGPL-3.0-or-later [5].

This has three consequences, and they point in different directions:

- **It permits extending.** An open-source, free product (Q7) is exactly what
  AGPL is designed to accommodate. There is no licensing obstacle to the intended
  model.
- **It obliges reciprocity over a network.** AGPL's distinguishing feature is that
  it applies to software offered as a network service, not only to distributed
  copies. A hosted derivative would carry a source-availability obligation. For
  the stated model this is a non-issue; it is recorded because it is a
  constraint, not because it is a problem.
- **It forecloses open core.** Q7 selected open source and free, so no conflict
  exists today. But if the model were ever revisited toward open core — free for
  the public, paid institutional features — AGPL over an extended codebase would
  make that a relicensing conversation with Streetmix LLC rather than a product
  decision. This is the one durable cost of the extend path. [assumption]

### The rest of the case

**For extending:**

- The section-level editing experience is table-stakes (Q3), and Streetmix is a
  mature implementation of exactly that, with institutional users including NYC
  DOT, Transport for London and SFMTA [6]. Rebuilding table-stakes is effort that
  buys no differentiation.
- Its purpose statement — a web version of the paper cut-out street design
  activity, promoting two-way communication between planners and the public [7] —
  is aligned with the audience selected at Q6.
- Two projects have already extended it (StreetDesign.ai, StreetPlan.net), which
  is weak evidence that extending is practical rather than theoretical [3][4].

**Against extending:**

- Streetmix's README states the project focuses on individual street sections,
  "not broader networks or maps" [7]. The extension is not an addition at the
  edge of the existing scope; it is a move to a level the project explicitly does
  not address. Whether the existing design accommodates that move is a technical
  question this phase cannot answer. [assumption]
- Streetmix is maintained by Bad Idea Factory with contributor support [7], and
  no governance process for accepting a change of this magnitude was established.
  Extending in a fork avoids that question but forgoes the upstream relationship.
  [assumption]

## Option 3 — Build New

**For building:**

- Network-scale design is the differentiating capability, and starting from a
  section-oriented foundation may constrain it. A fresh start optimises for the
  thing that matters rather than inheriting a shape built for something else.
  [assumption]
- No licence inheritance, so the commercial model stays open in future.
- No dependence on another project's governance or direction.

**Against building:**

- Everything table-stakes has to be built before anything differentiating can be
  demonstrated. Against the success metric in `intent-statement.md` — a corridor
  redesign that takes about a week of planner work today taking under a day —
  this is the slower path to a testable product. [assumption]
- The comparison set shows the open-source network-scale slot is occupied but
  quiet: A/B Street is map-grounded and open source, and its own documentation
  records development slowing as of January 2024 [8]. Building new without
  examining whether A/B Street is a better foundation than Streetmix would be an
  incomplete assessment.

## Where This Leaves the Decision

Applying the standard rule — build what differentiates, do not rebuild what does
not — points toward **extending rather than building**, because the table-stakes
(Q3) are precisely what Streetmix already provides and the differentiator is
what neither option provides yet.

That is a leaning, not a recommendation this stage can properly make. Two
questions decide it and both are technical, which puts them in Feasibility rather
than here:

1. Can a section-oriented foundation carry network-scale design, or does the move
   to networks and a real map amount to a different product wearing the same
   interface?
2. Is Streetmix the right foundation at all, or is A/B Street — already
   map-grounded and network-scale, and apparently quiet [8] — the better base?

**Recommendation to the next stage:** carry both the extend and build options
forward into Feasibility, add A/B Street as a third candidate foundation, and
treat the AGPL obligation as a confirmed constraint rather than an open question.
No option should be closed at this stage.

## Assumptions & Open Questions

- The claim that commercial products would not licence for free public
  redistribution is inferred from their institutional sales model, not from
  reading their licence terms. [assumption]
- Whether Streetmix's existing design can accommodate network-scale work is the
  central unknown and is not answerable at the ideation level. [assumption]
- A/B Street was identified late as a possible third foundation and has not been
  assessed on the same criteria as Streetmix. [assumption]
- No effort or cost estimate was produced for any option; the build/extend
  comparison is qualitative. A three-year total cost comparison, which the
  standard build-vs-buy method calls for, was not attempted because no team size,
  rate, or timeline has been established. [assumption]
- Streetmix's governance was characterised from its README only; no maintainer
  was contacted and no contribution or forking conversation has taken place.
  [assumption]

## Sources

1. [Remix Streets Software (Via)](https://ridewithvia.com/solutions/remix/streets)
2. [SUMO — Simulation of Urban MObility](https://sumo.dlr.de/pdf/dkrajzew_MESM2002_SUMO.pdf); Aimsun characterised as a commercial hybrid traffic modelling simulator in the same comparison literature
3. [A Better Streetmix Alternative — Introducing StreetDesign.ai (Beyond CAD)](https://beyondcad.com/streetmix-alternative/)
4. [StreetPlan.net — Free StreetMix Alternative, by Urban Innovators](https://www.urbaninnovators.com/streetplan)
5. Streetmix `LICENSE` file, read via the GitHub API at `repos/streetmix/streetmix/contents/LICENSE`: "The original license for the Streetmix codebase was the BSD 3-clause license. The current codebase, and all future modifications to it, are licensed under the GNU Affero General Public License v3 (or later)."
6. [Streetmix — Urbanism Next](https://www.urbanismnext.org/resources/streetmix)
7. [streetmix/streetmix on GitHub](https://github.com/streetmix/streetmix) — README scope, purpose and maintainer statements
8. [A/B Street — project documentation](https://a-b-street.github.io/docs/software/abstreet.html) and [a-b-street/abstreet on GitHub](https://github.com/a-b-street/abstreet)
