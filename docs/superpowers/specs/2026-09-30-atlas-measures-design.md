# Transit Priority Atlas measures in both editors

Date: 2026-09-30

## Goal

Support the measures of the Transit Priority Atlas toolbox
(https://tpa.transitcosts.com/atlas/toolbox) wherever they fit the two
editors, with the Atlas's codes and names, each drawn, editable and checked.
The Atlas is credited on the Credits page; CityLoom is not affiliated with it
and its definitions below are CityLoom's own simplifications.

## Decisions taken with the user

- Scope: everything that fits the editors. Not modelled, with the reason shown
  on the Measures tab: TSP (under development on the Atlas, and there is no
  signal timing), Z1 limited traffic areas (many streets), W dynamic and
  fixed-fee pricing (in development, and a price needs a city and demand).
- Depth: drawn, editable and checked, not just drawn.

## Where each measure lives

| Code | Measure | Editor |
|---|---|---|
| A1 A2 A3 | Transit streets, transit ways, transit and direct access streets | Street |
| B1 B2 B3 B4 | Centre-running lanes, on freeway medians, static and dynamic alternate-direction | Street |
| C1 D1 E1 E2 E3 | Edge-running bidirectional, offset, curb-adjacent, reversible parking and transit, freeway shoulders | Street |
| F1 F2 | Contraflow, offset contraflow | Street |
| G1 G2 G3 | Offset and curbside queue jumps, virtual queue jump | Intersection |
| H1 H2 | Signal- and yield-controlled bus gates | Intersection |
| L1 L2 L3 L4 | Indirect left via itinerary, within the intersection, right-in/right-out, dead-ending | Intersection |
| M1 M2 | Bus bulbs, signal-protected on-street platforms | Intersection |
| N1 | Transit modal filter | Intersection |

## Street editor

Needs: a bus lane may run one way (or two ways); an other-times type has its
own direction; a Shoulder kind; a freeway sample; a "Start from" list of
sample streets on the street page.

Each lane measure has a **definition** (what the section must show), **needs**
(a problem listed in redline) and an **arrangement** (laid out from the
street's width, keeping its edge sidewalks and planting). Definitions, on the
roadway pieces (sidewalks, planting and medians are not roadway):

- A1: bus lanes both ways, no driving lanes, parking or loading. A2: A1 with no
  bike lane. A3: bus lanes both ways, no driving lanes or parking, with loading.
- B1: bus lanes both ways, together, with roadway on both sides of them. B2: the
  same on a freeway. B3: one centre bus lane that runs one way and has an
  other-times window the opposite way. B4: one centre bus lane that is two-way.
- C1: exactly two bus lanes, one each way, together at an edge, with other
  traffic. D1: a bus lane with parking or loading between it and the curb.
  E1: a bus lane at the curb running with its side's traffic. E2: an edge
  parking lane with bus windows, or the reverse. E3: on a freeway, a bus lane at
  the edge or a shoulder with bus windows.
- F1: general lanes all run one way and a bus lane at the edge runs the other.
  F2: the same with parking or loading between it and the curb.
- Needs: a transit lane at least 3.3 m wide (synthetic); centre-running lanes
  need a median or platform beside them for stops.
- Arranging squeezes other traffic before transit lanes, leaves out removable
  pieces from alternate ends when the street is too narrow, grows pieces to
  fill the width, and refuses when it will not fit or is for a freeway only
  (B2, E3) or never for one.

## Intersection editor

Per street, in the left sidebar under Transit priority: a bus lane along the
way in; an approach measure (G1, G2, G3, H1, H2) with a length of 5 to 15 m; a
bus stop (M1, M2); turn management (L1 to L4); a modal filter (N1).

- Needs, listed in redline and failing "Transit measures work": G1 parking
  beside the curb; G3 a signal; H1 and H2 a bus lane and a traffic lane to hold
  back; M1 parking to extend into; M2 a signal and a crossing; N1 a bus lane.
- Turns a measure rules out are locked in the schedule with the reason and are
  not offered to lanes: dead end (L4), modal filter (N1), only right turns in
  and out (L3), left turns sent round by another street (L1). L2 keeps the left
  turn but draws it through the middle.

## Page

A Measures tab on both pages lists the whole Atlas by group, with where each is
modelled; on the street page a lane measure shows "This street" when the section
forms it (with its problems) or an Arrange button; on the junction page it shows
where this junction uses each.

## Out of scope

Real Atlas guidance (warrants, dimensions, signal phasing); transit signal
priority; area-wide measures and pricing; speed, delay or ridership outcomes.
