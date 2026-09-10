# Competitive Analysis — Streetmix at City Scale

## What This Is Measured Against

The comparison set was chosen to cover four groups: Streetmix itself, the
professional CAD and GIS tooling cities use today, public-participation
platforms, and simulation and analysis tools (Q2).

The upstream `intent-statement.md` frames the problem as designing corridors and
networks on a real map rather than one disconnected cross-section, and records
the trigger as existing tools stopping at the single cross-section. This analysis
tests that framing against what the market actually offers.

## The Landscape

| Product | Category | What it does | Strengths | Limits relative to this initiative |
|---------|----------|--------------|-----------|-------------------------------------|
| Streetmix | Street cross-section design | Drag-and-drop composition of one street slice; drag to change lane widths, transport modes, vegetation, underground infrastructure; street capacity metrics with switchable data sources [1][2] | Real institutional traction — used by NYC DOT, Transport for London, SFMTA, and firms including Nelson\Nygaard and Kimley-Horn [1]. Free and open source. | Its own README states the project focuses on individual street sections, "not broader networks or maps" [3]. This is the incumbent confirming the gap the intent statement names. |
| StreetDesign.ai (Beyond CAD) | Streetmix derivative | Described as an AI-powered evolution of Streetmix; adds generated configurations, phasing for design alternatives or construction phases, and stacked side-by-side comparison of designs [4] | Actively extends the Streetmix interaction model beyond the original scope | Still organised around the street section rather than the network; the described features stack and compare sections rather than connect them |
| StreetPlan.net (Urban Innovators) | Streetmix derivative | Free alternative used by city staff to sketch typical sections and project ideas for corridor plans, and by community groups to sketch improvements and rally support [5] | Explicitly positioned for corridor plans and community advocacy — close to this initiative's audience | Described as sketching sections *for inclusion in* corridor plans rather than designing the corridor itself |
| Remix Streets (Via) | Commercial corridor planning | Explores street design concepts, supports data-driven decisions, and moves early-stage transportation projects forward [6] | The commercial incumbent at corridor level; institutional credibility | Commercial and sold to agencies — structurally out of reach for an audience of advocates and the public [assumption] |
| A/B Street | Open-source network simulation | Plans, simulates and communicates visions for friendlier streets; edits streets and intersections, plans bike networks, creates low-traffic neighbourhoods; works anywhere via OpenStreetMap [7] | The closest open-source network-scale tool, and map-grounded by design | Its own documentation records that development of these tools slowed as of January 2024, with newer work moved to other repositories [7] |
| Konveio | Public participation | Turns static plans into interactive commentable documents; AI analysis of feedback; can embed live ArcGIS maps into plans [8] | Strong in the consultation step and already embedded in city plan workflows | Document- and comment-centred: residents comment on a plan rather than design an alternative [assumption] |
| Social Pinpoint | Public participation | 40+ engagement tools including surveys, forums, mapping and budgeting [9] | Broad engagement toolkit | Same limit: gathering input rather than producing a design |
| SUMO | Open-source traffic simulation | Microscopic multi-modal traffic simulation at city-network scale [10] | Free, mature, network-scale | A simulation engine, not a design tool; specialist operation |
| Aimsun | Commercial traffic simulation | Hybrid traffic modelling across large networks [10] | Depth of analysis | Commercial and specialist — the "too slow and too specialised" category the intent statement names |

## Where the Gap Actually Is

The intent statement's premise holds up against the evidence, but the shape of
the gap is narrower than "nothing exists above the cross-section":

- **Section-level design is well served and free.** Streetmix and two derivatives
  occupy this space [1][4][5].
- **Network-level design exists but is either commercial (Remix Streets [6]) or
  has slowed (A/B Street [7]).** The open-source network-scale slot is not empty,
  but it is not actively contested either.
- **Participation is well served but separate from design.** Konveio and Social
  Pinpoint collect opinions about plans; they do not let a participant produce
  one [8][9]. [assumption]
