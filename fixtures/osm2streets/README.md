# osm2streets golden fixtures

Five small, real OpenStreetMap extracts, fetched live from the public
Overpass API (`https://overpass-api.de/api/interpreter`, no auth) on
2026-09-16 and committed as `.osm.xml`. `team.md` (Testing Posture) names
this exact set — a well-tagged street, a thinly-tagged street, a one-way
street, a street with a separately-mapped cycleway, and one intersection —
as the committed input to the osm2streets adapter's golden-fixture suite:
capture real osm2streets *input* once, so the suite that will run against
it (`u4-street-import`, not yet built) never depends on a live OSM fetch at
test time. All five are real, named, geographically identifiable locations,
per `team.md`'s stated preference for this suite over synthetic fragments;
none of the five required a synthetic substitute.

Each file is the OSM way(s) named below plus every node they reference
(`way(id); (._;>;); out meta;`), which is the minimal complete unit
osm2streets needs — a way alone, without its node geometry, is not
buildable.

## `well-tagged-street.osm.xml`

**Northeast Pacific Street**, Seattle, WA (way id `4729721`, the segment
adjoining UW's South Campus). A `highway=secondary` road carrying `lanes=4`
split explicitly as `lanes:forward=3` / `lanes:backward=1`, an exact
`width=16` (metres), `turn:lanes:forward`, a separately-tagged
`cycleway:left=track`, `parking:lane:*`, and `sidewalk=separate`. Exercises
the adapter's happy path: every dimension osm2streets needs is present and
explicit, so nothing here should be inferred — the fixture this project's
provenance model should mark entirely `Mapped`, never `Inferred`.

## `thinly-tagged-street.osm.xml`

**Northeast 48th Street**, Seattle, WA (way id `6344532`), a residential
side street. Carries only `highway=residential`, `name`, `lanes=2`,
`maxspeed`, `sidewalk:left`/`sidewalk:right`, and `surface` — no `width`,
no `lanes:forward`/`lanes:backward` split, no `turn:lanes`. Forces
osm2streets to infer per-lane width and the forward/backward split from the
bare lane count and highway classification alone. Exercises the other half
of `team.md`'s named provenance test floor: at least one fixture that must
yield an `Inferred` value, distinguishable in the UI from `Mapped`
(`raid-log.md` R-2; Q3's pass/fail criterion).

## `one-way-street.osm.xml`

**Madison Street**, Seattle, WA (way id `6428348`), the two-lane
`highway=primary` one-way segment through downtown. Carries `oneway=yes`
with `lanes=2` (no `lanes:forward`/`lanes:backward` split — the whole
carriageway runs one direction, so osm2streets must attribute all lanes to
a single travel direction rather than split them). Exercises the adapter's
one-way handling independent of the cycleway and intersection concerns the
other fixtures cover. The extract also includes a nearby
`public_transport=stop_position` node (`Madison St & 2nd Ave`) picked up
incidentally by the node recursion; it is inert for lane/road parsing and
left in rather than hand-pruned, since a real OSM extract routinely carries
nodes like it.

## `street-with-cycleway.osm.xml`

**North Northlake Way** (way id `8111822`) running alongside the
**Burke-Gilman Trail** (way id `1081893497`), both in the Wallingford/Gas
Works Park area of Seattle, WA. Northlake Way is an ordinary
`highway=tertiary` road with no cycleway tag of its own
(`bicycle`/`cycleway:*` are absent from its tags); the Burke-Gilman Trail is
a distinct `highway=cycleway` way running parallel to it, mapped as its own
independent geometry rather than as a `cycleway:left`/`cycleway:right` tag
on the road (contrast with `well-tagged-street.osm.xml`'s
`cycleway:left=track`, which is the same-way form). Exercises the case
`team.md`'s named fixture set calls "a street with a separately mapped
cycleway" — the adapter must not conflate the two ways or assume every
street's cycleway is tagged on the road itself.

## `one-intersection.osm.xml`

The junction of **15th Avenue Northeast** and **Northeast 50th Street**,
University District, Seattle, WA — a real four-way signalized intersection.
Fetched as a bounding-box extract (`47.6645,-122.3135` to
`47.6656,-122.3115`) rather than a single way id, because an intersection is
inherently a multi-way, shared-node structure: the box captures the five
way segments osm2streets' `split_ways` divides the two streets into
around the junction node, plus the transit-stop nodes (`gtfs:stop_id:*`,
`Northeast 50th Street & University Way Northeast`, etc.) that happen to
fall inside it. The transit nodes are incidental context from a real
extract, not hand-curated away, matching this project's own no-live-fetch
rationale: a fixture drawn from a real bounding box looks like what the
adapter will actually see in production, not a hand-trimmed idealisation of
it.

## What this Unit does not do with these fixtures

Per `unit-of-work.md`'s Unit boundary and `code-generation-plan.md` Step
2.3: this Unit commits the fixture data only. The test code that runs
osm2streets against these five files and asserts specific output (lane
counts, widths, provenance values) belongs to `u4-street-import`, which
does not exist yet — its `unit-test-instructions.md` will own the actual
`cargo test -p <adapter-crate>` commands exercising these fixtures. This
Unit's own test floor (`unit-test-instructions.md`) is limited to "every
fixture file is well-formed XML."

## Provenance

All five extracts are © OpenStreetMap contributors, available under the
Open Database License (ODbL) — the same licence every OSM extract carries;
see the `<note>` element each fixture file's own Overpass response header
states verbatim. Fetched via the public Overpass API on 2026-09-16 with a
one-way-per-request, back-off-on-busy fetch pattern, never a bulk scrape,
per `team.md`'s "impolite to a free public API" caution about live OSM
fetches — the same caution that is why these are committed fixtures rather
than a live-fetch test suite in the first place.
