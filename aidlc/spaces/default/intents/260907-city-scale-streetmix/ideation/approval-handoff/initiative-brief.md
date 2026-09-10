# Initiative Brief — Streetmix at City Scale

The one-pager for the Ideation to Inception handoff. Compiled from
`intent-statement.md`, `stakeholder-map.md`, `competitive-analysis.md`,
`feasibility-assessment.md`, `constraint-register.md`, `scope-document.md`,
`intent-backlog.md`, and `wireframes.md`.

## The Initiative

**Problem.** Existing street-design tools stop at the single cross-section.
Streetmix and its alternatives let someone compose one slice of roadway, and
nothing fills the gap above that level. What is needed is designing corridors and
networks on a real map — whole streets and how they connect to one another,
grounded in actual geography, rather than one disconnected slice at a time.

**Who for.** City and municipal transportation and planning staff; community
advocates, neighbourhood groups and the general public; planning and engineering
consultants working for cities. The addressable audience is primarily the
advocates and the public, who are more numerous and easier to reach than
institutions.

**The differentiator.** Producing outputs a city will actually accept in a formal
process. This is precisely what the advocate audience lacks: `intent-statement.md`
records their pain as having no credible way to propose a street change that a
city will take seriously.

**Success measure.** A corridor redesign that takes a planner about a week of
work today should take under a day in the tool.

**Why now.** A gap in existing tools rather than an external deadline.

## Market Validation

`competitive-analysis.md` compared nine products across four categories. The
finding is that the gap is real but narrower than "nothing exists":

- **Section-level design is well served and free.** Streetmix and two derivatives
  occupy it. Streetmix's own README states its scope as individual street
  sections, "not broader networks or maps" — the incumbent confirming the gap.
- **Network-level design exists but is uncontested.** Remix Streets holds it
  commercially; A/B Street holds it in open source but has been dormant since
  September 2025.
- **Participation platforms collect comments, not designs.** Konveio and Social
  Pinpoint let residents react to a plan someone else drew.
- **The unoccupied position is the join**: free, map-grounded, network-scale
  design whose output an institution will accept.

The evidence is vendor and project descriptions rather than hands-on evaluation.
This was accepted with that caveat, to be revisited if a competitor turns out to
cover more than its marketing suggests.

## Feasibility and Risk

**Verdict: technically viable, with one significant dependency risk.**

Viable for one person with AI assistance on roughly $5 a month because the
hardest technical problem — turning OpenStreetMap data into lane-level street
geometry — has an existing, permissively licensed solution. osm2streets
(Apache-2.0) provides a schema in which each road carries lanes left-to-right
with type, direction and width. That is structurally a Streetmix cross-section
derived from OSM.

**Foundation decision:** build new against the osm2streets schema, with Streetmix
as an interaction-design reference rather than a code dependency. Extending
Streetmix was assessed seriously and set aside: its scope mismatch inverts the
core rather than extending it, osm2streets already supplies the hard part of the
model, extending would mean reconciling two schemas, and AGPL inheritance is
permanent. A/B Street the application was assessed and set aside as dormant; its
ecosystem is adopted through osm2streets.

**The three risks that would change the plan, all accepted:**

| Risk | Why it matters |
|------|---------------|
| osm2streets does not integrate as expected | Published packages are roughly three years old against an October 2025 repository; the WebAssembly binding may need building from source. If this fails, the foundation decision reopens |
| Inferred cross-section quality is poor where OSM tagging is thin | OSM defines `width` as carriageway kerb-to-kerb excluding sidewalks, and sidewalk width is largely unmapped. Derived cross-sections rest on defaults where tagging is thin |
| osm2streets becomes unmaintained | Quiet for about eleven months; its parent application dormant for a year. Apache-2.0 permits forking, but a solo builder would inherit a Rust and WebAssembly component they did not write |

The first is retired by B-0, the walking skeleton, before anything else is built.

**Regulatory position.** User accounts and saved designs tied to people, offered
to the general public with no geographic restriction, put this in scope for GDPR
and comparable regimes from the first public release. Data subject rights are
therefore a **gate on public availability** rather than a deferred capability. No
payment data and no special-category data, so PCI-DSS and HIPAA are out of scope.

## Scope Boundary

**Three delivery stages**, with the walking skeleton first.

| | Contents |
|---|---|
| **B-0 skeleton** | One street imported, edited and saved. Retires the osm2streets risk. Persistence without accounts |
| **Stage 1** | Map view; street import via osm2streets; cross-section editor; corridor and network editing; **hero landing page**; **visual identity**. The minimum viable scope — at the end of it a design exists |
| **Stage 2** | Accounts; saved designs with access-controlled sharing. Gated for public release on data subject rights |
| **Stage 3** | Meeting-ready output — exports, printable plans, presentation views |
| **Should Have** | Pluggable jurisdiction standards; city GIS import and export |