- **The unoccupied position is the join**: free, map-grounded, network-scale
  design whose output an institution will accept. No product in the set was found
  to occupy all four at once. [assumption]

## Differentiation

Table-stakes and differentiator were assigned as follows: the Streetmix-style
editing experience and map/network handling are both table-stakes — a serious
tool is assumed to have them — and the differentiator is producing outputs a city
will actually accept in a formal process (Q3).

That differentiator is well matched to the chosen audience. The addressable
audience is not primarily cities but advocates and the public (Q6), and the
`intent-statement.md` records that group's pain as having no credible way to
propose a street change a city will take seriously. City-acceptable output is
therefore the thing that converts an advocate's proposal into something a city
must respond to — the differentiator and the audience's pain are the same point
seen from two sides.

Positioned against the set, the claim is: **the tool an advocate can use for free
to produce something a city cannot dismiss.** Remix Streets produces
institutionally credible work but is not reachable by that audience [6]; Konveio
and Social Pinpoint are reachable but produce comments rather than designs
[8][9]; Streetmix is reachable and produces designs but stops at one slice [3].
[assumption]

## Pricing and Sustainability Context

The intended model is open source and free, sustained by grants, sponsorship or
goodwill, in the manner of Streetmix itself (Q7).

Against the set this is coherent: the two nearest neighbours by audience —
Streetmix and StreetPlan.net — are both free [1][5], and the paid products in the
set (Remix Streets, Aimsun) sell to institutions rather than to advocates
[6][10]. Competing on price is not available as a differentiator here, because
the closest competitors are already free; the differentiator has to be the
output's credibility, which is what Q3 selected.

## Assumptions & Open Questions

- The limits recorded for StreetDesign.ai, StreetPlan.net, Remix Streets,
  Konveio, and Social Pinpoint are read from vendor and project descriptions
  rather than from hands-on evaluation. A description that does not mention
  network-scale design is not proof that the product lacks it. [assumption]
- No pricing figures were established for Remix Streets or Aimsun; both are
  recorded as commercial without a price point. [assumption]
- A/B Street's slowdown is recorded from its own January 2024 documentation note
  [7]; whether development has since resumed was not established. [assumption]
- The claim that no product occupies all four positions at once is an inference
  from the descriptions gathered, not an exhaustive market sweep. Products
  outside this comparison set were not searched for. [assumption]
- Market size was not quantified. The audience was characterised qualitatively
  (advocates and the public, Q6) with no TAM/SAM/SOM figures, because no
  defensible source for them was found. [assumption]

## Sources

1. [Streetmix — Urbanism Next](https://www.urbanismnext.org/resources/streetmix)
   and [StreetMix | CIVITAS](https://civitas.eu/tool-inventory/streetmix)
2. [What's new in Streetmix? — Streetmix Documentation](https://docs.streetmix.net/user-guide/changelog)
3. [streetmix/streetmix on GitHub](https://github.com/streetmix/streetmix) — README scope statement
4. [A Better Streetmix Alternative — Introducing StreetDesign.ai (Beyond CAD)](https://beyondcad.com/streetmix-alternative/)
5. [StreetPlan.net — Free StreetMix Alternative, by Urban Innovators](https://www.urbaninnovators.com/streetplan)
6. [Remix Streets Software (Via)](https://ridewithvia.com/solutions/remix/streets)
7. [A/B Street — project documentation](https://a-b-street.github.io/docs/software/abstreet.html) and [a-b-street/abstreet on GitHub](https://github.com/a-b-street/abstreet)
8. [Konveio — Engagement Platform for Plans & Policies](https://www.konveio.com/)
9. [Social Pinpoint — Digital Tools for Public Participation](https://info.socialpinpoint.com/usa-digital-tools)
10. [SUMO — Simulation of Urban MObility](https://sumo.dlr.de/pdf/dkrajzew_MESM2002_SUMO.pdf); Aimsun characterised as a commercial hybrid traffic modelling simulator in the same comparison literature