**Out of scope this time:** traffic simulation and modelling; construction- or
engineering-grade output; editing the underlying OpenStreetMap data.

**Sequencing:** the skeleton first regardless of value order, then value-first.
The disagreement `intent-backlog.md` left open — whether meeting-ready output
precedes accounts — is now settled: **accounts and sharing come first**, as the
three-stage split says.

## Concept Visuals

`wireframes.md` covers all three stages across eight screens, with two
recommendations, both explicitly overturnable:

- **Layout: map primary, cross-section in a drawer.** The deciding reason is one
  layout for every form factor — a drawer over a full-bleed map is the native
  phone pattern and scales to desktop unchanged, where the alternatives need a
  second design and a second implementation.
- **Corridor: edit one street, then extend along connected streets**, with a
  per-street stepper as the growth path. Value lands at the first street, before
  the corridor concept appears at all.

**One interaction principle underpins the set.** Fully responsive editing
including phones and a keyboard-operable path for every editing action converge
on the same solution rather than compounding: select-then-act is primary — pick a
lane, then choose what it becomes and how wide — and drag is an accelerator
layered on top, never the sole route to any outcome. That works with touch, with
a keyboard, and with a screen reader from one implementation.

**Accessibility:** WCAG 2.1 AA throughout, including the editing canvas.

## Delivery Approach

One person building with AI assistance. Team Formation was skipped: its own
condition names solo projects, and a skill matrix, mob composition and RACI have
no content for a team of one. Bolt ownership and sequencing are Delivery
Planning's work.

**Constraints that bind delivery:** total capacity is one person, with no
parallel workstreams and no specialist cover; hosting and tooling stay at roughly
$5 a month on Railway's Hobby tier; the product is open source and free, so there
is no revenue mechanism to fund infrastructure growth; and Streetmix code and
assets must not be copied in, since the foundation decision deliberately did not
inherit AGPL.

## Go / No-Go Recommendation

**Go.**

The reasoning, in order of weight:

1. **The premise survived scrutiny.** The gap is confirmed by the incumbent's own
   scope statement, and the position it leaves open — free, map-grounded,
   network-scale, institutionally credible — is occupied by nothing in the
   comparison set.
2. **The hardest problem is a dependency rather than work.** osm2streets converts
   the initiative from "build lane inference from OSM tags" into "integrate a
   library that already does it". That is what makes one person on $5 a month a
   credible proposition rather than an optimistic one.
3. **The largest risk is retired first and cheaply.** B-0 proves osm2streets
   against a real area before anything depends on it. If it fails, the loss is one
   slice of work rather than a foundation.
4. **The scope has a real minimum.** Stage 1 stands alone: a design exists at the
   end of it, without accounts, sharing or export.

**What would change this recommendation:** B-0 failing. The foundation decision
rests on osm2streets being usable, and that has been established from its
documentation rather than by running it. This brief should be revisited if the
skeleton does not produce a rendered cross-section from a real street.

## Handoff to Inception

**Read this brief alongside `intent-backlog.md`, not the backlog alone.** The
backlog now understates the first release: two proto-Units were added to stage 1
at this stage's approval, and the sequencing question it explicitly left open has
been settled. Where they differ, this brief is current.

Inception should carry forward:

- **Requirements Analysis** — data subject rights as functional requirements, per
  the public-release gate; what satisfies that gate is unspecified and needs
  defining.
- **Domain Design** — the osm2streets lane schema shapes the street model. Do not
  assume OSM carries cross-section detail.
- **Units Generation** — B-0 through B-10 are candidates, not Unit boundaries.
- **Delivery Planning** — the sequencing decision is made; Bolt composition is not.

## Assumptions & Open Questions

- The go recommendation rests on osm2streets being usable, established from its
  documentation rather than by running it. [assumption]
- What satisfies the public-release gate — which rights, to what standard — is
  unspecified. [assumption]
- The success measure has no measured baseline; the "about a week today" figure
  is a stated estimate. [assumption]
- City GIS integration is a stated requirement with no city's data examined.
  [assumption]
- Market evidence is descriptions rather than hands-on evaluation, accepted with
  that caveat. [assumption]
- No effort or duration estimate attaches to any stage. [assumption]
- Visual identity was added to stage 1 without an estimate of what it involves,
  and competes with capability work for one person's time. [assumption]
